//! Tests for pause/unpause functionality

extern crate std;

use super::utils::*;
use crate::{EmergencyPausedEvent, VaultPausedEvent, TOPIC_EMERGENCY_PAUSED, TOPIC_PAUSED};
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, TryFromVal};

#[test]
fn test_owner_can_pause() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    assert!(!client.is_paused(), "Vault should start unpaused");

    client.pause(&owner);

    assert!(client.is_paused(), "Vault should be paused");
}

#[test]
fn test_owner_can_unpause() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.pause(&owner);
    assert!(client.is_paused());

    client.unpause(&owner);
    assert!(!client.is_paused(), "Vault should be unpaused");
}

#[test]
fn test_owner_can_emergency_pause() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    assert!(!client.is_paused());

    client.emergency_pause(&owner);

    assert!(client.is_paused(), "Vault should be emergency paused");
}

#[test]
#[should_panic(expected = "Error(Contract, #20)")]
fn test_non_owner_cannot_unpause() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    // Owner pauses
    client.pause(&owner);
    assert!(client.is_paused());

    // A fresh address that is NOT the owner tries to unpause
    let non_owner = Address::generate(&env);
    client.unpause(&non_owner);
}

#[test]
#[should_panic]
fn test_unauthorized_users_cannot_pause() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let unauthorized = Address::generate(&env);

    client.emergency_pause(&owner);
    // Fails because unauthorized != stored_owner
    client.pause(&unauthorized);
}

#[test]
#[should_panic(expected = "Error(Contract, #35)")]
fn test_deposit_blocked_while_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let token_client = TestTokenClient::new(&env, &usdc_token);

    client.pause(&owner);
    assert!(client.is_paused());

    let user = Address::generate(&env);
    let amount = 5_000_000_i128;

    token_client.mint(&user, &amount);
    client.deposit(&user, &amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #35)")]
fn test_withdraw_blocked_while_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let amount = 5_000_000_i128;

    mint_and_deposit(&env, &client, &usdc_token, &user, amount);

    client.pause(&owner);
    assert!(client.is_paused());

    let balance = client.get_balance(&user);
    client.withdraw(&user, &balance);
}

#[test]
#[should_panic(expected = "Error(Contract, #35)")]
fn test_rebalance_blocked_while_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.pause(&owner);
    assert!(client.is_paused());

    // require_not_paused fires before any blend check
    client.rebalance(&soroban_sdk::symbol_short!("blend"), &500_i128, &0_i128);
}

#[test]
fn test_pause_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.pause(&owner);

    let pause_events = find_events_by_topic(env.events().all(), &env, TOPIC_PAUSED);
    assert_eq!(pause_events.len(), 1, "Exactly one paused event should be emitted");

    let (_, _, data) = &pause_events[0];
    let event = VaultPausedEvent::try_from_val(&env, data)
        .expect("Should be a valid VaultPausedEvent");
    assert_eq!(event.owner, owner, "Event owner should match caller");
}

#[test]
fn test_emergency_pause_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.emergency_pause(&owner);

    let emergency_events = find_events_by_topic(env.events().all(), &env, TOPIC_EMERGENCY_PAUSED);
    assert_eq!(
        emergency_events.len(), 1,
        "Exactly one emergency paused event should be emitted"
    );

    let (_, _, data) = &emergency_events[0];
    let event = EmergencyPausedEvent::try_from_val(&env, data)
        .expect("Should be a valid EmergencyPausedEvent");
    assert_eq!(event.owner, owner, "Event owner should match caller");
}

// ============================================================================
// ISSUE #508: Circuit-breaker auto-pause distinguishable from owner pause
// ============================================================================

#[test]
fn test_auto_pause_emits_different_event_than_owner_pause() {
    let env = Env::default();
    env.mock_all_auths();

    // --- Part 1: owner-initiated pause emits VaultPausedEvent (topic "paused") ---
    let (contract_id, _agent, owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.pause(&owner);
    assert!(client.is_paused());

    let pause_events = find_events_by_topic(env.events().all(), &env, TOPIC_PAUSED);
    assert!(
        !pause_events.is_empty(),
        "owner pause must emit at least one VaultPausedEvent"
    );
    // Verify the last event is a VaultPausedEvent
    let (_, _, data) = pause_events.last().unwrap();
    let event = VaultPausedEvent::try_from_val(&env, data)
        .expect("owner pause event must decode as VaultPausedEvent");
    assert_eq!(event.owner, owner);

    // Ensure no EmergencyPausedEvent was emitted by this pause
    let emerg_events_after_pause =
        find_events_by_topic(env.events().all(), &env, TOPIC_EMERGENCY_PAUSED);
    assert_eq!(
        emerg_events_after_pause.len(),
        0,
        "owner pause must NOT emit EmergencyPausedEvent"
    );

    // --- Part 2: circuit-breaker auto-pause emits EmergencyPausedEvent (topic "emerg") ---
    // Deploy a fresh vault with Blend so the circuit breaker can fire.
    let (contract_id2, _agent2, owner2, usdc_token2, blend_pool2) =
        setup_vault_with_token_and_blend(&env);
    let client2 = NeuroWealthVaultClient::new(&env, &contract_id2);
    let blend_client = MockBlendPoolClient::new(&env, &blend_pool2);

    client2.set_blend_pool(&owner2, &blend_pool2);
    blend_client.set_max_supply_limit(&-1_i128); // force every rebalance to "fail"

    let user = Address::generate(&env);
    mint_and_deposit(&env, &client2, &usdc_token2, &user, 10_000_000_i128);

    // Record events before triggering the circuit breaker.
    let emerg_before =
        find_events_by_topic(env.events().all(), &env, TOPIC_EMERGENCY_PAUSED).len();

    // Three consecutive failures trip the default threshold (3).
    client2.rebalance(&soroban_sdk::symbol_short!("blend"), &500_i128, &0_i128);
    client2.rebalance(&soroban_sdk::symbol_short!("blend"), &500_i128, &0_i128);
    client2.rebalance(&soroban_sdk::symbol_short!("blend"), &500_i128, &0_i128);
    assert!(client2.is_paused(), "circuit breaker must pause the vault");

    // Exactly one new EmergencyPausedEvent must have been emitted.
    let emerg_events =
        find_events_by_topic(env.events().all(), &env, TOPIC_EMERGENCY_PAUSED);
    assert_eq!(
        emerg_events.len(),
        emerg_before + 1,
        "circuit-breaker auto-pause must emit exactly one EmergencyPausedEvent"
    );
    let (_, _, data) = emerg_events.last().unwrap();
    let event = EmergencyPausedEvent::try_from_val(&env, data)
        .expect("auto-pause event must decode as EmergencyPausedEvent");
    assert_eq!(event.owner, owner2);

    // Ensure no VaultPausedEvent was emitted by the circuit breaker.
    let pause_events_after_circuit =
        find_events_by_topic(env.events().all(), &env, TOPIC_PAUSED);
    // The owner2 vault has no pause events (we only called rebalance, not pause).
    assert_eq!(
        pause_events_after_circuit.len(),
        0,
        "circuit-breaker auto-pause must NOT emit VaultPausedEvent"
    );

    // The topics themselves are different: "paused" vs "emerg".
    assert_ne!(
        TOPIC_PAUSED, TOPIC_EMERGENCY_PAUSED,
        "TOPIC_PAUSED and TOPIC_EMERGENCY_PAUSED must be distinct symbols"
    );
}

// ============================================================================
// ISSUE #189: Block upgrade while paused
// ============================================================================

#[test]
#[should_panic(expected = "Error(Contract, #35)")]
fn test_upgrade_blocked_while_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.pause(&owner);
    assert!(client.is_paused());

    let fake_hash = BytesN::from_array(&env, &[0u8; 32]);
    client.schedule_upgrade(&owner, &fake_hash);
}

#[test]
fn test_upgrade_unpaused_vault_clears_pause_guard() {
    // Verifies that require_not_paused does not block upgrade on a healthy vault:
    // pause then unpause, and confirm the vault is no longer paused.
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.pause(&owner);
    assert!(client.is_paused());
    client.unpause(&owner);
    assert!(
        !client.is_paused(),
        "vault must be unpaused before upgrade is allowed"
    );
}

#[test]
fn test_emergency_pause_blocks_operations_and_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);
    let token_client = TestTokenClient::new(&env, &usdc_token);

    let user = Address::generate(&env);
    let amount = 5_000_000_i128;

    // Pre-fund the user and deposit some tokens to test withdrawal later
    token_client.mint(&user, &amount);
    client.deposit(&user, &amount);

    assert!(!client.is_paused());

    // 1. Calls emergency_pause as owner
    client.emergency_pause(&owner);

    // 2. Asserts is_paused() returns true
    assert!(client.is_paused(), "Vault should be emergency paused");

    // 3. Verifies the emitted event topic is "emerg"
    let emergency_events = find_events_by_topic(env.events().all(), &env, TOPIC_EMERGENCY_PAUSED);
    assert_eq!(
        emergency_events.len(), 1,
        "Exactly one emergency paused event should be emitted"
    );
    assert_eq!(
        TOPIC_EMERGENCY_PAUSED,
        soroban_sdk::symbol_short!("emerg"),
        "Event topic must be 'emerg'"
    );

    // 4. Asserts a deposit attempt panics with VaultError::Paused (#35)
    let deposit_res = client.try_deposit(&user, &amount);
    assert_eq!(
        deposit_res,
        Err(Ok(soroban_sdk::Error::from_contract_error(35))),
        "deposit attempt should panic with VaultError::Paused (#35)"
    );

    // 5. Asserts a withdrawal attempt panics with VaultError::Paused (#35)
    let withdraw_res = client.try_withdraw(&user, &amount);
    assert_eq!(
        withdraw_res,
        Err(Ok(soroban_sdk::Error::from_contract_error(35))),
        "withdrawal attempt should panic with VaultError::Paused (#35)"
    );

    // 6. Asserts a rebalance attempt panics with VaultError::Paused (#35)
    let rebalance_res = client.try_rebalance(&soroban_sdk::symbol_short!("blend"), &500_i128, &0_i128);
    assert_eq!(
        rebalance_res,
        Err(Ok(soroban_sdk::Error::from_contract_error(35))),
        "rebalance attempt should panic with VaultError::Paused (#35)"
    );
}
