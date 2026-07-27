## Summary
- Added a dedicated CI job for vault-client TypeScript typechecking and test execution.
- Added npm scripts for build, test, typecheck, and lint in the vault-client package.
- Introduced a typed VaultError wrapper for contract error handling in the generated client.
- Extended deployment verification to cover the newer getter functions for strategy, balances, breakdowns, cooldowns, approval TTL, and pending updates.

## Testing
- npm --prefix packages/vault-client install
- npm --prefix packages/vault-client run typecheck
- npm --prefix packages/vault-client run test
- bash scripts/verify-deployment.sh /tmp/does-not-exist
