// Example Soroban contract with various linting issues for demonstration
use soroban_sdk::{contract, contractimpl, Env, Address, Symbol, symbol_short};

const STORAGE_KEY_BALANCE: &str = "balance";  // Potential storage key collision
const STORAGE_KEY_BALANCE: &str = "balance";  // Duplicate key

#[contract]
pub struct TokenContract;

#[contractimpl]
impl TokenContract {
    /// Transfer tokens without auth check
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) -> Result<(), String> {
        // Missing auth check - should call env.require_auth(&from)
        
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
    
    /// Mint new tokens with unbounded loop
    pub fn mint(env: Env, amount: u64) {
        let mut counter = 0;
        loop {
            env.storage().persistent().set(
                &Symbol::new(&env, "total_supply"),
                &(amount as i128),
            );
            counter += 1;
            // Issue: unbounded loop without explicit break
        }
    }
    
    /// Transfer tokens with potential reentrancy
    pub fn send(env: Env, to: Address, amount: i128) {
        // Cross-contract call before state modification
        env.invoke_contract::<_, ()>(&to, &Symbol::new(&env, "receive"), (amount,));
        
        // State modification after cross-contract call
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
    let contract = TokenContract;
    
    // Test code can use unwrap - this should NOT trigger
    let val = Some(42).unwrap();
    assert_eq!(val, 42);
}
