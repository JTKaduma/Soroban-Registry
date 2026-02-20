#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol};

#[contracttype]
pub enum DataKey {
    ReserveA,
    ReserveB,
    TokenA,
    TokenB,
    LpSupply,
    LpBalance(Address),
}

#[contract]
pub struct {{CONTRACT_NAME}};

#[contractimpl]
impl {{CONTRACT_NAME}} {
    pub fn initialize(env: Env, token_a: Address, token_b: Address) {
        env.storage().instance().set(&DataKey::TokenA, &token_a);
        env.storage().instance().set(&DataKey::TokenB, &token_b);
        env.storage().instance().set(&DataKey::ReserveA, &0_i128);
        env.storage().instance().set(&DataKey::ReserveB, &0_i128);
        env.storage().instance().set(&DataKey::LpSupply, &0_i128);
    }

    pub fn add_liquidity(env: Env, provider: Address, amount_a: i128, amount_b: i128) -> i128 {
        provider.require_auth();
        let reserve_a: i128 = env.storage().instance().get(&DataKey::ReserveA).unwrap_or(0);
        let reserve_b: i128 = env.storage().instance().get(&DataKey::ReserveB).unwrap_or(0);
        let lp_supply: i128 = env.storage().instance().get(&DataKey::LpSupply).unwrap_or(0);

        let lp_minted = if lp_supply == 0 {
            (amount_a * amount_b).isqrt()
        } else {
            ((amount_a * lp_supply) / reserve_a).min((amount_b * lp_supply) / reserve_b)
        };

        env.storage().instance().set(&DataKey::ReserveA, &(reserve_a + amount_a));
        env.storage().instance().set(&DataKey::ReserveB, &(reserve_b + amount_b));
        env.storage().instance().set(&DataKey::LpSupply, &(lp_supply + lp_minted));
        let current: i128 = env.storage().instance().get(&DataKey::LpBalance(provider.clone())).unwrap_or(0);
        env.storage().instance().set(&DataKey::LpBalance(provider), &(current + lp_minted));
        lp_minted
    }

    pub fn swap(env: Env, caller: Address, amount_in: i128, from_a: bool) -> i128 {
        caller.require_auth();
        let reserve_a: i128 = env.storage().instance().get(&DataKey::ReserveA).unwrap_or(0);
        let reserve_b: i128 = env.storage().instance().get(&DataKey::ReserveB).unwrap_or(0);
        let fee_amount = amount_in * 3 / 1000;
        let amount_in_after_fee = amount_in - fee_amount;

        let (amount_out, new_a, new_b) = if from_a {
            let out = (amount_in_after_fee * reserve_b) / (reserve_a + amount_in_after_fee);
            (out, reserve_a + amount_in, reserve_b - out)
        } else {
            let out = (amount_in_after_fee * reserve_a) / (reserve_b + amount_in_after_fee);
            (out, reserve_a - out, reserve_b + amount_in)
        };

        env.storage().instance().set(&DataKey::ReserveA, &new_a);
        env.storage().instance().set(&DataKey::ReserveB, &new_b);
        amount_out
    }

    pub fn get_reserves(env: Env) -> (i128, i128) {
        let a: i128 = env.storage().instance().get(&DataKey::ReserveA).unwrap_or(0);
        let b: i128 = env.storage().instance().get(&DataKey::ReserveB).unwrap_or(0);
        (a, b)
    }
}
