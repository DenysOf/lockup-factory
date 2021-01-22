use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::Base64VecU8;
use near_sdk::{env, near_bindgen, AccountId, Promise};
use near_sdk::collections::UnorderedSet;

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

const CODE: &[u8] = include_bytes!("../../lockup/res/lockup_contract.wasm");

/// This gas spent on the call & account creation, the rest goes to the `new` call.
const CREATE_CALL_GAS: u64 = 40_000_000_000_000;

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize)]
pub struct LockupFactory {
    lockups: UnorderedSet<AccountId>,
}

impl Default for LockupFactory {
    fn default() -> Self {
        env::panic(b"LockupFactory should be initialized before usage")
    }
}

#[near_bindgen]
impl LockupFactory {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        Self {
            lockups: UnorderedSet::new(b"d".to_vec()),
        }
    }


    #[payable]
    pub fn create(&mut self, name: AccountId, args: Base64VecU8) -> Promise {
        let account_id = format!("{}.{}", name, env::current_account_id());
        Promise::new(account_id)
            .create_account()
            .deploy_contract(CODE.to_vec())
            .transfer(env::attached_deposit())
            .function_call(
                b"new".to_vec(),
                args.into(),
                0,
                env::prepaid_gas() - CREATE_CALL_GAS,
            )
    }
}

#[cfg(test)]
mod tests {
    use near_lib::context::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain};

    use super::*;

    #[test]
    fn test_basics() {
        testing_env!(VMContextBuilder::new().current_account_id(accounts(0)).finish());
        let mut factory = LockupFactory::new();
        testing_env!(VMContextBuilder::new().current_account_id(accounts(0)).attached_deposit(10).finish());
        factory.create("test".to_string(), "{}".as_bytes().to_vec().into());
    }
}
