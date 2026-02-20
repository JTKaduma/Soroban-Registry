#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
pub enum DataKey {
    Admin,
    Collateral(Address),
    Debt(Address),
    CollateralFactor,
    TotalLiquidity,
}

#[contract]
pub struct {{CONTRACT_NAME}};

#[contractimpl]
impl {{CONTRACT_NAME}} {
    pub fn initialize(env: Env, admin: Address, collateral_factor: u32) {
        admin.require_auth();
        assert!(collateral_factor <= 100, "collateral factor must be <= 100");
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::CollateralFactor, &collateral_factor);
        env.storage().instance().set(&DataKey::TotalLiquidity, &0_i128);
    }

    pub fn supply(env: Env, supplier: Address, amount: i128) {
        supplier.require_auth();
        assert!(amount > 0, "amount must be positive");
        let current: i128 = env.storage().instance().get(&DataKey::Collateral(supplier.clone())).unwrap_or(0);
        env.storage().instance().set(&DataKey::Collateral(supplier), &(current + amount));
        let liq: i128 = env.storage().instance().get(&DataKey::TotalLiquidity).unwrap_or(0);
        env.storage().instance().set(&DataKey::TotalLiquidity, &(liq + amount));
    }

    pub fn borrow(env: Env, borrower: Address, amount: i128) {
        borrower.require_auth();
        let collateral: i128 = env.storage().instance().get(&DataKey::Collateral(borrower.clone())).unwrap_or(0);
        let factor: u32 = env.storage().instance().get(&DataKey::CollateralFactor).unwrap_or(75);
        let max_borrow = collateral * factor as i128 / 100;
        let current_debt: i128 = env.storage().instance().get(&DataKey::Debt(borrower.clone())).unwrap_or(0);
        assert!(current_debt + amount <= max_borrow, "exceeds borrow limit");
        let liq: i128 = env.storage().instance().get(&DataKey::TotalLiquidity).unwrap_or(0);
        assert!(liq >= amount, "insufficient liquidity");
        env.storage().instance().set(&DataKey::Debt(borrower), &(current_debt + amount));
        env.storage().instance().set(&DataKey::TotalLiquidity, &(liq - amount));
    }

    pub fn repay(env: Env, borrower: Address, amount: i128) {
        borrower.require_auth();
        let debt: i128 = env.storage().instance().get(&DataKey::Debt(borrower.clone())).unwrap_or(0);
        let repay_amount = amount.min(debt);
        env.storage().instance().set(&DataKey::Debt(borrower), &(debt - repay_amount));
        let liq: i128 = env.storage().instance().get(&DataKey::TotalLiquidity).unwrap_or(0);
        env.storage().instance().set(&DataKey::TotalLiquidity, &(liq + repay_amount));
    }

    pub fn collateral_of(env: Env, user: Address) -> i128 {
        env.storage().instance().get(&DataKey::Collateral(user)).unwrap_or(0)
    }

    pub fn debt_of(env: Env, user: Address) -> i128 {
        env.storage().instance().get(&DataKey::Debt(user)).unwrap_or(0)
    }
}
