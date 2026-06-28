# Changelog

All notable changes to this repository are documented in this file.
This changelog is tied to the vault contract `Version` storage value. Each released contract upgrade should add a new entry matching the stored version number.

> **Contributors:** Any PR that changes on-chain contract behavior, emitted events, error codes,
> or the `Version` storage value **must** update this file. Add your changes under `[Unreleased]`
> and note the target `Version` value if an upgrade is planned. See the PR checklist in
> `.github/pull_request_template.md`.

## [Unreleased]
<!-- Add entries below. Format: `- Short description (Issue #N).` -->
<!-- If this PR bumps get_version(), note the new Version value here. -->
- Add GitHub issue templates as structured YAML forms (Issue #330).
- Migrate weak `!events.is_empty()` test assertions to strict payload checks (Issue #333).
- Dedicated `TvlCapUpdatedEvent` / `UserDepositCapUpdatedEvent` replace ambiguous
  `LimitsUpdatedEvent` for cap-only updates; indexer migration note added to EVENTS.md (Issue #328).
- CHANGELOG.md now tied to contract `Version` with PR template reminder (Issue #335).
- **DEX liquidity pool integration (Issue #228):** the vault can now deploy USDC
  to a Stellar DEX liquidity pool in addition to Blend, implementing the
  on-chain side of the Balanced/Growth strategies.
  - Added owner-configurable `DataKey::DexPool` with `set_dex_pool` / `get_dex_pool`.
  - Added `supply_to_dex` / `withdraw_from_dex` internal helpers mirroring Blend.
  - `rebalance` now accepts the `"dex"` protocol symbol with `min_out` slippage
    protection; `CurrentProtocol` and `ProtocolChangedEvent` reflect DEX deployments.
  - User `withdraw` / `withdraw_all` pull liquidity back from the DEX when needed.
  - New events: `DexSupplyEvent` (`dex_sup`), `DexWithdrawEvent` (`dex_wd`),
    `DexPoolConfiguredEvent` (`dex_cfg`).
  - New errors: `DexPoolNotConfigured` (#46), `OnlyOwnerCanSetDexPool` (#47).
  - New `dex-devnet` test feature flag. No `Version` bump (additive, pre-mainnet).
  - See `docs/DEX_INTEGRATION.md`.

## [1]
- Initial vault implementation with ERC-4626-inspired share accounting.
- `get_version()` returns the contract version from `DataKey::Version`.
- `UpgradedEvent` emits both `old_version` and `new_version` for on-chain auditability.
