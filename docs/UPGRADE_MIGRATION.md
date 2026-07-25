# NeuroWealth Contract Upgrade & Storage Migration Guide

This document provides comprehensive guidelines for upgrading the NeuroWealth smart contract and managing its storage schema safely. It serves as a reference for contributors and maintainers to ensure data integrity during contract evolutions.

## 1. Overview

In the Soroban smart contract environment, a contract upgrade involves replacing the underlying WebAssembly (WASM) code of a contract while its data (storage) remains attached to the same contract ID.

**What is preserved during an upgrade:**
* **Persistent Storage**: Data meant to outlive the transaction and remain available indefinitely (e.g., user balances, shares, config).
* **Temporary Storage**: Short-lived data, but still persists across the WASM swap until its TTL expires.
* **Instance Storage**: Contract-level global state (e.g., admin addresses, token IDs).
* **The Contract ID**: The address of the contract remains exactly the same.

**What is replaced during an upgrade:**
* **Contract Code (WASM)**: All logic, entrypoints, and type definitions are completely replaced by the new WASM binary.

**Upgrade vs. Migration:**
* **Code Upgrade**: Swapping the executable WASM file. If the storage schema (the structure of saved data) has not changed, an upgrade requires no further action.
* **Storage Migration**: Re-structuring the existing data stored on the ledger to match new type definitions in the upgraded code. This typically requires a dedicated migration entrypoint to transition old data formats to new ones.

---

## 2. Upgrade Safety Principles

Soroban storage keys (`DataKey`s) and values are heavily tied to their Rust serialized representations (XDR). Altering these types requires extreme care.

### Safe Changes
* Adding new functions or entrypoints.
* Adding new events or changing event structures (events are not state).
* Adding new `DataKey` variants at the *end* of the enum (does not affect existing serialized variants).
* Adding optional struct fields (if standard XDR evolution rules are strictly followed and supported).

### Risky Changes
* **Renaming `DataKey` variants**: Changes the conceptual mapping but technically doesn't break XDR if the variant index and internal types are identical. However, it requires a logical migration if the underlying intent changes.
* **Reordering enum variants**: This alters the discriminant values used in serialization, breaking access to existing storage entries.
* **Changing serialized struct layouts**: Adding or reordering fields in a stored struct breaks deserialization of existing data.
* **Changing stored value types**: E.g., changing `u32` to `u64`.

### Dangerous Changes
**Example of a catastrophic change:**
Before:
```rust
DataKey::UserBalance(Address)
```
Changed to:
```rust
DataKey::Balance(Address)
```
Without a proper migration, the contract will look for `DataKey::Balance` and find nothing, effectively zeroing out all user balances, while the old `DataKey::UserBalance` data becomes permanently orphaned in storage.

---

## 3. Storage Layout Guidelines

To maintain upgrade compatibility, adhere to the following `DataKey` design patterns:

```rust
#[contracttype]
pub enum DataKey {
    Config,
    User(Address),
    Vault(Address),
    Position(u64),
}
```

**Guidelines:**
* **Use typed `DataKey` enums**: Avoid raw `Symbol` or string keys to prevent typos and namespace collisions.
* **Keep variants stable**: Once a variant is used in production, treat it as immutable.
* **Never reorder variants**: Always append new variants to the end of the `DataKey` enum.
* **Namespace logically**: Group related data logically within the enum or nested enums to avoid top-level clutter.

---

## 4. Versioning Strategy

To safely track and orchestrate migrations, the contract must maintain a storage version in its instance or persistent storage.

```rust
pub const STORAGE_VERSION: u32 = 1;
```

When a schema change occurs, increment the version:
```rust
pub const STORAGE_VERSION: u32 = 2;
```

**Versioning Rules:**
* **When to increment**: Increment the `STORAGE_VERSION` constant anytime a structural change is made to stored structs, or when `DataKey` semantics change requiring a migration script.
* **Tracking Migrations**: Store the current migrated version on-chain.
* **Upgrade Scripts**: The migration entrypoint must verify the on-chain version against the expected old version before running, preventing double-migrations.

---

## 5. When Migrations Are Required

### New Storage Key (No Migration Required)
Example: Adding `DataKey::Treasury`.
If you are simply introducing a new key and no existing data needs to be restructured, no migration script is required. The new data will be written on demand.

### Added Struct Field (Migration Required)
Before:
```rust
pub struct Vault {
    pub balance: i128,
}
```
After:
```rust
pub struct Vault {
    pub balance: i128,
    pub reward_rate: u32,
}
```
**Why:** Existing serialized `Vault` values on the ledger lack the `reward_rate` field and cannot automatically deserialize into the new struct. A migration function must read the old bytes/struct, populate the missing field with a default, and write the new struct back.

### Key Rename / Semantic Shift (Migration Required)
Before:
```rust
DataKey::Vault(id)
```
After:
```rust
DataKey::Position(id)
```
**Why:** The data lives under the old serialized key. A migration must read the data from `DataKey::Vault(id)`, write it to `DataKey::Position(id)`, and explicitly delete the old `DataKey::Vault(id)` to free up space and recover storage deposits.

---

## 6. Example Migration Workflow

**Scenario:** Introduce `DataKey::TreasuryBalance` and migrate legacy treasury values.

**Migration Entrypoint:**
```rust
pub fn migrate(env: Env) {
    // 1. Verify admin/owner auth
    env.storage().instance().get::<_, Address>(&DataKey::Admin).unwrap().require_auth();

    // 2. Check current version to prevent double execution
    let current_version: u32 = env.storage().instance().get(&DataKey::Version).unwrap_or(1);
    assert!(current_version == 1, "Migration already executed");

    // 3. Read old values
    let legacy_val: i128 = env.storage().persistent().get(&DataKey::LegacyTreasury).unwrap_or(0);

    // 4. Write new values
    env.storage().persistent().set(&DataKey::TreasuryBalance, &legacy_val);

    // 5. Clean up old storage (crucial for ledger health)
    env.storage().persistent().remove(&DataKey::LegacyTreasury);

    // 6. Update storage version
    env.storage().instance().set(&DataKey::Version, &2u32);
}
```

**Lifecycle:**
1. **Upload new WASM**: Install the compiled contract to the ledger.
2. **Upgrade contract**: Call the Soroban system upgrade functionality to swap the WASM.
3. **Invoke migration entrypoint**: Immediately call `migrate()` before unpausing the contract or allowing user interactions.
4. **Verify storage**: Check state to ensure the migration succeeded.
5. **Remove old migration code**: In a future release (v3), the `migrate` function for v1->v2 can be safely removed to save bytecode size.

---

## 7. Upgrade Checklist

Use this practical checklist for every upgrade.

### Before Upgrade
- [ ] All unit and integration tests passing.
- [ ] Storage migration scripts written and rigorously reviewed.
- [ ] `STORAGE_VERSION` constant bumped in code.
- [ ] Testnet deployment and migration fully validated.
- [ ] Production data backup/export completed (if applicable/possible).

### Deployment
- [ ] **Step 1: Install WASM**: Install the compiled WASM binary to the Stellar ledger and obtain its hex hash.
- [ ] **Step 2: Propose / Schedule Upgrade**: Call the `schedule_upgrade(owner, new_wasm_hash)` contract function (emits `UpgradeScheduledEvent`).
- [ ] **Step 3: Monitor Timelock**: Monitor the 24-hour mandatory delay window (17,280 ledgers) for any `UpgradeScheduledEvent` or `UpgradeCancelledEvent` anomalies.
  - If a mistake or key compromise is discovered, the owner must call `cancel_upgrade(owner)` (emits `UpgradeCancelledEvent`) immediately as an escape hatch.
- [ ] **Step 4: Execute Upgrade**: Once the timelock expires (current ledger sequence >= `UpgradeTimelockExpiry`), call `execute_upgrade(owner)` (emits `UpgradedEvent`).
- [ ] **Step 5: Run Migration**: Invoke the `migrate()` entrypoint immediately (if applicable).
- [ ] **Step 6: Verify Version**: Call `get_version()` and verify it returns the incremented version.
- [ ] **Step 7: Validate State**: Validate critical state and balances via RPC queries.

### After Deployment
- [ ] Verify Total Assets, Total Shares, and random User Balances.
- [ ] Verify Vault / Blend position accounting.
- [ ] Verify successful event emission on a small test transaction.
- [ ] Monitor RPC logs for unforeseen deserialization errors.
- [ ] Monitor network dashboards for elevated error rates.

---

## 8. Mainnet Upgrade Procedure

Recommended production flow for upgrading the vault under the timelock architecture:

1. **Step 1: Local Testing**: Deploy and test the upgrade extensively on a local environment using a mainnet state fork.
2. **Step 2: Install WASM**: Upload the compiled new WASM contract to the Testnet network to get the WASM hash.
3. **Step 3: Testnet Scheduling**: Call `schedule_upgrade` on Testnet.
4. **Step 4: Testnet Execution**: After the timelock expires on Testnet, call `execute_upgrade` and run `migrate()`. Verify the flow works end-to-end.
5. **Step 5: Mainnet scheduling announcement**: Schedule the mainnet upgrade and notify stakeholders, detailing the proposed WASM hash and the scheduled execution ledger/time.
6. **Step 6: Install WASM on Mainnet**: Install the WASM bytecode onto Mainnet to acquire the mainnet WASM hash.
7. **Step 7: Schedule Upgrade (Step 1 of Timelock)**: Call `schedule_upgrade(owner, new_wasm_hash)` on the mainnet vault. This initiates the mandatory 24-hour window.
8. **Step 8: Monitoring & Delay**: Monitor the network. Ensure no cancellation events are triggered and check that the correct hash is pending.
9. **Step 9: Execute Upgrade (Step 2 of Timelock)**: Once the timelock sequence is reached, execute the upgrade via `execute_upgrade(owner)`.
10. **Step 10: Run Migration**: Run the migration script and perform post-upgrade validation before resuming normal deposits and withdrawals.

---

## 9. Common Mistakes

**Mistake:** Removing a `DataKey` variant entirely from the enum.
**Result:** Orphaned storage. The data still exists on the ledger, consuming rent/deposits, but the contract completely lacks the type definitions to ever access or delete it.

**Mistake:** Changing struct field order.
**Result:** Deserialization failures. Soroban XDR relies on exact field ordering. The contract will trap/panic whenever it attempts to read the old data.

**Mistake:** Skipping migration version checks in the `migrate` function.
**Result:** Repeated migrations. If a migration is accidentally called twice, it might overwrite valid data with defaults or panic due to missing legacy keys.

---

## 10. Example DataKey Evolution

**Version 1 (Initial):**
```rust
pub enum DataKey {
    Config,
    Vault(u64),
}
```

**Version 2 (Safe Evolution):**
```rust
pub enum DataKey {
    Config,
    Vault(u64),
    Treasury,
}
```
*Why this is safe:* We appended `Treasury` to the end. The XDR discriminants for `Config` (0) and `Vault` (1) remain unchanged. No migration is required for existing data.

**Version 3 (Unsafe Evolution - Migration Required):**
```rust
pub enum DataKey {
    Config,
    Position(u64),
    Treasury,
}
```
*Why migration is required:* `Vault(u64)` was renamed to `Position(u64)`. While the XDR discriminant is technically still `1`, if the semantic meaning changed, or if we changed the inner type (e.g., from `u64` to an `Address`), the old data is now inaccessible via `Position`. A migration must be run to pull data from the old layout and restructure it into the new one.
