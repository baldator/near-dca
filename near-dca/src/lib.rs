// Find all our documentation at https://docs.near.org
use near_sdk::json_types::U128;
use near_sdk::{
 AccountId, near, PanicOnDefault, env, Promise, NearToken, log, Gas, PromiseError, near_bindgen
};
use std::collections::HashMap;
use ext::{create_ref_message, ext_fungible_token, ext_wrap, ref_contract};

pub const GAS_FOR_FT_TRANSFER: Gas = Gas::from_gas(20_000_000_000_000);
pub const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas::from_gas(20_000_000_000_000);
pub const YOCTO_DEPOSIT: NearToken = NearToken::from_yoctonear(1);


pub mod ext;

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
    pub reverse: bool,
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
    pub fn register_user(&mut self, amount_per_swap: U128, swap_interval: u64, reverse: Option<bool>) {
        // get attached deposit
        let amount = env::attached_deposit();
        assert!(amount.as_yoctonear() > 0, "Deposit must be greater than 0");
        assert!(amount.as_yoctonear() > amount_per_swap.0, "Deposit must be greater than swap amount");

        // user must not exist
        assert!(!self.users.contains_key(&env::signer_account_id()), "User already exists");

        // check if reverse is not set then set it to false
        let reverse_flag = match reverse {
            Some(reverse) => reverse,
            None => false
        };
        
        let user = User {
            wallet: env::signer_account_id(),
            amount_per_swap,
            swap_interval,
            last_swap_timestamp: 0,
            total_swapped: U128(0),
            amount: amount.as_yoctonear().into(),
            pause: false,
            reverse: reverse_flag,
        };
        self.users.insert(env::signer_account_id(), user);
        self.user_addresses.push(env::signer_account_id());

        // wrap the amount
        ext_wrap::ext(self.wrap_account.clone())
            .with_static_gas(GAS_FOR_FT_TRANSFER)
            .with_attached_deposit(NearToken::from_yoctonear(amount.as_yoctonear().into()))
            .near_deposit();
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

        // wrap the amount
        ext_wrap::ext(self.wrap_account.clone())
            .with_static_gas(GAS_FOR_FT_TRANSFER)
            .with_attached_deposit(NearToken::from_yoctonear(amount.as_yoctonear().into()))
            .near_deposit();
    }

    #[payable]
    pub fn withdraw_near(&mut self, amount: U128) {
        let deposit = env::attached_deposit();
        assert!(deposit.as_yoctonear() == 1, "Deposit must be 1");
        // user must exist
        assert!(self.users.contains_key(&env::signer_account_id()), "User does not exist");
        let mut user = self.users.get(&env::signer_account_id()).unwrap().clone();

        // check if user has enough balance
        assert!(user.amount >= amount, "User does not have enough balance");

        let new_amount = user.amount.0.checked_sub(amount.0).expect("Insufficient funds");
        user.amount = U128(new_amount); // subtract amount;
        self.users.insert(env::signer_account_id(), user.clone());

        let near_amount: NearToken = NearToken::from_yoctonear(amount.0);

        // wrap the amount
        ext_wrap::ext(self.wrap_account.clone())
            .with_static_gas(GAS_FOR_FT_TRANSFER)
            .with_attached_deposit(YOCTO_DEPOSIT)
            .near_withdraw(amount).
        then(Promise::new(env::signer_account_id()).transfer(near_amount));
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
        let deposit = env::attached_deposit();
        assert!(deposit.as_yoctonear() == 1, "Deposit must be 1");
        // user must exist
        assert!(self.users.contains_key(&env::signer_account_id()), "User does not exist");

        // withdraw all funds
        self.withdraw_near(self.users.get(&env::signer_account_id()).unwrap().amount);
        self.withdraw_ft(self.users.get(&env::signer_account_id()).unwrap().total_swapped);

        // remove user from users map
        self.users.remove(&env::signer_account_id());
    }

    #[payable]
    pub fn change_swap_interval(&mut self, swap_interval: u64) {
        // user must exist
        assert!(self.users.contains_key(&env::signer_account_id()), "User does not exist");

        let mut user = self.users.get(&env::signer_account_id()).unwrap().clone();
        user.swap_interval = swap_interval;
        self.users.insert(env::signer_account_id(), user.clone());
    }

    pub fn can_swap(&self, reverse: Option<bool>) -> bool {
        
        // check if reverse is not set then set it to false
        let reverse_flag = match reverse {
            Some(reverse) => reverse,
            None => false
        };

        for (_, user) in self.users.iter() {
            // check if user has to swap and if it is not paused
            // create a mutable copy of the user
            let mut _tmp_user = user.clone();

            if env::block_timestamp() >= user.last_swap_timestamp + user.swap_interval && user.pause == false && user.reverse == reverse_flag {
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
    pub fn swap(&mut self, reverse: Option<bool>) {
        assert_eq!(env::signer_account_id(), self.owner);
        // iterate over all users
        // check if timestamp is greater than last_swap_timestamp + swap_interval
        // if yes add them to the batch

        // check if reverse is not set then set it to false
        let reverse_flag = match reverse {
            Some(reverse) => reverse,
            None => false
        };

        let mut batch_amount:U128 = U128(0);
        let mut batch_users:Vec<AccountId> = Vec::new();
        
        for (_, user) in self.users.iter() {
            // check if user has to swap and if it is not paused
            // create a mutable copy of the user
            let mut _tmp_user = user.clone();

            if env::block_timestamp() >= user.last_swap_timestamp + user.swap_interval && user.pause == false && user.reverse == reverse_flag {
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


        let target_ft_account;
        // format the actions
        if reverse_flag == false {
            target_ft_account = self.wrap_account.clone();
        }
        else {
            target_ft_account = self.token_address.clone();
        }

        ext_wrap::ext(target_ft_account.clone())
            .with_static_gas(Gas::from_tgas(30))
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .ft_transfer_call(
                self.pool_address.clone(),
                batch_amount_total.into(),
                Some("".to_string()),
            )
        .then(
            Self::ext(env::current_account_id())
                .with_static_gas(Gas::from_tgas(150))
                .pool_transfer_callback(
                    batch_users,
                    batch_amount,
                    batch_amount_total,
                    reverse_flag
        ));
    }

    #[private]
    pub fn pool_transfer_callback(&mut self, batch_users:Vec<AccountId>, batch_amount:U128, batch_amount_total:u128, reverse: bool, #[callback_result] call_result: Result<String, PromiseError>,) {
        assert_eq!(env::promise_results_count(), 1);
        if call_result.is_err() {
            log!("There was an error while swapping");
            // we should rollback the transaction
            // self.rollback();
            
            return;
        }

        let amount = call_result.unwrap();
        
        let action;
        if reverse == false {
            action = create_ref_message(
                self.pool_id.into(),
                self.wrap_account.clone(),
                self.token_address.clone(),
                amount.parse::<u128>().unwrap(),
                0,
            );
        }
        else {
            action = create_ref_message(
                self.pool_id.into(),
                self.token_address.clone(),
                self.wrap_account.clone(),
                amount.parse::<u128>().unwrap(),
                0,
            );
        }

        ref_contract::ext(self.pool_address.clone())
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(Gas::from_tgas(30))
            .swap(action)
            .then(
                Self::ext(env::current_account_id())
                .with_static_gas(Gas::from_tgas(150))
                .pool_swap_callback(batch_users, batch_amount, batch_amount_total, reverse)
            );

        
    }

    #[private]
    pub fn pool_swap_callback(&mut self, batch_users:Vec<AccountId>, batch_amount:U128, batch_amount_total:u128, reverse: bool, #[callback_result] call_result: Result<U128, PromiseError>,) -> HashMap<AccountId, u128> {
        let amount = call_result.unwrap();
        
        let token_out;
        if reverse == false {
            token_out = self.token_address.clone();
        }        
        else {
            token_out = self.wrap_account.clone();
        }

        ref_contract::ext(self.pool_address.clone())
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(Gas::from_tgas(30))
            .withdraw(token_out.clone(), amount);

        // initialize the return value
        let mut return_value: HashMap<AccountId, u128> = HashMap::new();

        // update last_swap_timestamp, total_swapped and amount for users in the batch
        for user in batch_users {
            let mut user_tmp: User = self.users.get(&user.clone()).unwrap().clone();
            user_tmp.last_swap_timestamp = env::block_timestamp();
            // get the percentage of total_swapped and amount
            let target_amount = (((user_tmp.amount_per_swap.0*100) / batch_amount.0) * amount.0)/100;
            let final_amount = target_amount.checked_sub(target_amount.checked_mul(self.fees as u128).unwrap_or(0) / 10000).unwrap_or(0);
            user_tmp.total_swapped = U128(user_tmp.total_swapped.0 +target_amount);
            let new_amount = user_tmp.amount.0.checked_sub(user_tmp.amount_per_swap.0).expect("Insufficient funds");
            user_tmp.amount = U128(new_amount);
            self.users.insert(user_tmp.wallet.clone(), user_tmp.clone());
            // log the swap
            if reverse == false {
                log!("<swapLog> {{\"user\": \"{}\", \"source\": \"{}\", \"source_amount\": {}, \"target\": \"{}\", \"target_amount\": \"{}\"}}", user_tmp.wallet.clone(), self.wrap_account, user_tmp.amount_per_swap.0, self.token_address, final_amount);
            }
            else {
                log!("<swapLog> {{\"user\": \"{}\", \"source\": \"{}\", \"source_amount\": {}, \"target\": \"{}\", \"target_amount\": \"{}\"}}", user_tmp.wallet.clone(), self.token_address, user_tmp.amount_per_swap.0, self.wrap_account, final_amount);
            }
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
    trait that will be used as the callback from the FT contract. When ft_transfer_call is
    called, it will fire a cross contract call to this marketplace and this is the function
    that is invoked. 
*/
trait FungibleTokenReceiver {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128
    ) -> U128;
}

//implementation of the trait
#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    /// This is how users will fund their FT balances in the contract
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128
    ) -> U128 {
        // get the contract ID which is the predecessor
        let ft_contract_id = env::predecessor_account_id();
        // Ensure only the specified FT can be used
        // check if the predecessor is the FT contract
        assert_eq!(
            ft_contract_id, self.token_address, "The FT token accepted is {}", self.token_address
        );
        
        //get the signer which is the person who initiated the transaction
        let signer_id = env::signer_account_id();

        //make sure that the signer isn't the predecessor. This is so that we're sure
        //this was called via a cross-contract call
        assert_ne!(
            ft_contract_id,
            signer_id,
            "ft_on_transfer should only be called via cross-contract call"
        );
        //make sure the owner ID is the signer. 
        assert_eq!(
            sender_id,
            signer_id,
            "sender_id should be signer_id"
        );
    
        
        // user must exist
        assert!(self.users.contains_key(&env::signer_account_id()), "User does not exist");

        let mut user = self.users.get(&env::signer_account_id()).unwrap().clone();

        user.amount = U128(user.total_swapped.0 + amount.0); // add amount;
        self.users.insert(env::signer_account_id(), user.clone());

        // We don't return any FTs to the sender because we're storing all of them in their balance
        U128(0)
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
    