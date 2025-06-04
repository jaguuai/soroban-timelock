#![cfg(test)]
extern crate std;

use super::*;
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Ledger};
use soroban_sdk::{symbol_short, token, vec, Address, Env, IntoVal};
use token::Client as TokenClient;
use token::StellarAssetClient as TokenAdminClient;

/// Utility function to create a token contract and return both its client and admin interfaces.
fn create_token_contract<'a>(e: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let sac = e.register_stellar_asset_contract_v2(admin.clone());
    (
        token::Client::new(e, &sac.address()),
        token::StellarAssetClient::new(e, &sac.address()),
    )
}

/// Utility function to register the claimable balance contract and return its client.
fn create_claimable_balance_contract<'a>(e: &Env) -> ClaimableBalanceContractClient<'a> {
    ClaimableBalanceContractClient::new(e, &e.register(ClaimableBalanceContract, ()))
}

/// Struct to encapsulate and organize all resources used in tests.
struct ClaimableBalanceTest<'a> {
    env: Env,
    deposit_address: Address,
    claim_addresses: [Address; 3],
    token: TokenClient<'a>,
    contract: ClaimableBalanceContractClient<'a>,
}

impl<'a> ClaimableBalanceTest<'a> {
    /// Sets up the test environment: initializes contract, token, balances, and mock addresses.
    fn setup() -> Self {
        let env = Env::default();
        env.mock_all_auths(); // Automatically mock authorization

        // Set current ledger timestamp
        env.ledger().with_mut(|li| {
            li.timestamp = 12345;
        });

        let deposit_address = Address::generate(&env);

        let claim_addresses = [
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
        ];

        let token_admin = Address::generate(&env);

        // Create token and mint initial balance to depositor
        let (token, token_admin_client) = create_token_contract(&env, &token_admin);
        token_admin_client.mint(&deposit_address, &1000);

        let contract = create_claimable_balance_contract(&env);
        ClaimableBalanceTest {
            env,
            deposit_address,
            claim_addresses,
            token,
            contract,
        }
    }
}

#[test]
fn test_deposit_and_claim() {
    let test = ClaimableBalanceTest::setup();

    // Deposit 800 tokens with time bound "before 12346"
    test.contract.deposit(
        &test.deposit_address,
        &test.token.address,
        &800,
        &vec![
            &test.env,
            test.claim_addresses[0].clone(),
            test.claim_addresses[1].clone(),
        ],
        &TimeBound {
            kind: TimeBoundKind::Before,
            timestamp: 12346,
        },
    );

    // Verify the auth trail: deposit includes token transfer
    assert_eq!(
        test.env.auths(),
        [(
            test.deposit_address.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract(( // `deposit` was authorized
                    test.contract.address.clone(),
                    symbol_short!("deposit"),
                    (
                        test.deposit_address.clone(),
                        test.token.address.clone(),
                        800_i128,
                        vec![
                            &test.env,
                            test.claim_addresses[0].clone(),
                            test.claim_addresses[1].clone()
                        ],
                        TimeBound {
                            kind: TimeBoundKind::Before,
                            timestamp: 12346,
                        },
                    )
                        .into_val(&test.env),
                )),
                sub_invocations: std::vec![AuthorizedInvocation { // Token transfer
                    function: AuthorizedFunction::Contract((
                        test.token.address.clone(),
                        symbol_short!("transfer"),
                        (
                            test.deposit_address.clone(),
                            &test.contract.address,
                            800_i128,
                        )
                            .into_val(&test.env),
                    )),
                    sub_invocations: std::vec![]
                }]
            }
        ),]
    );

    // Check balances before claim
    assert_eq!(test.token.balance(&test.deposit_address), 200);
    assert_eq!(test.token.balance(&test.contract.address), 800);
    assert_eq!(test.token.balance(&test.claim_addresses[1]), 0);

    // Perform claim
    test.contract.claim(&test.claim_addresses[1]);

    // Verify authorization and final balances
    assert_eq!(
        test.env.auths(),
        [(
            test.claim_addresses[1].clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    test.contract.address.clone(),
                    symbol_short!("claim"),
                    (test.claim_addresses[1].clone(),).into_val(&test.env),
                )),
                sub_invocations: std::vec![]
            }
        ),]
    );

    assert_eq!(test.token.balance(&test.deposit_address), 200);
    assert_eq!(test.token.balance(&test.contract.address), 0);
    assert_eq!(test.token.balance(&test.claim_addresses[1]), 800);
}

#[test]
#[should_panic(expected = "contract has been already initialized")]
fn test_double_deposit_not_possible() {
    let test = ClaimableBalanceTest::setup();

    // First deposit works
    test.contract.deposit(
        &test.deposit_address,
        &test.token.address,
        &1,
        &vec![&test.env, test.claim_addresses[0].clone()],
        &TimeBound {
            kind: TimeBoundKind::Before,
            timestamp: 12346,
        },
    );

    // Second deposit should panic (already initialized)
    test.contract.deposit(
        &test.deposit_address,
        &test.token.address,
        &1,
        &vec![&test.env, test.claim_addresses[0].clone()],
        &TimeBound {
            kind: TimeBoundKind::Before,
            timestamp: 12346,
        },
    );
}

#[test]
#[should_panic(expected = "claimant is not allowed to claim this balance")]
fn test_unauthorized_claim_not_possible() {
    let test = ClaimableBalanceTest::setup();

    // Setup with claimants 0 and 1
    test.contract.deposit(
        &test.deposit_address,
        &test.token.address,
        &800,
        &vec![
            &test.env,
            test.claim_addresses[0].clone(),
            test.claim_addresses[1].clone(),
        ],
        &TimeBound {
            kind: TimeBoundKind::Before,
            timestamp: 12346,
        },
    );

    // Claim attempt by address 2 should panic
    test.contract.claim(&test.claim_addresses[2]);
}

#[test]
#[should_panic(expected = "time predicate is not fulfilled")]
fn test_out_of_time_bound_claim_not_possible() {
    let test = ClaimableBalanceTest::setup();

    // Deposit requires AFTER 12346, but ledger timestamp is 12345 -> should fail
    test.contract.deposit(
        &test.deposit_address,
        &test.token.address,
        &800,
        &vec![&test.env, test.claim_addresses[0].clone()],
        &TimeBound {
            kind: TimeBoundKind::After,
            timestamp: 12346,
        },
    );

    test.contract.claim(&test.claim_addresses[0]); // Should panic due to time condition
}

#[test]
#[should_panic]
fn test_double_claim_not_possible() {
    let test = ClaimableBalanceTest::setup();

    // Valid deposit and claim
    test.contract.deposit(
        &test.deposit_address,
        &test.token.address,
        &800,
        &vec![&test.env, test.claim_addresses[0].clone()],
        &TimeBound {
            kind: TimeBoundKind::Before,
            timestamp: 12346,
        },
    );

    test.contract.claim(&test.claim_addresses[0]);
    assert_eq!(test.token.balance(&test.claim_addresses[0]), 800);

    // Second claim should panic as balance was already claimed
    test.contract.claim(&test.claim_addresses[0]);
}

#[test]
#[should_panic(expected = "contract has been already initialized")]
fn test_deposit_after_claim_not_possible() {
    let test = ClaimableBalanceTest::setup();

    // Deposit and valid claim
    test.contract.deposit(
        &test.deposit_address,
        &test.token.address,
        &800,
        &vec![&test.env, test.claim_addresses[0].clone()],
        &TimeBound {
            kind: TimeBoundKind::After,
            timestamp: 12344,
        },
    );

    test.contract.claim(&test.claim_addresses[0]);
    assert_eq!(test.token.balance(&test.claim_addresses[0]), 800);

    // Re-deposit attempt should panic due to one-time init guard
    test.contract.deposit(
        &test.deposit_address,
        &test.token.address,
        &200,
        &vec![&test.env, test.claim_addresses[0].clone()],
        &TimeBound {
            kind: TimeBoundKind::After,
            timestamp: 12344,
        },
    );
}
