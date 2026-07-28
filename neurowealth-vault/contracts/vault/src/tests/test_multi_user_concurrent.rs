//! Multi-user concurrent deposit/withdraw invariant test (#413).
//!
//! Simulates multiple users depositing and withdrawing in interleaved order,
//! verifying the vault solvency invariant after every operation:
//!
//!   idle_balance + deployed_assets == total_assets
//!   total_shares > 0  =>  total_assets > 0
//!
//! Also verifies that every user's share-derived balance is non-negative and
//! that the sum of individual balances stays consistent with total_assets.

use super::utils::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

const MIN_DEPOSIT: i128 = 1_000_000;
const MAX_DEPOSIT: i128 = 10_000_000_000;
const NUM_USERS: usize = 5;
const STEPS: usize = 100;

fn assert_vault_invariants(client: &NeuroWealthVaultClient, users: &[Address]) {
    let total_assets = client.get_total_assets();
    let total_shares = client.get_total_shares();
    let idle = client.get_idle_balance();
    let deployed = client.get_deployed_assets();

    // Core invariant: idle + deployed == total_assets
    assert_eq!(
        idle + deployed,
        total_assets,
        "solvency invariant violated: idle({}) + deployed({}) != total_assets({})",
        idle,
        deployed,
        total_assets
    );

    // If there are shares, there must be assets
    assert!(
        total_shares <= 0 || total_assets > 0,
        "share invariant violated: total_shares({}) > 0 but total_assets({}) <= 0",
        total_shares,
        total_assets
    );

    // Each user's balance must be non-negative
    let mut sum_balances: i128 = 0;
    for user in users {
        let bal = client.get_balance(user);
        assert!(bal >= 0, "user balance cannot be negative: got {}", bal);
        sum_balances = sum_balances
            .checked_add(bal)
            .expect("balance sum overflow");
    }

    // The total of all user get_balance() should be <= total_assets.
    // It can be slightly less due to rounding (floor conversion).
    assert!(
        sum_balances <= total_assets,
        "balance invariant violated: sum_user_balances({}) > total_assets({})",
        sum_balances,
        total_assets
    );
}

/// Simpler pseudo-random number generator (PCG minimal).
fn pcg(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    *state
}

#[test]
fn test_multi_user_concurrent_deposit_withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let token = TestTokenClient::new(&env, &usdc_token);

    // Create multiple users, each funded with 50_000 XLM worth of tokens
    let mut users: Vec<Address> = Vec::with_capacity(NUM_USERS);
    for _ in 0..NUM_USERS {
        let user = Address::generate(&env);
        token.mint(&user, &50_000_000_000);
        assert_eq!(token.balance(&user), 50_000_000_000);
        users.push(user);
    }

    let mut rng: u64 = 0xDEAD_BEEF_CAFE_u64;

    for step in 0..STEPS {
        let user_idx = (pcg(&mut rng) as usize) % NUM_USERS;
        let user = &users[user_idx];
        let op = pcg(&mut rng) % 4; // 0=deposit, 1=withdraw, 2=deposit, 3=withdraw

        // Determine amount from PRNG
        let amount_base = (pcg(&mut rng) as i128 % 100) + 1; // 1..=100
        let amount = MIN_DEPOSIT * amount_base;

        match op {
            0 | 2 => {
                // Deposit
                let token_balance = token.balance(user);
                if amount > MAX_DEPOSIT || token_balance < amount {
                    continue;
                }
                client.deposit(user, &amount);
            }
            1 | 3 => {
                // Withdraw
                let balance = client.get_balance(user);
                if balance < MIN_DEPOSIT {
                    continue;
                }
                let withdraw_amount = amount.min(balance / 2).max(MIN_DEPOSIT);
                client.withdraw(user, &withdraw_amount);
            }
            _ => unreachable!(),
        }

        // Verify invariants after every operation
        assert_vault_invariants(&client, &users);

        // Verify user-level consistency: shares and balance
        let shares = client.get_shares(user);
        let balance = client.get_balance(user);
        assert!(shares >= 0, "negative shares for user {} at step {}", user_idx, step);
        assert!(balance >= 0, "negative balance for user {} at step {}", user_idx, step);
        // If user has shares, they should have a balance (or be zero after withdraw-all)
        // If user has no shares, balance must be 0
        if shares == 0 {
            assert_eq!(balance, 0, "user {} has zero shares but non-zero balance {} at step {}", user_idx, balance, step);
        }
    }

    // Final invariant check
    assert_vault_invariants(&client, &users);

    // Attempt to withdraw everything from all users
    for user in &users {
        let balance = client.get_balance(user);
        if balance > 0 {
            client.withdraw(user, &balance);
        }
        assert_eq!(client.get_balance(user), 0);
        assert_eq!(client.get_shares(user), 0);
    }

    // After full withdrawal, idle should be 0, deployed may still have assets
    // Total assets should be >= 0
    assert!(client.get_total_assets() >= 0);
    // If all shares are zero, the vault may still have residual assets from deployed
    // positions — that's expected.
}
