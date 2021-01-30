mod types;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, Promise, Balance};
use near_sdk::serde::{Serialize};
use near_lib::types::{WrappedDuration, WrappedTimestamp};
pub use crate::types::*;
use near_sdk::json_types::U64;

/// There is no deposit balance attached.
const NO_DEPOSIT: Balance = 0;

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

const CODE: &[u8] = include_bytes!("../../lockup/res/lockup_contract.wasm");

/// This gas spent on the call & account creation, the rest goes to the `new` call.
const CREATE_CALL_GAS: u64 = 40_000_000_000_000;
const MIN_ATTACHED_BALANCE: Balance = 30_000_000_000_000_000_000_000_000;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct LockupFactory {
    staking_pool_whitelist_account_id: AccountId,
    foundation_account_id: AccountId,
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct LockupArgs {
    lockup_duration: WrappedDuration,
    lockup_timestamp: Option<WrappedTimestamp>,
    transfers_information: TransfersInformation,
    vesting_schedule: Option<VestingScheduleOrHash>,
    release_duration: Option<WrappedDuration>,
    staking_pool_whitelist_account_id: AccountId,
    foundation_account_id: AccountId,
    owner_account_id: AccountId
}


impl Default for LockupFactory {
    fn default() -> Self {
        env::panic(b"LockupFactory should be initialized before usage")
    }
}

#[near_bindgen]
impl LockupFactory {
    #[init]
    pub fn new(staking_pool_whitelist_account_id: AccountId, foundation_account_id: AccountId) -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");

        assert!(
            env::is_valid_account_id(staking_pool_whitelist_account_id.as_bytes()),
            "The staking pool whitelist account ID is invalid"
        );

        assert!(
            env::is_valid_account_id(foundation_account_id.as_bytes()),
            "The foundation account is invalid"
        );

        Self {
            staking_pool_whitelist_account_id,
            foundation_account_id,
        }
    }

    /// Returns the foundation account id.
    pub fn get_foundation_account_id(&self) -> String {
        self.foundation_account_id.to_string()
    }


    #[payable]
    pub fn create(&mut self,
                  lockup_account_id: AccountId,
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

        assert!(
            lockup_account_id.find('.').is_none(),
            "The lockup account can't contain `.`"
        );

        assert!(
            env::is_valid_account_id(owner_account_id.as_bytes()),
            "The owner account ID is invalid"
        );


        Promise::new(lockup_account_id)
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
                        transfers_timestamp: U64(0),
                    },
                    vesting_schedule,
                    release_duration,
                    staking_pool_whitelist_account_id: self.staking_pool_whitelist_account_id.clone(),
                    foundation_account_id: self.foundation_account_id.clone(),
                }).unwrap(),
                NO_DEPOSIT,
                env::prepaid_gas() - CREATE_CALL_GAS,
            )
    }
}


#[cfg(test)]
mod tests {
    use near_lib::context::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain};

    #[test]
    fn test_basics() {
        testing_env!(VMContextBuilder::new().current_account_id(accounts(0)).finish());
        //let mut factory = LockupFactory::new("whitelist".to_string(), "foundation".to_string());
    }
}
