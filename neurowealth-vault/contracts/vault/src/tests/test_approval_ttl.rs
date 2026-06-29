//! Tests for configurable Blend token approval TTL.

use super::utils::*;
use crate::DEFAULT_APPROVAL_TTL;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal,
};

fn setup_blend_position(
    env: &Env,
    ttl: Option<u32>,
) -> (
    Address,
    Address,
    Address,
    NeuroWealthVaultClient<'_>,
    TestTokenClient<'_>,
) {
    env.mock_all_auths();

    let (contract_id, _agent, owner, usdc_token, blend_pool) =
        setup_vault_with_token_and_blend(env);
    let client = NeuroWealthVaultClient::new(env, &contract_id);
    let token_client = TestTokenClient::new(env, &usdc_token);

    client.set_blend_pool(&owner, &blend_pool);
    if let Some(ttl) = ttl {
        client.set_approval_ttl(&ttl);
    }

    let user = Address::generate(env);
    mint_and_deposit(env, &client, &usdc_token, &user, 10_000_000_i128);

    (contract_id, usdc_token, blend_pool, client, token_client)
}

/// Sets up a vault with a configured DEX pool, a funded user deposit, and an
/// optional approval TTL — the DEX analogue of [`setup_blend_position`].
fn setup_dex_position(
    env: &Env,
    ttl: Option<u32>,
) -> (
    Address,
    Address,
    Address,
    NeuroWealthVaultClient<'_>,
    TestTokenClient<'_>,
) {
    env.mock_all_auths();

    let (contract_id, _agent, owner, usdc_token, dex_pool) = setup_vault_with_token_and_dex(env);
    let client = NeuroWealthVaultClient::new(env, &contract_id);
    let token_client = TestTokenClient::new(env, &usdc_token);

    client.set_dex_pool(&owner, &dex_pool);
    if let Some(ttl) = ttl {
        client.set_approval_ttl(&ttl);
    }

    let user = Address::generate(env);
    mint_and_deposit(env, &client, &usdc_token, &user, 10_000_000_i128);

    (contract_id, usdc_token, dex_pool, client, token_client)
}

#[test]
fn test_approval_expiry_uses_configured_ttl() {
    let env = Env::default();
    let configured_ttl = 2_500_u32;
    let (contract_id, _usdc_token, blend_pool, client, token_client) =
        setup_blend_position(&env, Some(configured_ttl));

    let sequence = env.ledger().sequence();
    client.rebalance(&symbol_short!("blend"), &700_i128, &0_i128);

    let expiration = token_client.allowance_expiration(&contract_id, &blend_pool);
    assert_eq!(expiration, sequence + configured_ttl);
}

#[test]
fn test_approval_expiry_minimum_ttl_is_valid() {
    let env = Env::default();
    let minimum_ttl = 1_000_u32;
    let (contract_id, _usdc_token, blend_pool, client, token_client) =
        setup_blend_position(&env, Some(minimum_ttl));

    let sequence = env.ledger().sequence();
    client.rebalance(&symbol_short!("blend"), &700_i128, &0_i128);

    let expiration = token_client.allowance_expiration(&contract_id, &blend_pool);
    assert_eq!(expiration, sequence + minimum_ttl);
    assert!(expiration > sequence);
}

#[test]
fn test_set_approval_ttl_requires_owner_auth() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner) = setup_vault(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let attacker = Address::generate(&env);
    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "set_approval_ttl",
            args: (2_000_u32,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = client.try_set_approval_ttl(&2_000_u32);
    assert!(
        result.is_err(),
        "set_approval_ttl must fail without the owner's authorization"
    );
}

#[test]
fn test_default_approval_ttl_used_when_unconfigured() {
    let env = Env::default();
    let (contract_id, _usdc_token, blend_pool, client, token_client) =
        setup_blend_position(&env, None);

    assert_eq!(client.get_approval_ttl(), DEFAULT_APPROVAL_TTL);

    let sequence = env.ledger().sequence();
    client.rebalance(&symbol_short!("blend"), &700_i128, &0_i128);

    let expiration = token_client.allowance_expiration(&contract_id, &blend_pool);
    assert_eq!(expiration, sequence + DEFAULT_APPROVAL_TTL);
}

#[test]
fn test_set_approval_ttl_rejects_below_minimum() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner) = setup_vault(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let result = client.try_set_approval_ttl(&999_u32);
    assert!(
        result.is_err(),
        "set_approval_ttl should reject TTL below minimum"
    );
}

#[test]
fn test_set_approval_ttl_rejects_above_maximum() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner) = setup_vault(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let result = client.try_set_approval_ttl(&500_001_u32);
    assert!(
        result.is_err(),
        "set_approval_ttl should reject TTL above maximum"
    );
}

// ─── DEX supply path (#341) ─────────────────────────────────────────────────
//
// The DEX supply path (`rebalance("dex", ..)` → `add_liquidity`) approves the
// pool to spend USDC using the same configurable approval TTL as Blend. These
// tests mirror the Blend coverage above for the DEX flow.

#[test]
fn test_dex_approval_expiry_uses_configured_ttl() {
    let env = Env::default();
    let configured_ttl = 2_500_u32;
    let (contract_id, _usdc_token, dex_pool, client, token_client) =
        setup_dex_position(&env, Some(configured_ttl));

    let sequence = env.ledger().sequence();
    client.rebalance(&symbol_short!("dex"), &700_i128, &0_i128);

    let expiration = token_client.allowance_expiration(&contract_id, &dex_pool);
    assert_eq!(expiration, sequence + configured_ttl);
}

#[test]
fn test_dex_approval_expiry_minimum_ttl_is_valid() {
    let env = Env::default();
    let minimum_ttl = 1_000_u32;
    let (contract_id, _usdc_token, dex_pool, client, token_client) =
        setup_dex_position(&env, Some(minimum_ttl));

    let sequence = env.ledger().sequence();
    client.rebalance(&symbol_short!("dex"), &700_i128, &0_i128);

    let expiration = token_client.allowance_expiration(&contract_id, &dex_pool);
    assert_eq!(expiration, sequence + minimum_ttl);
    assert!(expiration > sequence);
}

#[test]
fn test_default_approval_ttl_used_for_dex_supply() {
    let env = Env::default();
    let (contract_id, _usdc_token, dex_pool, client, token_client) =
        setup_dex_position(&env, None);

    assert_eq!(client.get_approval_ttl(), DEFAULT_APPROVAL_TTL);

    let sequence = env.ledger().sequence();
    client.rebalance(&symbol_short!("dex"), &700_i128, &0_i128);

    let expiration = token_client.allowance_expiration(&contract_id, &dex_pool);
    assert_eq!(expiration, sequence + DEFAULT_APPROVAL_TTL);
}

#[test]
fn test_dex_position_set_approval_ttl_rejects_below_minimum() {
    let env = Env::default();
    let (_contract_id, _usdc_token, _dex_pool, client, _token_client) =
        setup_dex_position(&env, None);

    let result = client.try_set_approval_ttl(&999_u32);
    assert!(
        result.is_err(),
        "set_approval_ttl should reject TTL below minimum on the DEX path"
    );
}

#[test]
fn test_dex_position_set_approval_ttl_rejects_above_maximum() {
    let env = Env::default();
    let (_contract_id, _usdc_token, _dex_pool, client, _token_client) =
        setup_dex_position(&env, None);

    let result = client.try_set_approval_ttl(&500_001_u32);
    assert!(
        result.is_err(),
        "set_approval_ttl should reject TTL above maximum on the DEX path"
    );
}
