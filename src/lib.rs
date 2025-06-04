#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Vec};

/// Enum used as storage keys for the contract.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Init,     // Indicates whether the contract has been initialized
    Balance,  // Stores the claimable balance data
}

/// Enum representing the type of time-bound restriction.
#[derive(Clone)]
#[contracttype]
pub enum TimeBoundKind {
    Before,  // Claim allowed before a given timestamp
    After,   // Claim allowed after a given timestamp
}

/// Struct representing the time constraint for claiming.
#[derive(Clone)]
#[contracttype]
pub struct TimeBound {
    pub kind: TimeBoundKind,  // Type of constraint: Before or After
    pub timestamp: u64,       // UNIX timestamp used as time threshold
}

/// Struct representing a claimable token balance with a time lock and designated claimants.
#[derive(Clone)]
#[contracttype]
pub struct ClaimableBalance {
    pub token: Address,           // Address of the token contract
    pub amount: i128,             // Amount of tokens to claim
    pub claimants: Vec<Address>,  // List of addresses allowed to claim
    pub time_bound: TimeBound,    // Time-bound condition for claiming
}

#[contract]
pub struct ClaimableBalanceContract;

/// Internal helper function to evaluate if the current ledger timestamp satisfies the given time-bound condition.
fn check_time_bound(env: &Env, time_bound: &TimeBound) -> bool {
    let ledger_timestamp = env.ledger().timestamp();

    match time_bound.kind {
        TimeBoundKind::Before => ledger_timestamp <= time_bound.timestamp,
        TimeBoundKind::After => ledger_timestamp >= time_bound.timestamp,
    }
}

#[contractimpl]
impl ClaimableBalanceContract {
    /// Deposits a claimable token balance to the contract, locked by a time condition and restricted to specific claimants.
    pub fn deposit(
        env: Env,
        from: Address,              // Address sending the tokens
        token: Address,             // Token contract address
        amount: i128,               // Amount of tokens to deposit
        claimants: Vec<Address>,    // Allowed claimants
        time_bound: TimeBound,      // Time-bound constraint
    ) {
        // Enforce a maximum number of claimants
        if claimants.len() > 10 {
            panic!("too many claimants");
        }

        // Ensure the contract is not already initialized
        if is_initialized(&env) {
            panic!("contract has been already initialized");
        }

        // Require that 'from' address authorizes this call
        from.require_auth();

        // Transfer tokens from 'from' address to this contract
        token::Client::new(&env, &token).transfer(&from, &env.current_contract_address(), &amount);

        // Store the claimable balance data in contract storage
        env.storage().instance().set(
            &DataKey::Balance,
            &ClaimableBalance {
                token,
                amount,
                time_bound,
                claimants,
            },
        );

        // Mark contract as initialized to prevent further deposits
        env.storage().instance().set(&DataKey::Init, &());
    }

    /// Allows a designated claimant to claim the locked token balance if the time condition is met.
    pub fn claim(env: Env, claimant: Address) {
        // Require that claimant authorizes the claim
        claimant.require_auth();

        // Retrieve the stored claimable balance; panic if already claimed
        let claimable_balance: ClaimableBalance =
            env.storage().instance().get(&DataKey::Balance).unwrap();

        // Check if current time satisfies the time condition
        if !check_time_bound(&env, &claimable_balance.time_bound) {
            panic!("time predicate is not fulfilled");
        }

        // Check if the claimant is among the allowed addresses
        let claimants = &claimable_balance.claimants;
        if !claimants.contains(&claimant) {
            panic!("claimant is not allowed to claim this balance");
        }

        // Transfer the token amount to the claimant
        token::Client::new(&env, &claimable_balance.token).transfer(
            &env.current_contract_address(),
            &claimant,
            &claimable_balance.amount,
        );

        // Remove the claimable balance entry after successful claim
        env.storage().instance().remove(&DataKey::Balance);
    }
}

/// Helper function to check if the contract has already been initialized with a deposit.
fn is_initialized(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Init)
}

// Test module is defined in a separate file.
mod test;
