#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol};

#[contracttype]
pub enum DataKey {
    Admin,
    Price(Symbol),
    LastUpdate(Symbol),
    Decimals,
}

#[contract]
pub struct {{CONTRACT_NAME}};

#[contractimpl]
impl {{CONTRACT_NAME}} {
    pub fn initialize(env: Env, admin: Address, decimals: u32) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Decimals, &decimals);
    }

    pub fn set_price(env: Env, asset: Symbol, price: i128) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        assert!(price > 0, "price must be positive");
        env.storage().instance().set(&DataKey::Price(asset.clone()), &price);
        env.storage().instance().set(&DataKey::LastUpdate(asset), &env.ledger().timestamp());
    }

    pub fn get_price(env: Env, asset: Symbol) -> i128 {
        env.storage().instance().get(&DataKey::Price(asset)).unwrap_or(0)
    }

    pub fn last_update(env: Env, asset: Symbol) -> u64 {
        env.storage().instance().get(&DataKey::LastUpdate(asset)).unwrap_or(0)
    }

    pub fn decimals(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::Decimals).unwrap_or(7)
    }
}
