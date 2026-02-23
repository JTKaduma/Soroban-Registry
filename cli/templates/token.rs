#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Map, Symbol};

#[contracttype]
pub enum DataKey {
    Balance(Address),
    Allowance(Address, Address),
    TotalSupply,
    Name,
    Symbol,
    Admin,
}

#[contract]
pub struct {{CONTRACT_NAME}};

#[contractimpl]
impl {{CONTRACT_NAME}} {
    pub fn initialize(env: Env, admin: Address, name: Symbol, symbol: Symbol, supply: i128) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Name, &name);
        env.storage().instance().set(&DataKey::Symbol, &symbol);
        env.storage().instance().set(&DataKey::TotalSupply, &supply);
        env.storage().instance().set(&DataKey::Balance(admin.clone()), &supply);
    }

    pub fn balance(env: Env, owner: Address) -> i128 {
        env.storage().instance().get(&DataKey::Balance(owner)).unwrap_or(0)
    }

    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        let from_balance: i128 = Self::balance(env.clone(), from.clone());
        assert!(from_balance >= amount, "insufficient balance");
        env.storage().instance().set(&DataKey::Balance(from), &(from_balance - amount));
        let to_balance: i128 = Self::balance(env.clone(), to.clone());
        env.storage().instance().set(&DataKey::Balance(to), &(to_balance + amount));
    }

    pub fn total_supply(env: Env) -> i128 {
        env.storage().instance().get(&DataKey::TotalSupply).unwrap_or(0)
    }

    pub fn name(env: Env) -> Symbol {
        env.storage().instance().get(&DataKey::Name).unwrap()
    }
}
