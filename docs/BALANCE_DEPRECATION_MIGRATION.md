# Balance Storage Deprecation — Migration Guide

## Overview

`DataKey::Balance(Address)` is the **first variant** (discriminant 0) of the vault's
`DataKey` enum. It is **deprecated** — no production code reads or writes it — but it
**cannot be removed** because doing so would shift the discriminant values of every
subsequent variant, breaking on-chain storage compatibility across upgrades.

All user balances are now derived from `Shares(user)` and the current exchange rate:

```text
balance = (shares * sharePrice) / PRECISION
```

## Why Balance Is Retained

Soroban serializes enum variants by their position (discriminant). The current layout:

| Discriminant | Variant        | Status     |
|-------------|----------------|------------|
| 0           | `Balance(Address)` | **Deprecated** — retained for layout only |
| 1           | `Shares(Address)`  | Active — canonical per-user storage |
| 2+          | Other variants     | Active |

Removing `Balance(Address)` would make `Shares(Address)` shift from discriminant 1 to
discriminant 0, causing all existing `Shares` storage entries to become unreadable.

## Migration Path

### What Changed

1. **Deposit flow**: Previously wrote `DataKey::Balance(user)` directly. Now mints
   `Shares(user)` and the balance is derived from the share-to-asset exchange rate.
2. **Withdraw flow**: Previously read `DataKey::Balance(user)`. Now reads `Shares(user)`
   and computes the withdrawable amount.
3. **Getters**: `get_balance(user)` now computes `shares * totalAssets / totalShares`
   instead of reading from storage.

### For Integrators

No action required. The `get_balance(user)` API remains unchanged — the derivation
happens internally. Existing off-chain indexers that read events will continue to
work since `DepositEvent` and `WithdrawEvent` emit both `amount` and `shares`.

### For Upgrades

When deploying a new contract version:

1. **Do NOT remove `DataKey::Balance` from the enum.** The variant must remain at
   discriminant 0 with the same `Address` payload type.
2. **Do NOT rename `DataKey::Balance`.** The serialized form depends on the variant
   name for Soroban's `contracttype` macro.
3. You MAY add new variants after the existing ones (appending to the enum).
4. Run the verification script before and after upgrade to ensure storage integrity.

## Verification

### Automated Check

Run the deprecation verification script:

```bash
bash scripts/check-balance-deprecation.sh
```

This script verifies:
- `DataKey::Balance(Address)` is declared and marked deprecated
- No production code paths read or write `DataKey::Balance`
- All test/fuzz references use the mock `TokenDataKey::Balance`, not the vault's
- `Balance` remains the first enum variant (discriminant 0)
- No getters read from the Balance key

### Manual Checklist

- [ ] `DataKey::Balance(Address)` is the first variant in the enum
- [ ] The variant has a `/// Deprecated` doc comment
- [ ] `grep -rn 'DataKey::Balance' neurowealth-vault/contracts/vault/src/lib.rs`
    shows only the enum definition (no reads/writes in function bodies)
- [ ] `get_balance()` derives from shares, not storage
- [ ] All deposits mint shares via `DataKey::Shares`
- [ ] `test_upgrade_compatibility.rs` passes (discriminant stability test)
- [ ] `scripts/check-balance-deprecation.sh` exits 0

### What to Look For in Code Reviews

Any new code that introduces `DataKey::Balance` reads or writes should be **rejected**
in code review with the following rationale:

> DataKey::Balance is deprecated. User balances must be derived from
> Shares(user) and the exchange rate. See docs/BALANCE_DEPRECATION_MIGRATION.md.

## Future Removal

In a hypothetical future where Soroban supports storage schema versioning or
migration entrypoints, `DataKey::Balance` could be removed. Until then:

- The variant stays in the enum at discriminant 0
- The `BalanceDeprecation.sol` placeholder in `contracts/migrations/` is retained
  as documentation of the migration concept
- The test in `test_balance_deprecation.rs` validates layout stability on every
  CI run

## References

- `neurowealth-vault/contracts/vault/src/lib.rs` — `DataKey` enum definition
- `neurowealth-vault/contracts/vault/src/tests/test_upgrade_compatibility.rs` — discriminant stability tests
- `scripts/check-balance-deprecation.sh` — automated verification script
- `docs/UPGRADE_MIGRATION.md` — general upgrade migration guide
