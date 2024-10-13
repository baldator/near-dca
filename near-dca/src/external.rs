use near_sdk::ext_contract;

pub const NO_DEPOSIT: u128 = 0;
pub const XCC_SUCCESS: u64 = 1;

// Validator interface, for cross-contract calls
#[ext_contract(ref_finance)]
trait Contract  {
    fn get_greeting(&self) -> String;
    fn swap(&self, actions: Vec<SwapAction>, referral_id: Option<ValidAccountId>) -> U128;
}

#[ext_contract(ext_fungible_token)]
pub trait Contract  {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

#[ext_contract(ext_self)]
pub trait ExtSelf {
    fn callback_post_withdraw_reward(
        &mut self,
        token_id: AccountId,
        sender_id: AccountId,
        amount: U128,
    );
}