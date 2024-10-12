// Find all our documentation at https://docs.near.org
use near_sdk::json_types::U128;
use near_sdk::{
 AccountId, near, PanicOnDefault, env, Promise, NearToken, log
};
use std::collections::HashMap;


// Define the contract structure
#[near(contract_state, serializers = [json, borsh])]
#[derive(PanicOnDefault)]
pub struct Contract {
    pub users: HashMap<AccountId, User>,
    pub user_addresses: Vec<AccountId>,
    pub batch_swap_threshold: u8,
    pub pool_address: AccountId,
    pub token_address: AccountId,
    pub owner: AccountId,
    pub fees: u8,
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
    pub fn init(pool_address: AccountId, token_address: AccountId, owner: AccountId, fees: u8) -> Self {
        Self {
            
            users: HashMap::new(),
            user_addresses: Vec::new(),
            batch_swap_threshold: 10, // Adjust threshold as needed
            pool_address,
            token_address,
            owner,
            fees,
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

        user.amount = U128(user.amount.0 - amount.0); // subtract amount;
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

        user.total_swapped = U128(user.total_swapped.0 - amount.0); // subtract amount;
        self.users.insert(env::signer_account_id(), user.clone());

        // ext_self::self.token_address
        //     .with_static_gas(gas)
        //     .call(ft_transfer_call {
        //         from: env::current_account_id(),
        //         to: env::signer_account_id(),
        //         amount: amount_to_withdraw,
        //         msg: "".to_string(),
        //     });
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
            let mut tmp_user = user.clone();

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

        let batch_amount_total:u128 = batch_amount.0 - (batch_amount.0 * self.fees as u128) / 100;
        

        // update last_swap_timestamp, total_swapped and amount for users in the batch

        for user in batch_users {
            let mut user_tmp: User = self.users.get(&user.clone()).unwrap().clone();
            user_tmp.last_swap_timestamp = env::block_timestamp();
            // get the percentage of total_swapped and amount
            user_tmp.total_swapped = U128(user_tmp.total_swapped.0 + (user_tmp.amount_per_swap.0 / batch_amount.0) * batch_amount_total);
            user_tmp.amount = U128(user_tmp.amount.0 - user_tmp.amount_per_swap.0);
            self.users.insert(user_tmp.wallet.clone(), user_tmp.clone());
            // log the swap
            log!("User {} swapped {} NEAR for {} {}", user_tmp.wallet.clone(), user_tmp.amount_per_swap.0, user_tmp.total_swapped.0, self.token_address);
        }

        
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
    pub fn set_pool_address(&mut self, new_pool_address: AccountId) {
        assert_eq!(env::signer_account_id(), self.owner);
        self.pool_address = new_pool_address;
    }

    pub fn get_pool_address(&self) -> AccountId {
        self.pool_address.clone()
    }

    #[payable]
    pub fn set_fees(&mut self, new_fees: u8) {
        assert_eq!(near_sdk::env::signer_account_id(), self.owner);
        self.fees = new_fees;
    }

    pub fn get_fees(&self) -> u8 {
        self.fees
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
    