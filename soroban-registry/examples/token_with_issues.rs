// Example Soroban contract with various linting issues for demonstration
// Vulnerability note: missing require_auth allows anyone to transfer from any account.
use soroban_sdk::{contract, contractimpl, Env, Address, Symbol, symbol_short};

const STORAGE_KEY_BALANCE: &str = "balance";  // Potential storage key collision
// i removed const STORAGE_KEY_BALANCE: &str = "balance"; because the duplicate key was a compile error.


/// Maximum number of iterations allowed in the mint function.
/// This prevents instruction budget exhaustion on the Stellar network.
/// Soroban contracts are subject to strict CPU instruction limits per transaction.
const MAX_MINT_ITERATIONS: u64 = 1_000;

pub struct TokenContract;

#[contractimpl]
impl TokenContract {
    /// Transfer tokens with auth check (secure version)
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) -> Result<(), String> {
        from.require_auth();
        
        let current_balance = env.storage()
            .persistent()
            .get::<_, i128>(&Symbol::new(&env, "balance"))
            .unwrap_or(0);  // Issue: unwrap in public function
        
        if current_balance < amount {
            panic!("Insufficient balance");  // Issue: panic in contract
        }
        
        let new_balance = current_balance + amount;  // Issue: unchecked arithmetic
        env.storage().persistent().set(&Symbol::new(&env, "balance"), &new_balance);
        
        Ok(())
    }

    /// Transfer tokens without auth check (vulnerable version for comparison)
    pub fn transfer_vulnerable(env: Env, from: Address, to: Address, amount: i128) -> Result<(), String> {
        // Missing auth check - should call env.require_auth(&from)

        let current_balance = env.storage()
            .persistent()
            .get::<_, i128>(&Symbol::new(&env, "balance"))
            .unwrap_or(0);

        if current_balance < amount {
            panic!("Insufficient balance");
        }

        let new_balance = current_balance + amount;
        env.storage().persistent().set(&Symbol::new(&env, "balance"), &new_balance);

        Ok(())
    }
    
    /// Approve tokens with hardcoded address
    pub fn approve(env: Env, owner: Address) {
        let admin = "GBBD47UZQ5CZKRQFWWXD4ZCSWI5GGMOWYCFTEUQMDFEBNFNJ5VQJEWWV";  // Issue: hardcoded address
        
        env.storage().persistent().remove(&Symbol::new(&env, "allowance"));  // Issue: direct storage clear
        
        let unused_var = 42;  // Issue: unused variable
        
        Ok(())
    }
    
    /// Get balance without documentation
    pub fn get_balance(env: Env, account: Address) -> i128 {
        env.storage()
            .persistent()
            .get::<_, i128>(&Symbol::new(&env, "balance"))
            .expect("No balance found")  // Issue: expect() in public function
    }
    
/// Mint new tokens up to a bounded iteration limit.
///
/// # Resource Limits
///
/// - Maximum iterations: `MAX_MINT_ITERATIONS` (1,000)
/// - `amount` must be greater than 0
/// - `amount` must not exceed `MAX_MINT_ITERATIONS`
///
/// These limits are intentionally enforced because Soroban contracts execute
/// within a fixed CPU instruction budget per transaction. Allowing an
/// unbounded loop could exhaust that budget, causing the transaction to fail
/// or potentially opening the door to denial-of-service (DoS) scenarios.
///
/// By capping iterations and validating input early, we ensure predictable
/// resource usage and safer execution.
///
/// # Panics
///
/// - Panics if `amount` is 0
/// - Panics if `amount` exceeds `MAX_MINT_ITERATIONS`
       pub fn mint(env: Env, amount: u64) {
        // Parameter validation
        if amount == 0 {
            panic!("amount must be greater than zero");
        }
        if amount > MAX_MINT_ITERATIONS {
            panic!(
                "amount exceeds maximum allowed iterations ({})",
                MAX_MINT_ITERATIONS
            );
        }

        let mut counter: u64 = 0;
        loop {
            // early exit when iteration limit is reached
            if counter >= amount || counter >= MAX_MINT_ITERATIONS {
                break;
            }

            env.storage().persistent().set(
                &Symbol::new(&env, "total_supply"),
                &(amount as i128),
            );
            counter += 1;
        }
    }
    
    /// Transfer tokens with reentrancy protection (checks-effects-interactions + guard)
    pub fn send(env: Env, to: Address, amount: i128) {
        let balance_key = Symbol::new(&env, "balance");
        let guard_key = Symbol::new(&env, "reentrancy_guard");

        let guard_active = env.storage().persistent().get::<_, bool>(&guard_key).unwrap_or(false);
        if guard_active {
            panic!("Reentrancy detected");
        }

        // Effects before interactions
        let current = env.storage().persistent().get::<_, i128>(&balance_key).unwrap_or(0);
        env.storage().persistent().set(&balance_key, &(current - amount));

        // Guard during external call
        env.storage().persistent().set(&guard_key, &true);
        env.invoke_contract::<_, ()>(&to, &Symbol::new(&env, "receive"), (amount,));
        env.storage().persistent().set(&guard_key, &false);
    }

    /// Vulnerable send for comparison (reentrancy risk)
    pub fn send_vulnerable(env: Env, to: Address, amount: i128) {
        env.invoke_contract::<_, ()>(&to, &Symbol::new(&env, "receive"), (amount,));

        let balance_key = Symbol::new(&env, "balance");
        let current = env.storage().persistent().get::<_, i128>(&balance_key).unwrap_or(0);
        env.storage().persistent().set(&balance_key, &(current - amount));
    }
    
    /// Inefficient clone usage
    pub fn process(env: Env, data: String) -> String {
        data.clone().clone()  // Issue: redundant clone
    }
}



#[test]
fn test_transfer() {
    let env = Env::new();
    
    // Test code can use unwrap - this should NOT trigger
    let val = Some(42).unwrap();
    assert_eq!(val, 42);
}

/// Verifies that minting with a valid amount completes successfully without panicking.
#[test]
fn test_mint_valid_amount() {
    let env = Env::new();
    // amount = 10, well within the MAX_MINT_ITERATIONS limit
    TokenContract::mint(env, 10);
}

/// Verifies that minting with the exact MAX_MINT_ITERATIONS amount completes successfully without panicking.
#[test]
fn test_mint_exact_limit() {
    let env = Env::new();
    TokenContract::mint(env, MAX_MINT_ITERATIONS);
}

/// Mint with amount = 0 must panic (invalid input).
#[test]
#[should_panic(expected = "amount must be greater than zero")]
fn test_mint_zero_amount() {
    let env = Env::new();
    TokenContract::mint(env, 0);
}

#[test]
#[should_panic(expected = "amount exceeds maximum allowed iterations")]
fn test_mint_exceeds_limit() {
    let env = Env::new();
    TokenContract::mint(env, MAX_MINT_ITERATIONS + 1);
}

#[test]
#[should_panic]
fn test_reentrancy_guard_blocks_recursive_call() {
    use soroban_sdk::testutils::Address as AddressTestutils;

    let env = Env::new();
fn test_transfer_requires_auth() {
    use soroban_sdk::testutils::Address as AddressTestutils;

    let env = Env::new();
    let from = Address::generate(&env);
    let to = Address::generate(&env);

    env.storage()
        .persistent()
        .set(&Symbol::new(&env, "balance"), &100i128);
    env.storage()
        .persistent()
        .set(&Symbol::new(&env, "reentrancy_guard"), &true);

    TokenContract::send(env, to, 10);

    let _ = TokenContract::transfer(env, from, to, 10);
}

#[test]
fn test_transfer_authorized() {
    use soroban_sdk::testutils::Address as AddressTestutils;

    let env = Env::new();
    env.mock_all_auths();

    let from = Address::generate(&env);
    let to = Address::generate(&env);

    env.storage()
        .persistent()
        .set(&Symbol::new(&env, "balance"), &100i128);

    let result = TokenContract::transfer(env, from, to, 10);
    assert!(result.is_ok());
}
