import { ContractExample } from './api';

export const MOCK_EXAMPLES: Record<string, ContractExample[]> = {
  // Hello World Oracle ID (from the seed data, or any placeholder)
  'CC56J7J77K56J7K56J7K56J7K56J7K56J7K56J7K56J7K56J': [
    {
      id: 'mock-1',
      contract_id: 'CC56J7J77K56J7K56J7K56J7K56J7K56J7K56J7K56J7K56J',
      title: 'Initialize Contract',
      description: 'How to initialize the contract client and invoke the hello function.',
      category: 'basic',
      rating_up: 15,
      rating_down: 2,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      code_js: `import { Contract, Networks } from "@stellar/stellar-sdk";

const contractId = "CC56J7J77K56J7K56J7K56J7K56J7K56J7K56J7K56J7K56J";
const contract = new Contract(contractId);

// Simulate the "hello" function
// Note: In a real app you would use a wallet and server.invoke()
console.log("Contract initialized:", contractId);
console.log("Ready to invoke methods.");`,
      code_rust: `#![no_std]
use soroban_sdk::{contractimpl, Env, Symbol, symbol_short};

pub struct HelloContract;

#[contractimpl]
impl HelloContract {
    pub fn hello(env: Env, to: Symbol) -> Symbol {
        symbol_short!("Hello")
    }
}`
    },
    {
      id: 'mock-2',
      contract_id: 'CC56J7J77K56J7K56J7K56J7K56J7K56J7K56J7K56J7K56J',
      title: 'Reading Storage',
      description: 'How to read the greeting value from the ledger.',
      category: 'advanced',
      rating_up: 8,
      rating_down: 0,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      code_js: `// Assuming the contract has a "get_greeting" method
const op = contract.call("get_greeting");
console.log("built operation:", op);`,
      code_rust: `pub fn get_greeting(env: Env) -> Symbol {
    env.storage().instance().get(&symbol_short!("GREET")).unwrap()
}`
    },
    {
      id: 'mock-3',
      contract_id: 'CC56J7J77K56J7K56J7K56J7K56J7K56J7K56J7K56J7K56J',
      title: 'Cross-Contract Call',
      description: 'Calling this contract from another contract.',
      category: 'integration',
      rating_up: 22,
      rating_down: 1,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      code_rust: `pub fn call_hello(env: Env, address: Address) {
    let client = HelloContractClient::new(&env, &address);
    client.hello(&symbol_short!("Dev"));
}`
    }
  ]
};
