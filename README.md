# Lockup Factory

# Deployment & Usage

## TestNet

near dev-deploy --wasmFile=res/lockup_factory.wasm

# bash
CONTRACT_ID="dev-1611146006203-1841764"
# fish
set CONTRACT_ID "dev-1611146006203-1841764"

# Initialize the factory.
near call $CONTRACT_ID new '{}' --accountId $CONTRACT_ID 

# bash
ARGS=`echo '{"owner_account_id": "dev-1611146006203-1841764", "lockup_duration": "31536000000000000", "transfers_information": {"TransfersDisabled": {"transfer_poll_account_id": "transfer-vote.dev-1611146006203-1841764"}}, "vesting_schedule": { "VestingSchedule": {"start_timestamp": "1535760000000000000", "cliff_timestamp": "1567296000000000000", "end_timestamp": "1661990400000000000"}}, "release_duration": "126230400000000000", "staking_pool_whitelist_account_id": "whitelist.dev-1611146006203-1841764", "foundation_account_id": "dev-1611146006203-1841764"}' | base64`

# Create a new DAO with the given parameters.
near call $CONTRACT_ID create "{\"name\": \"lockup1\", \"public_key\": null, \"args\": \"$ARGS\"}"  --accountId $CONTRACT_ID --amount 50 --gas 100000000000000

