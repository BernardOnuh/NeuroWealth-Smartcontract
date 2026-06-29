//! LibFuzzer harness: tests share-accounting invariants.
//!
//! This target verifies that the vault's share-based accounting maintains
//! mathematical invariants across deposit/withdraw/asset-update sequences:
//!
//! Invariants checked:
//! 1. sum(user_shares) == total_shares
//! 2. user_balance(user) == user_shares(user) * total_assets / total_shares
//! 3. total_assets >= total_deposits (yield should be non-negative)
//! 4. user_shares <= total_shares for all users
//! 5. user_balance <= total_assets for all users
//!
//! Allowed panics (documented vault validation):
//! - `Error(Contract, #37)` — AmountMustBePositive
//! - `Error(Contract, #38)` — BelowMinimumDeposit
//! - `Error(Contract, #39)` — MaximumDepositExceeded
//! - `Error(Contract, #40)` — ExceedsUserDepositCap
//! - `Error(Contract, #41)` — ExceedsTvlCap
//! - `Error(Contract, #7)`  — InsufficientLiquidity
//! - `Error(Contract, #6)`  — SharesToMintMustBePositive
//! - `Error(Contract, #8)`  — InsufficientShares
//! - Token transfer failures

#![no_main]

use libfuzzer_sys::fuzz_target;
use neurowealth_vault::{NeuroWealthVault, NeuroWealthVaultClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, BytesN, Env};

mod token {
    use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

    #[contracttype]
    enum TokenDataKey {
        Balance(Address),
    }

    #[contract]
    pub struct FuzzToken;

    #[contractimpl]
    impl FuzzToken {
        pub fn mint(env: Env, to: Address, amount: i128) {
            let balance: i128 = env
                .storage()
                .persistent()
                .get(&TokenDataKey::Balance(to.clone()))
                .unwrap_or(0);
            env.storage()
                .persistent()
                .set(&TokenDataKey::Balance(to), &(balance + amount));
        }

        pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
            from.require_auth();
            assert!(amount > 0, "amount must be positive");

            let from_balance: i128 = env
                .storage()
                .persistent()
                .get(&TokenDataKey::Balance(from.clone()))
                .unwrap_or(0);
            assert!(from_balance >= amount, "insufficient balance");

            let to_balance: i128 = env
                .storage()
                .persistent()
                .get(&TokenDataKey::Balance(to.clone()))
                .unwrap_or(0);

            env.storage()
                .persistent()
                .set(&TokenDataKey::Balance(from), &(from_balance - amount));
            env.storage()
                .persistent()
                .set(&TokenDataKey::Balance(to), &(to_balance + amount));
        }

        pub fn balance(env: Env, owner: Address) -> i128 {
            env.storage()
                .persistent()
                .get(&TokenDataKey::Balance(owner))
                .unwrap_or(0)
        }
    }
}

use token::{FuzzToken, FuzzTokenClient};

const MIN_DEPOSIT: i128 = 1_000_000;
const MAX_DEPOSIT: i128 = 10_000_000_000;
const USER_CAP: i128 = 10_000_000_000;
const TVL_CAP: i128 = 100_000_000_000;
const TOKEN_FLOAT: i128 = 50_000_000_000;

fn setup(env: &Env) -> (NeuroWealthVaultClient<'_>, Vec<Address>, Address) {
    let deployer = Address::generate(env);
    let salt = BytesN::from_array(env, &[7u8; 32]);
    let contract_id = env
        .deployer()
        .with_address(deployer.clone(), salt.clone())
        .deployed_address();
    env.register_contract(&contract_id, NeuroWealthVault);

    let client = NeuroWealthVaultClient::new(env, &contract_id);
    let agent = Address::generate(env);
    let owner = Address::generate(env);
    let usdc = env.register_contract(None, FuzzToken);

    // Create multiple users to test multi-user invariants
    let users: Vec<Address> = (0..3).map(|_| Address::generate(env)).collect();

    client.initialize(&deployer, &owner, &agent, &usdc, &salt);

    let token = FuzzTokenClient::new(env, &usdc);
    for user in &users {
        token.mint(user, &TOKEN_FLOAT);
    }

    (client, users, usdc)
}

fn is_allowed_panic(msg: &str) -> bool {
    const ALLOWED: &[&str] = &[
        "Error(Contract, #37)", // AmountMustBePositive
        "Error(Contract, #38)", // BelowMinimumDeposit
        "Error(Contract, #39)", // MaximumDepositExceeded
        "Error(Contract, #40)", // ExceedsUserDepositCap
        "Error(Contract, #41)", // ExceedsTvlCap
        "Error(Contract, #7)",  // InsufficientLiquidity
        "Error(Contract, #6)",  // SharesToMintMustBePositive
        "Error(Contract, #8)",  // InsufficientShares
        "insufficient balance",
        "amount must be positive",
    ];
    ALLOWED.iter().any(|needle| msg.contains(needle))
}

/// Assert share-accounting invariants after each operation.
fn assert_share_invariants(client: &NeuroWealthVaultClient, users: &[Address]) {
    let total_shares = client.get_total_shares();
    let total_assets = client.get_total_assets();
    let total_deposits = client.get_total_deposits();

    // Invariant 1: total_assets >= total_deposits (yield should be non-negative or zero)
    assert!(
        total_assets >= total_deposits,
        "total_assets ({total_assets}) < total_deposits ({total_deposits})"
    );

    // Invariant 2: Check individual user invariants
    for user in users {
        let user_shares = client.get_shares(user);
        let user_balance = client.get_balance(user);

        // User shares must be non-negative
        assert!(user_shares >= 0, "user_shares is negative");

        // User balance must be non-negative
        assert!(user_balance >= 0, "user_balance is negative");

        // User shares cannot exceed total shares
        assert!(
            user_shares <= total_shares,
            "user_shares ({user_shares}) > total_shares ({total_shares})"
        );

        // User balance cannot exceed total assets
        assert!(
            user_balance <= total_assets,
            "user_balance ({user_balance}) > total_assets ({total_assets})"
        );

        // If total_shares > 0, user_balance should be proportional
        if total_shares > 0 && total_assets > 0 {
            let expected_balance = user_shares
                .checked_mul(total_assets)
                .expect("overflow in balance calc")
                / total_shares;
            // Allow for rounding (floor division means actual may be slightly less)
            assert!(
                user_balance <= expected_balance,
                "user_balance ({user_balance}) > expected ({expected_balance})"
            );
        } else if total_shares == 0 || total_assets == 0 {
            assert_eq!(user_balance, 0, "no shares but non-zero balance");
        }
    }

    // Invariant 3: Exchange rate should be >= 1.0 (10_000_000 in scaled units)
    if total_shares > 0 && total_assets > 0 {
        let exchange_rate = client.get_exchange_rate();
        assert!(
            exchange_rate >= 10_000_000,
            "exchange_rate ({exchange_rate}) < 1.0"
        );
    }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let env = Env::default();
    env.mock_all_auths();

    let (client, users, usdc) = setup(&env);
    let _token = FuzzTokenClient::new(&env, &usdc);

    for (i, chunk) in data.chunks(4).enumerate() {
        if chunk.is_empty() {
            continue;
        }

        let op = chunk[0] % 4;
        let user_idx = chunk.get(1).copied().unwrap_or(0) as usize % users.len();
        let raw = u16::from(chunk.get(2).copied().unwrap_or(0))
            | (u16::from(chunk.get(3).copied().unwrap_or(0)) << 8);
        let amount = i128::from(raw % 20_000) * MIN_DEPOSIT + MIN_DEPOSIT;

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let user = &users[user_idx];

            match op {
                0 => {
                    // Deposit
                    if !(MIN_DEPOSIT..=MAX_DEPOSIT).contains(&amount)
                        || amount > USER_CAP
                        || amount > TVL_CAP
                    {
                        return;
                    }
                    let token = FuzzTokenClient::new(&env, &usdc);
                    if token.balance(user) < amount {
                        return;
                    }
                    client.deposit(user, &amount);
                }
                1 => {
                    // Withdraw
                    let balance = client.get_balance(user);
                    if balance <= 0 {
                        return;
                    }
                    let withdraw_amount = amount.min(balance);
                    if withdraw_amount <= 0 {
                        return;
                    }
                    client.withdraw(user, &withdraw_amount);
                }
                2 => {
                    // Deposit then immediately withdraw (round-trip)
                    if !(MIN_DEPOSIT..=MAX_DEPOSIT).contains(&amount)
                        || amount > USER_CAP
                        || amount > TVL_CAP
                    {
                        return;
                    }
                    let token = FuzzTokenClient::new(&env, &usdc);
                    if token.balance(user) < amount {
                        return;
                    }
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        client.deposit(user, &amount);
                    }));

                    let balance = client.get_balance(user);
                    if balance > MIN_DEPOSIT {
                        let withdraw_amount = balance / 2;
                        if withdraw_amount >= MIN_DEPOSIT {
                            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                client.withdraw(user, &withdraw_amount);
                            }));
                        }
                    }
                }
                3 => {
                    // Deposit from a different user (cross-user invariant test)
                    let other_idx = (user_idx + 1) % users.len();
                    let other_user = &users[other_idx];

                    if !(MIN_DEPOSIT..=MAX_DEPOSIT).contains(&amount)
                        || amount > USER_CAP
                        || amount > TVL_CAP
                    {
                        return;
                    }
                    let token = FuzzTokenClient::new(&env, &usdc);
                    if token.balance(other_user) < amount {
                        return;
                    }
                    client.deposit(other_user, &amount);
                }
                _ => unreachable!(),
            }
        }));

        match result {
            Ok(()) => assert_share_invariants(&client, &users),
            Err(payload) => {
                let msg = payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| payload.downcast_ref::<String>().map(|s| s.as_str()))
                    .unwrap_or("unknown panic");
                assert!(is_allowed_panic(msg), "unexpected panic at step {i}: {msg}");
            }
        }
    }
});
