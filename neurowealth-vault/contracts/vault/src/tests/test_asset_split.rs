//! Tests for idle vs deployed asset tracking getters:
//! `get_idle_balance`, `get_deployed_assets`, and `get_asset_breakdown`.

use super::utils::*;
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};

// ============================================================================
// get_idle_balance
// ============================================================================

/// Before any rebalance the vault holds all deposited USDC directly, so
/// `get_idle_balance` must equal the deposited amount.
#[test]
fn test_get_idle_balance_before_rebalance_equals_deposit() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, usdc_token, _blend_pool) =
        setup_vault_with_token_and_blend(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let deposit_amount = 10_000_000_i128; // 1 USDC (7 decimals)
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);

    assert_eq!(
        client.get_idle_balance(),
        deposit_amount,
        "idle balance should equal deposited amount before rebalance"
    );
}

/// Before any rebalance no funds have been supplied to a protocol, so
/// `get_deployed_assets` must be 0.
#[test]
fn test_get_deployed_assets_before_rebalance_is_zero() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, usdc_token, _blend_pool) =
        setup_vault_with_token_and_blend(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let deposit_amount = 10_000_000_i128;
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);

    assert_eq!(
        client.get_deployed_assets(),
        0_i128,
        "deployed assets should be 0 before any rebalance"
    );
}

// ============================================================================
// get_deployed_assets and get_idle_balance after rebalance → blend
// ============================================================================

/// After rebalancing to Blend all vault USDC is supplied to the pool, so:
/// - `get_idle_balance` should drop to 0 (vault holds nothing locally).
/// - `get_deployed_assets` should be > 0 (funds are in the pool).
#[test]
fn test_after_rebalance_to_blend_deployed_grows_idle_shrinks() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, usdc_token, blend_pool) =
        setup_vault_with_token_and_blend(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    // Configure the Blend pool so the vault can rebalance into it.
    client.set_blend_pool(&owner, &blend_pool);

    let user = Address::generate(&env);
    let deposit_amount = 10_000_000_i128;
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);

    // Sanity-check pre-conditions.
    assert_eq!(client.get_idle_balance(), deposit_amount);
    assert_eq!(client.get_deployed_assets(), 0_i128);

    // Rebalance: all vault USDC is transferred to MockBlendPool.
    client.rebalance(&symbol_short!("blend"), &500_i128, &0_i128);

    // After rebalance the vault should hold no USDC locally.
    assert_eq!(
        client.get_idle_balance(),
        0_i128,
        "vault should hold no idle USDC after full rebalance to Blend"
    );

    // The Blend pool should now hold all the deposited funds.
    assert!(
        client.get_deployed_assets() > 0_i128,
        "deployed assets should be positive after rebalance to Blend"
    );
    assert_eq!(
        client.get_deployed_assets(),
        deposit_amount,
        "deployed assets should equal the original deposit after full supply"
    );
}

// ============================================================================
// get_asset_breakdown
// ============================================================================

/// `get_asset_breakdown` must return the same values as calling
/// `get_idle_balance` and `get_deployed_assets` individually.
#[test]
fn test_get_asset_breakdown_matches_individual_getters() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, usdc_token, blend_pool) =
        setup_vault_with_token_and_blend(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.set_blend_pool(&owner, &blend_pool);

    let user = Address::generate(&env);
    let deposit_amount = 10_000_000_i128;
    mint_and_deposit(&env, &client, &usdc_token, &user, deposit_amount);

    // Check breakdown before rebalance.
    let (idle_before, deployed_before) = client.get_asset_breakdown();
    assert_eq!(
        idle_before,
        client.get_idle_balance(),
        "breakdown.idle should match get_idle_balance before rebalance"
    );
    assert_eq!(
        deployed_before,
        client.get_deployed_assets(),
        "breakdown.deployed should match get_deployed_assets before rebalance"
    );

    // Rebalance to Blend.
    client.rebalance(&symbol_short!("blend"), &500_i128, &0_i128);

    // Check breakdown after rebalance.
    let (idle_after, deployed_after) = client.get_asset_breakdown();
    assert_eq!(
        idle_after,
        client.get_idle_balance(),
        "breakdown.idle should match get_idle_balance after rebalance"
    );
    assert_eq!(
        deployed_after,
        client.get_deployed_assets(),
        "breakdown.deployed should match get_deployed_assets after rebalance"
    );
    assert_eq!(
        idle_after, 0_i128,
        "idle should be 0 after full supply to Blend"
    );
    assert_eq!(
        deployed_after, deposit_amount,
        "deployed should equal deposited amount after full supply to Blend"
    );
}
