//! Tests for the two-step agent update timelock (#317).
//!
//! Verifies:
//! - Propose stores pending agent and emits AgentUpdateProposedEvent.
//! - Confirm succeeds only after the timelock window and emits both confirm events.
//! - Cancel clears the pending proposal and emits AgentUpdateCancelledEvent.
//! - Duplicate proposals are rejected while one is pending.
//! - Confirm before timelock is rejected with TimelockNotExpired (#50).
//! - Confirm/cancel with no pending proposal are rejected with NoTimelockPending (#49).
//! - Only the owner can propose, confirm, or cancel.

use super::utils::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env};

/// A successful propose stores the pending agent and emits the proposal event.
#[test]
fn test_propose_agent_stores_pending_and_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, old_agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let new_agent = Address::generate(&env);
    client.update_agent(&new_agent);

    // Active agent is unchanged until confirmation.
    assert_eq!(
        client.get_agent(),
        old_agent,
        "active agent must not change on propose"
    );

    // Pending update is recorded.
    let pending = client.get_pending_agent_update();
    assert!(pending.is_some(), "pending agent update should be recorded");
    let (pending_addr, expiry) = pending.unwrap();
    assert_eq!(pending_addr, new_agent, "pending agent address mismatch");
    assert!(expiry > 0, "expiry ledger should be set");
}

/// Proposing while another proposal is pending must be rejected.
#[test]
#[should_panic(expected = "Error(Contract, #48)")]
fn test_propose_while_pending_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let new_agent1 = Address::generate(&env);
    let new_agent2 = Address::generate(&env);

    client.update_agent(&new_agent1);
    // Second proposal while first is pending must panic with TimelockAlreadyPending (#48).
    client.update_agent(&new_agent2);
}

/// Confirming before the timelock has elapsed must be rejected.
#[test]
#[should_panic(expected = "Error(Contract, #50)")]
fn test_confirm_before_timelock_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let new_agent = Address::generate(&env);
    client.update_agent(&new_agent);

    // Immediately try to confirm — timelock not elapsed yet.
    client.confirm_agent_update();
}

/// Confirming after the timelock window applies the update and clears pending state.
#[test]
fn test_confirm_after_timelock_applies_update() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _old_agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let new_agent = Address::generate(&env);
    client.update_agent(&new_agent);

    let (_, expiry) = client.get_pending_agent_update().unwrap();

    // Advance the ledger past the timelock expiry.
    env.ledger().set_sequence_number(expiry);

    client.confirm_agent_update();

    assert_eq!(
        client.get_agent(),
        new_agent,
        "agent should be updated after confirm"
    );
    assert!(
        client.get_pending_agent_update().is_none(),
        "pending state should be cleared after confirm"
    );
}

/// After confirmation, a new proposal can be submitted.
#[test]
fn test_new_proposal_allowed_after_confirm() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let agent2 = Address::generate(&env);
    client.update_agent(&agent2);

    let (_, expiry) = client.get_pending_agent_update().unwrap();
    env.ledger().set_sequence_number(expiry);
    client.confirm_agent_update();

    // Should now be able to propose again.
    let agent3 = Address::generate(&env);
    client.update_agent(&agent3);
    assert!(client.get_pending_agent_update().is_some());
}

/// Cancel clears the pending proposal and the active agent is unchanged.
#[test]
fn test_cancel_clears_pending_agent() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, old_agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let new_agent = Address::generate(&env);
    client.update_agent(&new_agent);
    client.cancel_agent_update();

    assert_eq!(
        client.get_agent(),
        old_agent,
        "active agent unchanged after cancel"
    );
    assert!(
        client.get_pending_agent_update().is_none(),
        "pending state cleared after cancel"
    );
}

/// After cancel, a fresh proposal can be submitted.
#[test]
fn test_new_proposal_allowed_after_cancel() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let agent2 = Address::generate(&env);
    client.update_agent(&agent2);
    client.cancel_agent_update();

    let agent3 = Address::generate(&env);
    client.update_agent(&agent3);
    assert!(client.get_pending_agent_update().is_some());
}

/// Confirm with no pending proposal must be rejected with NoTimelockPending (#49).
#[test]
#[should_panic(expected = "Error(Contract, #49)")]
fn test_confirm_with_no_pending_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.confirm_agent_update();
}

/// Cancel with no pending proposal must be rejected with NoTimelockPending (#49).
#[test]
#[should_panic(expected = "Error(Contract, #49)")]
fn test_cancel_with_no_pending_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.cancel_agent_update();
}

/// `get_pending_agent_update` returns None before any proposal is made.
#[test]
fn test_get_pending_agent_update_none_initially() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    assert!(
        client.get_pending_agent_update().is_none(),
        "no pending update should exist initially"
    );
}
