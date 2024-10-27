// Find all our documentation at https://docs.near.org
use near_sdk::json_types::U128;
use near_sdk::{
 AccountId, near, PanicOnDefault, env, Promise, NearToken, log, Gas, PromiseResult, PromiseError
};
use std::collections::HashMap;
use near_sdk::ext_contract;

pub const GAS_FOR_FT_TRANSFER: Gas = Gas::from_gas(20_000_000_000_000);
pub const GAS_FOR_FT_TRANSFER_CALL: Gas = Gas::from_gas(100_000_000_000_000);
pub const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas::from_gas(20_000_000_000_000);
pub const YOCTO_DEPOSIT: NearToken = NearToken::from_yoctonear(1);

#[ext_contract(ext_fungible_token)]
pub trait ExtFungibleToken  {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

#[ext_contract(ext_wrap)]
pub trait ExtWrap  {
    fn near_deposit(&mut self);
    fn ft_transfer_call(&mut self, receiver_id: AccountId, amount: U128, msg: Option<String>);
   // Arguments: {
    //     "receiver_id": "v2.ref-finance.near",
    //     "amount": "1000000000000000000000000",
    //     "msg": "{\"force\":0,\"actions\":[{\"pool_id\":974,\"token_in\":\"wrap.near\",\"token_out\":\"2260fac5e5542a773aa44fbcfedf7c193bc2c599.factory.bridge.near\",\"amount_in\":\"1000000000000000000000000\",\"amount_out\":\"0\",\"min_amount_out\":\"7486\"}]}"
    //   }
}

// Define the contract structure
#[near(contract_state, serializers = [json, borsh])]
#[derive(PanicOnDefault)]
pub struct Contract {
    pub users: HashMap<AccountId, User>,
    pub user_addresses: Vec<AccountId>,
    pub batch_swap_threshold: u8,
    pub token_address: AccountId,
    pub owner: AccountId,
    pub fees: u8,
    pub wrap_account: AccountId,
    pub pool_id: u16,
    pub pool_address: AccountId,
}

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct User {
    pub wallet: AccountId,
    pub amount_per_swap: U128,
    pub swap_interval: u64,
    pub last_swap_timestamp: u64,
    pub total_swapped: U128,
    pub amount: U128,
    pub pause: bool,
}

// Define the default, which automatically initializes the contract
#[near]
impl Contract {
    #[init]
    #[private]
    pub fn init(token_address: AccountId, owner: AccountId, fees: u8, wrap_account: AccountId, pool_id: u16, pool_address: AccountId) -> Self {
        Self {
            
            users: HashMap::new(),
            user_addresses: Vec::new(),
            batch_swap_threshold: 10, // Adjust threshold as needed
            token_address,
            owner,
            fees,
            wrap_account,
            pool_id,
            pool_address,
        }
    }

    #[payable]
    pub fn register_user(&mut self, amount_per_swap: U128, swap_interval: u64) {
        // get attached deposit
        let amount = env::attached_deposit();
        assert!(amount.as_yoctonear() > 0, "Deposit must be greater than 0");
        assert!(amount.as_yoctonear() > amount_per_swap.0, "Deposit must be greater than 0");

        // user must not exist
        assert!(!self.users.contains_key(&env::signer_account_id()), "User already exists");
        
        let user = User {
            wallet: env::signer_account_id(),
            amount_per_swap,
            swap_interval,
            last_swap_timestamp: 0,
            total_swapped: U128(0),
            amount: amount.as_yoctonear().into(),
            pause: false,
        };
        self.users.insert(env::signer_account_id(), user);
        self.user_addresses.push(env::signer_account_id());
    }

    #[payable]
    pub fn topup(&mut self) {
        let amount = env::attached_deposit();
        assert!(amount.as_yoctonear() > 0, "Deposit must be greater than 0");
        
        // user must exist
        assert!(self.users.contains_key(&env::signer_account_id()), "User does not exist");

        let mut user = self.users.get(&env::signer_account_id()).unwrap().clone();

        user.amount = U128(user.amount.0 + amount.as_yoctonear()); // add amount;
        self.users.insert(env::signer_account_id(), user.clone());
    }

    #[payable]
    pub fn withdraw_near(&mut self, amount: U128) {
        // user must exist
        assert!(self.users.contains_key(&env::signer_account_id()), "User does not exist");
        let mut user = self.users.get(&env::signer_account_id()).unwrap().clone();

        // check if user has enough balance
        assert!(user.amount >= amount, "User does not have enough balance");

        let new_amount = user.amount.0.checked_sub(amount.0).expect("Insufficient funds");
        user.amount = U128(new_amount); // subtract amount;
        self.users.insert(env::signer_account_id(), user.clone());

        let near_amount: NearToken = NearToken::from_yoctonear(amount.0);
        Promise::new(env::signer_account_id()).transfer(near_amount);
    }

    #[payable]
    pub fn withdraw_ft(&mut self, amount: U128) {
        // user must exist
        assert!(self.users.contains_key(&env::signer_account_id()), "User does not exist");
        let mut user = self.users.get(&env::signer_account_id()).unwrap().clone();
        // check if user has enough balance
        assert!(user.total_swapped >= amount, "User does not have enough balance");

        let new_total_swapped = user.total_swapped.0.checked_sub(amount.0).expect("Amount to withdraw is greater than total swapped");
        user.total_swapped = U128(new_total_swapped); // subtract amount;
        self.users.insert(env::signer_account_id(), user.clone());


        ext_fungible_token::ext(self.token_address.clone())
            .with_static_gas(GAS_FOR_FT_TRANSFER)
            .with_attached_deposit(YOCTO_DEPOSIT)
            .ft_transfer(
                env::signer_account_id(),
                amount.into(),
                None
            )
        .then(Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
                .callback_post_withdraw_reward(
        ));

        // ext_self::self.token_address
        //     .with_static_gas(gas)
        //     .call(ft_transfer_call {
        //         from: env::current_account_id(),
        //         to: env::signer_account_id(),
        //         amount: amount_to_withdraw,
        //         msg: "".to_string(),
        //     });
    }

    #[private]
    pub fn callback_post_withdraw_reward(){

    }

    #[payable]
    pub fn pause(&mut self) {
        // user must exist
        assert!(self.users.contains_key(&env::signer_account_id()), "User does not exist");

        let mut user = self.users.get(&env::signer_account_id()).unwrap().clone();

        assert!(!user.pause, "User is already paused");
        user.pause = true;

        self.users.insert(env::signer_account_id(), user.clone());
    }

    #[payable]
    pub fn resume(&mut self) {
        // user must exist
        assert!(self.users.contains_key(&env::signer_account_id()), "User does not exist");

        let mut user = self.users.get(&env::signer_account_id()).unwrap().clone();

        assert!(user.pause, "User is not paused");
        user.pause = false;

        self.users.insert(env::signer_account_id(), user.clone());
    }

    #[payable]
    pub fn remove_user(&mut self) {
        // user must exist
        assert!(self.users.contains_key(&env::signer_account_id()), "User does not exist");

        // withdraw all funds
        self.withdraw_near(self.users.get(&env::signer_account_id()).unwrap().amount);
        self.withdraw_ft(self.users.get(&env::signer_account_id()).unwrap().total_swapped);

        // remove user from users map
        self.users.remove(&env::signer_account_id());
    }

    pub fn can_swap(&self) -> bool {
        for (_, user) in self.users.iter() {
            // check if user has to swap and if it is not paused
            // create a mutable copy of the user
            let mut _tmp_user = user.clone();

            if env::block_timestamp() >= user.last_swap_timestamp + user.swap_interval && user.pause == false {
                // check if amount per swap is bigger than amount otherwise pause the user
                if user.amount < user.amount_per_swap {
                    // tmp_user.pause = true;
                    // self.users.insert(user.wallet.clone(), tmp_user.clone());
                    continue;
                }
            
                return true;
            }
        }

        return false;
        
    }

    #[payable]
    pub fn swap(&mut self) {
        assert_eq!(env::signer_account_id(), self.owner);
        // iterate over all users
        // check if timestamp is greater than last_swap_timestamp + swap_interval
        // if yes add them to the batch

        let mut batch_amount:U128 = U128(0);
        let mut batch_users:Vec<AccountId> = Vec::new();
        
        for (_, user) in self.users.iter() {
            // check if user has to swap and if it is not paused
            // create a mutable copy of the user
            let mut _tmp_user = user.clone();

            if env::block_timestamp() >= user.last_swap_timestamp + user.swap_interval && user.pause == false {
                // check if amount per swap is bigger than amount otherwise pause the user
                if user.amount < user.amount_per_swap {
                    // tmp_user.pause = true;
                    // self.users.insert(user.wallet.clone(), tmp_user.clone());
                    continue;
                }
                
                // check if the batch is full
                if batch_users.len() >= self.batch_swap_threshold.into() {
                    break;
                }

                // add to batch
                batch_amount = U128(batch_amount.0 + user.amount_per_swap.0);
                batch_users.push(user.wallet.clone());

            }
        }

        // check if batch is empty
        if batch_users.len() == 0 {
            return;
        }

        let batch_amount_total = batch_amount.0.checked_sub(
            batch_amount.0.checked_mul(self.fees as u128).unwrap_or(0) / 10000
        ).unwrap_or(0);

        ext_wrap::ext(self.wrap_account.clone())
            .with_static_gas(GAS_FOR_FT_TRANSFER)
            .with_attached_deposit(NearToken::from_yoctonear(batch_amount_total))
            .near_deposit(
            )
        .then(Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
                .callback_post_wrap(
                    batch_users,
                    batch_amount,
                    batch_amount_total
        ));

    }

    #[private]
    pub fn callback_post_wrap(&mut self, batch_users:Vec<AccountId>, batch_amount:U128, batch_amount_total:u128) {
        assert_eq!(env::promise_results_count(), 1);
        assert_eq!(env::promise_result(0), PromiseResult::Successful(vec![]), "Failed to wrap tokens");

        // format the actions
        let action: String = format!(
            "{{\"force\":0,\"actions\":[{{\"pool_id\":{},\"token_in\":\"{}\",\"token_out\":\"{}\",\"amount_in\":\"{}\",\"amount_out\":\"0\",\"min_amount_out\":\"0\"}}]}}",
            self.pool_id.to_string(),
            self.wrap_account.clone(),
            self.token_address.clone(),
            batch_amount_total
        );

        ext_wrap::ext(self.wrap_account.clone())
            .with_static_gas(GAS_FOR_FT_TRANSFER_CALL)
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .ft_transfer_call(
                self.pool_address.clone(),
                batch_amount_total.into(),
                Some(action.clone())
            )
        .then(Self::ext(env::current_account_id())
                .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
                .callback_post_swap(
                    batch_users,
                    batch_amount,
                    batch_amount_total
        ));
    }

    #[private]
    pub fn callback_post_swap(&mut self, batch_users:Vec<AccountId>, batch_amount:U128, batch_amount_total:u128, #[callback_result] call_result: Result<String, PromiseError>,) -> HashMap<AccountId, u128> {
        assert_eq!(env::promise_results_count(), 1);
        if call_result.is_err() {
            log!("There was an error while swapping");
            // we should rollback the transaction
            // self.rollback();
            
            return HashMap::new();
        }
        
        // initialize the return value
        let mut return_value: HashMap<AccountId, u128> = HashMap::new();

        // update last_swap_timestamp, total_swapped and amount for users in the batch
        for user in batch_users {
            let mut user_tmp: User = self.users.get(&user.clone()).unwrap().clone();
            user_tmp.last_swap_timestamp = env::block_timestamp();
            // get the percentage of total_swapped and amount
            user_tmp.total_swapped = U128(user_tmp.total_swapped.0 + (user_tmp.amount_per_swap.0 / batch_amount.0) * batch_amount_total);
            let new_amount = user_tmp.amount.0.checked_sub(user_tmp.amount_per_swap.0).expect("Insufficient funds");
            user_tmp.amount = U128(new_amount);
            self.users.insert(user_tmp.wallet.clone(), user_tmp.clone());
            // log the swap
            log!("<swapLog> {{\"user\": \"{}\", \"source\": \"NEAR\", \"source_amount\": {}, \"target\": \"{}\", \"target_amount\": \"{}\"}}", user_tmp.wallet.clone(), user_tmp.amount_per_swap.0, self.token_address, user_tmp.total_swapped.0);

            // add to return value
            return_value.insert(user.clone(), user_tmp.total_swapped.0);
        }

        return_value
    }

    #[payable]
    pub fn set_batch_swap_threshold(&mut self, new_threshold: u8) {
        assert_eq!(env::signer_account_id(), self.owner);
        self.batch_swap_threshold = new_threshold;
    }

    pub fn get_batch_swap_threshold(&self) -> u8 {
        self.batch_swap_threshold.into()
    }

    #[payable]
    pub fn set_fees(&mut self, new_fees: u8) {
        assert_eq!(near_sdk::env::signer_account_id(), self.owner);
        self.fees = new_fees;
    }

    pub fn get_fees(&self) -> u8 {
        self.fees
    }

    pub fn get_user(&self, user: AccountId) -> User {
        self.users.get(&user).unwrap().clone()
    }
}

/*
 * The rest of this file holds the inline tests for the code above
 * Learn more about Rust tests: https://doc.rust-lang.org/book/ch11-01-writing-tests.html
 */
#[cfg(test)]
mod tests {
    use super::*;

    
}
    