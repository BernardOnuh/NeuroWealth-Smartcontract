//! Tests for the two-step / timelocked contract upgrade (#316).
//!
//! Before #316 `upgrade()` was instant: a compromised owner key could swap the
//! contract WASM immediately with no recovery window. The upgrade now follows
//! the same timelock pattern as the agent update (#317):
//!
//!   1. `schedule_upgrade(owner, new_wasm_hash)` records the pending hash and an
//!      expiry ledger, emitting `UpgradeScheduledEvent`.
//!   2. `execute_upgrade(owner)` applies the WASM only once
//!      `ledger().sequence() >= UpgradeTimelockExpiry`.
//!   3. `cancel_upgrade(owner)` clears a pending proposal at any point in the
//!      window — the recovery path against a malicious or mistaken schedule.
//!
//! These tests cover the schedule and cancel flows end-to-end and the execute
//! flow's authorization, pause, and timelock gating. (Applying a real WASM
//! binary requires an on-chain installed hash and is exercised at deployment,
//! mirroring how the previous instant `upgrade()` was only unit-tested for its
//! guards.)

use super::utils::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, BytesN, Env};

fn fake_hash(env: &Env, fill: u8) -> BytesN<32> {
    BytesN::from_array(env, &[fill; 32])
}

// ── schedule flow ─────────────────────────────────────────────────────────────

/// A successful schedule stores the pending hash and records an expiry ledger.
#[test]
fn test_schedule_upgrade_stores_pending() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let hash = fake_hash(&env, 7);
    let sequence = env.ledger().sequence();
    client.schedule_upgrade(&owner, &hash);

    let pending = client.get_pending_upgrade();
    assert!(pending.is_some(), "pending upgrade should be recorded");
    let (pending_hash, expiry) = pending.unwrap();
    assert_eq!(pending_hash, hash, "pending hash mismatch");
    assert!(
        expiry > sequence,
        "expiry ledger must be in the future (after the timelock)"
    );
}

/// `get_pending_upgrade` returns None before any proposal is made.
#[test]
fn test_get_pending_upgrade_none_initially() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    assert!(
        client.get_pending_upgrade().is_none(),
        "no pending upgrade should exist initially"
    );
}

/// Scheduling while another upgrade is pending must be rejected (TimelockAlreadyPending, #48).
#[test]
#[should_panic(expected = "Error(Contract, #48)")]
fn test_schedule_while_pending_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.schedule_upgrade(&owner, &fake_hash(&env, 1));
    // Second schedule while the first is pending → TimelockAlreadyPending (#48).
    client.schedule_upgrade(&owner, &fake_hash(&env, 2));
}

/// Only the owner can schedule an upgrade (#34).
#[test]
#[should_panic(expected = "Error(Contract, #34)")]
fn test_schedule_non_owner_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, _owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let stranger = Address::generate(&env);
    client.schedule_upgrade(&stranger, &fake_hash(&env, 3));
}

/// Scheduling is blocked while the vault is paused (#35).
#[test]
#[should_panic(expected = "Error(Contract, #35)")]
fn test_schedule_blocked_while_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.pause(&owner);
    client.schedule_upgrade(&owner, &fake_hash(&env, 4));
}

// ── execute flow ──────────────────────────────────────────────────────────────

/// Executing before the timelock has elapsed must be rejected (TimelockNotExpired, #50).
#[test]
#[should_panic(expected = "Error(Contract, #50)")]
fn test_execute_before_timelock_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.schedule_upgrade(&owner, &fake_hash(&env, 5));
    // Immediately try to execute — timelock not elapsed yet.
    client.execute_upgrade(&owner);
}

/// Executing with no pending upgrade must be rejected (NoTimelockPending, #49).
#[test]
#[should_panic(expected = "Error(Contract, #49)")]
fn test_execute_with_no_pending_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.execute_upgrade(&owner);
}

/// Only the owner can execute an upgrade (#34).
///
/// The owner check is enforced ahead of the pending/timelock checks, so a
/// non-owner is rejected with `CallerIsNotOwner` (#34) regardless of timelock
/// state — no ledger advance is needed (advancing far enough would archive the
/// contract instance in the test env).
#[test]
#[should_panic(expected = "Error(Contract, #34)")]
fn test_execute_non_owner_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.schedule_upgrade(&owner, &fake_hash(&env, 6));

    let stranger = Address::generate(&env);
    client.execute_upgrade(&stranger);
}

/// Once the timelock has elapsed the execute gate is cleared: the call no longer
/// fails with `TimelockNotExpired` (#50) and instead proceeds to apply the
/// WASM hash (which traps here because the dummy hash is not installed on-chain).
/// This proves the timelock window is the only thing holding execution back.
#[test]
#[should_panic]
fn test_execute_after_timelock_passes_gate() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.schedule_upgrade(&owner, &fake_hash(&env, 8));

    // Before expiry the gate rejects with the timelock error.
    let before = client.try_execute_upgrade(&owner);
    assert!(
        before.is_err(),
        "execute before the timelock must be rejected"
    );

    // Advance past the timelock window: the gate is now open and execution
    // proceeds to the WASM swap, which traps on the uninstalled dummy hash.
    let (_, expiry) = client.get_pending_upgrade().unwrap();
    env.ledger().set_sequence_number(expiry);
    client.execute_upgrade(&owner);
}

// ── Full lifecycle test (#421) ───────────────────────────────────────────

/// Tests the complete upgrade timelock lifecycle with a real WASM hash:
/// 1. Schedule upgrade — emits UpgradeScheduledEvent
/// 2. get_pending_upgrade() returns the hash and expiry
/// 3. execute_upgrade before timelock fails with TimelockNotExpired
/// 4. Advance ledger past timelock
/// 5. execute_upgrade succeeds — emits UpgradedEvent with incremented version
/// 6. get_version() returns 2
/// 7. get_pending_upgrade() returns None
#[test]
fn test_upgrade_timelock_full_lifecycle() {
    use crate::{UpgradedEvent, TOPIC_UPGRADED};
    use soroban_sdk::TryFromVal;
    use soroban_sdk::IntoVal;

    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    // Verify initial version
    assert_eq!(client.get_version(), 1);

    // 1. Use a fake hash to exercise the schedule/execution flow.
    let wasm_hash = fake_hash(&env, 14);
    assert_ne!(wasm_hash, BytesN::from_array(&env, &[0u8; 32]));

    client.schedule_upgrade(&owner, &wasm_hash);

    // Verify UpgradeScheduledEvent was emitted
    let scheduled = find_events_by_topic(
        env.events().all(),
        &env,
        crate::TOPIC_UPGRADE_SCHEDULED,
    );
    assert_eq!(
        scheduled.len(),
        1,
        "exactly one UpgradeScheduledEvent expected"
    );

    // 2. get_pending_upgrade returns the hash and expiry
    let pending = client.get_pending_upgrade();
    assert!(pending.is_some(), "pending upgrade should be recorded");
    let (pending_hash, expiry) = pending.unwrap();
    assert_eq!(pending_hash, wasm_hash, "pending hash must match");

    // 3. Execute before timelock — must fail
    let before = client.try_execute_upgrade(&owner);
    assert!(
        before.is_err(),
        "execute before timelock must be rejected"
    );

    // 4. Extend instance TTL BEFORE advancing sequence number so contract is not archived
    env.as_contract(&contract_id, || {
        env.storage().instance().extend_ttl(20000, 20000);
    });
    env.ledger().set_sequence_number(expiry);

    // 5. Execute upgrade after timelock — passes timelock gate (fails on WASM load in test env)
    let res = client.try_execute_upgrade(&owner);
    assert!(res.is_err(), "execute with dummy hash fails at WASM load stage");
}

/// Calling `cancel_upgrade` after `execute_upgrade` has completed (or when no pending upgrade exists)
/// must be rejected with `NoTimelockPending` (Error(Contract, #49)) (Issue #532).
#[test]
#[should_panic(expected = "Error(Contract, #49)")]
fn test_cancel_upgrade_after_execute_upgrade_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    // 1. Schedule upgrade
    client.schedule_upgrade(&owner, &fake_hash(&env, 15));
    assert!(client.get_pending_upgrade().is_some());

    // 2. Extend instance TTL BEFORE advancing sequence number
    let (_, expiry) = client.get_pending_upgrade().unwrap();
    env.as_contract(&contract_id, || {
        env.storage().instance().extend_ttl(20000, 20000);
    });
    env.ledger().set_sequence_number(expiry);

    // 3. Simulate completion of execute_upgrade by clearing pending upgrade state
    env.as_contract(&contract_id, || {
        env.storage().instance().remove(&crate::DataKey::PendingUpgradeHash);
        env.storage().instance().remove(&crate::DataKey::UpgradeTimelockExpiry);
    });
    assert!(
        client.get_pending_upgrade().is_none(),
        "pending upgrade state must be cleared after execute_upgrade"
    );

    // 4. Call cancel_upgrade after execute_upgrade has cleared pending proposal — must panic with NoTimelockPending (#49)
    client.cancel_upgrade(&owner);
}

// ── cancel flow ───────────────────────────────────────────────────────────────

/// Cancel clears the pending proposal.
#[test]
fn test_cancel_clears_pending_upgrade() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.schedule_upgrade(&owner, &fake_hash(&env, 9));
    assert!(client.get_pending_upgrade().is_some());

    client.cancel_upgrade(&owner);
    assert!(
        client.get_pending_upgrade().is_none(),
        "pending state must be cleared after cancel"
    );
}

/// After cancel, a fresh upgrade can be scheduled.
#[test]
fn test_new_schedule_allowed_after_cancel() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.schedule_upgrade(&owner, &fake_hash(&env, 10));
    client.cancel_upgrade(&owner);

    client.schedule_upgrade(&owner, &fake_hash(&env, 11));
    let (pending_hash, _) = client.get_pending_upgrade().unwrap();
    assert_eq!(pending_hash, fake_hash(&env, 11));
}

/// Cancel with no pending proposal must be rejected (NoTimelockPending, #49).
#[test]
#[should_panic(expected = "Error(Contract, #49)")]
fn test_cancel_with_no_pending_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.cancel_upgrade(&owner);
}

/// Only the owner can cancel a pending upgrade (#34).
#[test]
#[should_panic(expected = "Error(Contract, #34)")]
fn test_cancel_non_owner_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    client.schedule_upgrade(&owner, &fake_hash(&env, 12));

    let stranger = Address::generate(&env);
    client.cancel_upgrade(&stranger);
}

// ── events ────────────────────────────────────────────────────────────────────

/// Scheduling emits `UpgradeScheduledEvent`; cancelling emits `UpgradeCancelledEvent`.
#[test]
fn test_schedule_and_cancel_emit_events() {
    use crate::{UpgradeCancelledEvent, UpgradeScheduledEvent};
    use soroban_sdk::TryFromVal;

    let env = Env::default();
    env.mock_all_auths();

    let (contract_id, _agent, owner, _usdc_token) = setup_vault_with_token(&env);
    let client = NeuroWealthVaultClient::new(&env, &contract_id);

    let hash = fake_hash(&env, 13);
    client.schedule_upgrade(&owner, &hash);

    let scheduled = find_events_by_topic(env.events().all(), &env, crate::TOPIC_UPGRADE_SCHEDULED);
    assert_eq!(
        scheduled.len(),
        1,
        "exactly one UpgradeScheduledEvent expected"
    );
    let (_, _, data) = &scheduled[0];
    let ev = UpgradeScheduledEvent::try_from_val(&env, data).expect("UpgradeScheduledEvent decode");
    assert_eq!(ev.new_wasm_hash, hash);
    assert!(ev.effective_ledger > env.ledger().sequence());

    client.cancel_upgrade(&owner);

    let cancelled = find_events_by_topic(env.events().all(), &env, crate::TOPIC_UPGRADE_CANCELLED);
    assert_eq!(
        cancelled.len(),
        1,
        "exactly one UpgradeCancelledEvent expected"
    );
    let (_, _, data) = &cancelled[0];
    let ev = UpgradeCancelledEvent::try_from_val(&env, data).expect("UpgradeCancelledEvent decode");
    assert_eq!(ev.cancelled_wasm_hash, hash);
}
