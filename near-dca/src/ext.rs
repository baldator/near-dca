use near_sdk::json_types::U128;
use near_sdk::{ext_contract, near, AccountId, PromiseOrValue};

#[near(serializers = [json])]
pub struct Action {
    pool_id: u64,
    token_in: AccountId,
    token_out: AccountId,
    amount_in: U128,
    min_amount_out: U128,
}

pub fn create_ref_message(
    pool_id: u64,
    token_in: AccountId,
    token_out: AccountId,
    amount_in: u128,
    min_amount_out: u128,
) -> Vec<Action> {
    // Create the RefInnerMsg instance
    let action = Action {
        pool_id,
        token_in,
        token_out,
        amount_in: U128(amount_in),
        min_amount_out: U128(min_amount_out),
    };

    vec![action]
}

// FT transfer interface
#[allow(dead_code)]
#[ext_contract(ext_fungible_token)]
trait FT {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);

    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;

    fn ft_balance_of(&self, account_id: AccountId) -> U128;
}

#[allow(dead_code)]
#[ext_contract(ref_contract)]
trait Ref {
    fn swap(&mut self, actions: Vec<Action>) -> U128;

    fn withdraw(&mut self, token_id: AccountId, amount: U128) -> U128;
}

#[ext_contract(ext_wrap)]
pub trait ExtWrap  {
    fn near_deposit(&mut self);
    fn ft_transfer_call(&mut self, receiver_id: AccountId, amount: U128, msg: Option<String>);
    fn near_withdraw(&mut self, amount: U128);
   // Arguments: {
    //     "receiver_id": "v2.ref-finance.near",
    //     "amount": "1000000000000000000000000",
    //     "msg": "{\"force\":0,\"actions\":[{\"pool_id\":974,\"token_in\":\"wrap.near\",\"token_out\":\"2260fac5e5542a773aa44fbcfedf7c193bc2c599.factory.bridge.near\",\"amount_in\":\"1000000000000000000000000\",\"amount_out\":\"0\",\"min_amount_out\":\"7486\"}]}"
    //   }
}
