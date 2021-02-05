mod types;
mod utils;

use crate::utils::*;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, ext_contract, near_bindgen, AccountId, Promise, PromiseOrValue, Balance};
use near_sdk::json_types::{U128};
use near_sdk::serde::{Serialize};
use near_lib::types::{WrappedDuration, WrappedTimestamp};
pub use crate::types::*;
use sha2::{Sha256, Digest};
use crate::gas::{LOCKUP_NEW, CALLBACK};

/// There is no deposit balance attached.
const NO_DEPOSIT: Balance = 0;
const TRANSFER_STARTED: u64 = 1603274400000000000;

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

const CODE: &[u8] = include_bytes!("../../lockup/res/lockup_contract.wasm");


pub mod gas {
    use near_sdk::Gas;

    /// The base amount of gas for a regular execution.
    const BASE: Gas = 35_000_000_000_000;

    /// The amount of Gas the contract will attach to the promise to create the lockup.
    pub const LOCKUP_NEW: Gas = BASE * 2;

    /// The amount of Gas the contract will attach to the callback to itself.
    /// The base for the execution and the base for cash rollback.
    pub const CALLBACK: Gas = BASE * 2;
}

const MIN_ATTACHED_BALANCE: Balance = 35_000_000_000_000_000_000_000_000;

/// External interface for the callbacks to self.
#[ext_contract(ext_self)]
pub trait ExtSelf {
    fn on_lockup_create(
        &mut self,
        lockup_account_id: AccountId,
        attached_deposit: U128,
        predecessor_account_id: AccountId,
    ) -> Promise;
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct LockupFactory {
    master_account_id: AccountId,
    lockup_master_account_id: AccountId,
    whitelist_account_id: AccountId,
    foundation_account_id: AccountId,
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct LockupArgs {
    owner_account_id: AccountId,
    lockup_duration: WrappedDuration,
    lockup_timestamp: Option<WrappedTimestamp>,
    transfers_information: TransfersInformation,
    vesting_schedule: Option<VestingScheduleOrHash>,
    release_duration: Option<WrappedDuration>,
    staking_pool_whitelist_account_id: AccountId,
    foundation_account_id: Option<AccountId>,
}


impl Default for LockupFactory {
    fn default() -> Self {
        env::panic(b"LockupFactory should be initialized before usage")
    }
}

#[near_bindgen]
impl LockupFactory {
    #[init]
    pub fn new(master_account_id: AccountId,
               lockup_master_account_id: AccountId,
               whitelist_account_id: AccountId,
               foundation_account_id: AccountId) ->
               Self {
        assert!(!env::state_exists(), "The contract is already initialized");

        assert!(
            env::is_valid_account_id(master_account_id.as_bytes()),
            "The master account ID is invalid"
        );

        assert!(
            env::is_valid_account_id(lockup_master_account_id.as_bytes()),
            "The lockup account ID is invalid"
        );
        assert!(
            env::is_valid_account_id(whitelist_account_id.as_bytes()),
            "The whitelist account ID is invalid"
        );

        assert!(
            env::is_valid_account_id(foundation_account_id.as_bytes()),
            "The foundation account is invalid"
        );
        Self {
            master_account_id,
            lockup_master_account_id,
            whitelist_account_id,
            foundation_account_id,
        }
    }

    /// Returns the foundation account id.
    pub fn get_foundation_account_id(&self) -> String {
        self.foundation_account_id.to_string()
    }

    /// Returns the master account id.
    pub fn get_master_account_id(&self) -> String {
        self.master_account_id.to_string()
    }

    /// Returns the lockup account id.
    pub fn get_lockup_master_account_id(&self) -> String {
        self.lockup_master_account_id.to_string()
    }

    pub fn get_min_attached_balance(&self) -> U128 {
        MIN_ATTACHED_BALANCE.into()
    }


    #[payable]
    pub fn create(&mut self,
                  owner_account_id: AccountId,
                  lockup_duration: WrappedDuration,
                  lockup_timestamp: Option<WrappedTimestamp>,
                  vesting_schedule: Option<VestingScheduleOrHash>,
                  release_duration: Option<WrappedDuration>,
    ) -> Promise {
        assert!(
            env::attached_deposit() >= MIN_ATTACHED_BALANCE,
            "Not enough attached deposit"
        );

        let byte_slice = Sha256::new().chain(owner_account_id.to_string()).finalize();
        let string: String = format!("{:x}", byte_slice);
        let lockup_suffix = ".".to_string() + &self.lockup_master_account_id.to_string();
        let sliced_string = &string[..40];
        let lockup_account_id: AccountId = sliced_string.to_owned() + &lockup_suffix;

        assert!(
            env::is_valid_account_id(owner_account_id.as_bytes()),
            "The owner account ID is invalid"
        );

        let mut foundation_account: Option<AccountId> = None;
        if vesting_schedule.is_some() {
            foundation_account = Option::from(self.foundation_account_id.clone());
        };


        let transfers_enabled: WrappedTimestamp = TRANSFER_STARTED.into();
        Promise::new(lockup_account_id.clone())
            .create_account()
            .deploy_contract(CODE.to_vec())
            .transfer(env::attached_deposit())
            .function_call(
                b"new".to_vec(),
                near_sdk::serde_json::to_vec(&LockupArgs {
                    owner_account_id,
                    lockup_duration,
                    lockup_timestamp,
                    transfers_information: TransfersInformation::TransfersEnabled {
                        transfers_timestamp: transfers_enabled,
                    },
                    vesting_schedule,
                    release_duration,
                    staking_pool_whitelist_account_id: self.whitelist_account_id.clone(),
                    foundation_account_id: foundation_account,
                }).unwrap(),
                NO_DEPOSIT,
                LOCKUP_NEW,
            )
            .then(ext_self::on_lockup_create(
                lockup_account_id,
                env::attached_deposit().into(),
                env::predecessor_account_id(),
                &env::current_account_id(),
                NO_DEPOSIT,
                CALLBACK,
            ))
    }

    /// Callback after a lockup was created.
    /// Returns the promise if the lockup creation succeeded.
    /// Otherwise refunds the attached deposit and returns `false`.
    pub fn on_lockup_create(
        &mut self,
        lockup_account_id: AccountId,
        attached_deposit: U128,
        predecessor_account_id: AccountId,
    ) -> PromiseOrValue<bool> {
        assert_self();

        let lockup_account_created = is_promise_success();

        if lockup_account_created {
            env::log(
                format!(
                    "The lockup contract @{} was successfully created.",
                    lockup_account_id
                )
                .as_bytes(),
            );
            return PromiseOrValue::Value(true).into();
        } else {
            env::log(
                format!(
                    "The lockup @{} creation has failed. Returning attached deposit of {} to @{}",
                    lockup_account_id,
                    attached_deposit.0,
                    predecessor_account_id
                ).as_bytes()
            );
            Promise::new(predecessor_account_id).transfer(attached_deposit.0);
            PromiseOrValue::Value(false)
        }
    }
}


#[cfg(test)]
mod tests {
    use near_lib::context::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain};
    use sha2::{Sha256, Digest};

    #[test]
    fn test_basics() {
        testing_env!(VMContextBuilder::new().current_account_id(accounts(0)).finish());
        let lockup_account_id = "boby.nearnet".to_string();
        let byte_slice = Sha256::new().chain(lockup_account_id).finalize();
        let string: String = format!("{:x}", byte_slice);
        let lockup_suffix = ".lockup.nearnet".to_string();
        let x = &string[..40];
        let r = x.to_owned() + &lockup_suffix;
        println!("Result: {:?}", r);
    }
}
