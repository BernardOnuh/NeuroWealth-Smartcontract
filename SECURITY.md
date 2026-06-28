# Security Model

This document describes the security architecture, trust model, and threat model for the NeuroWealth Vault contract.

## Trust Model

The NeuroWealth Vault implements a partitioned trust model with three distinct roles:

### Owner

The contract owner has the following permissions:
- **Pause/Unpause**: Can halt all deposits and withdrawals during emergencies
- **Set TVL Cap**: Can limit total deposits to manage risk exposure
- **Set User Deposit Cap**: Can limit per-user exposure
- **Update Agent**: Can change the authorized AI agent address
- **Upgrade Contract**: Can upgrade contract code (Phase 2)

The owner **CANNOT**:
- Access user funds directly
- Withdraw funds from user accounts
- Modify user balances

### AI Agent

The authorized AI agent has the following permissions:
- **Rebalance**: Can call `rebalance()` to signal strategy changes and move funds between protocols
- **Update Total Assets**: Can report yield accrual or strategy losses
- **Emergency Pause**: Can trigger an immediate emergency pause if anomalies are detected
- **Read Access**: Can read all vault state to make yield decisions

The agent **CANNOT**:
- Withdraw user funds directly to itself
- Change vault configuration (caps, pools)
- Access USDC tokens directly outside of protocol interactions
- Modify user balances without valid asset reporting

### Users

Regular users have the following permissions:
- **Deposit**: Can deposit USDC into the vault
- **Withdraw**: Can withdraw their own USDC at any time
- **Read**: Can query their balance and vault state

Users **CANNOT**:
- Access other users' funds
- Manipulate vault configuration
- Call agent-only or owner-only functions

## Withdrawal Guarantees

### Automated Liquidity Management

The vault automatically manages liquidity between idle USDC (held in the contract) and deployed assets (e.g., in Blend protocol):
1. **Idle Withdrawals**: If the vault holds sufficient idle USDC, withdrawals are processed immediately.
2. **Protocol Withdrawals**: If idle USDC is insufficient, the vault automatically attempts to withdraw the required amount from the active protocol (e.g., Blend).
3. **Partial Withdrawals**: If the protocol has insufficient liquidity (e.g., high utilization), the user receives all available USDC and **retains their remaining shares** in the vault. This ensures users are not forced into unfavorable liquidations during protocol-wide liquidity crunches.

### Withdrawal Priority

Users can withdraw their USDC at any time without:
- Lock-up periods
- Withdrawal fees
- Approval requirements beyond their signature

## Risk Analysis

### 1. External Protocol Risk (Blend & DEX)

The vault can route idle USDC into external protocols (`get_current_protocol` reports `idle`, `blend`, or `dex`). Each integration introduces systemic risk:
- **Liquidity Risk (Blend)**: If Blend utilization is 100%, the vault cannot pull funds immediately. Users will experience partial withdrawals until liquidity returns to the protocol.
- **Slippage & Liquidity Risk (DEX)**: When the active strategy is a DEX pool, withdrawals and strategy switches execute swaps. Thin pool liquidity can cause slippage or a failed switch; the low-liquidity strategy-switch path returns funds to idle rather than forcing an unfavorable swap.
- **Protocol Failure**: A bug or exploit in Blend or the DEX could result in loss of deployed assets.

### 2. Asset Reporting Risk

The `update_total_assets` function used by the AI agent has built-in guardrails:
- **Solvency Check**: The agent cannot inflate total assets beyond the combined balance of idle USDC and funds actually deployed to external protocols.
- **Decrease Bounding**: Reporting a loss is capped (default 10% per call) to prevent sudden, massive devaluations from a single malicious or erroneous call.

### 3. Agent Rebalance Risk

The AI agent can move funds between protocols via `rebalance()`, but is constrained:
- **Rebalance Cooldown**: Consecutive rebalances are rate-limited by a configurable cooldown (`get_rebalance_cooldown` / `get_last_rebalance_ledger`), which bounds how quickly a compromised or malfunctioning agent can churn funds across protocols.
- **No Direct Custody**: Rebalancing only moves funds between the vault's own positions in whitelisted pools; the agent cannot redirect funds to an arbitrary address.

### 4. Upgrade Risks

The contract owner can upgrade the contract code. This introduces:
- **Single Point of Failure**: The owner key is a high-value target.
- **Mitigation**: Use multi-sig for the owner and timelocks for code upgrades.

### 5. State Rent & TTL Expiry

Soroban persistent entries (such as each user's `Shares` record) accrue state rent and expire if their TTL is not periodically extended:
- **Pure Read-Only Getters**: `get_balance` and `get_shares` are side-effect free — they do **not** extend storage TTL. This keeps pure reads cheap and prevents read traffic from silently mutating ledger state.
- **Explicit Maintenance**: Off-chain indexers or maintenance jobs should call the permissionless `touch_user_ttl(user)` to refresh a user's `Shares` TTL. State-changing calls (`deposit`, `withdraw`) already rewrite `Shares` and refresh its TTL during normal operation.
- **Risk**: A long-dormant user who never transacts and whose entry is never touched could see their `Shares` entry expire and require restoration. Active users, and any indexer running `touch_user_ttl`, are unaffected.

## Access Control Summary

| Function | Owner | Agent | User | Anyone |
|----------|-------|-------|------|--------|
| set_agent | ✅ | - | - | - |
| update_total_assets | - | ✅ | - | - |
| deposit | - | - | ✅ | - |
| withdraw | - | - | ✅ | - |
| rebalance | - | ✅ | - | - |
| pause | ✅ | - | - | - |
| emergency_pause | - | ✅ | - | - |
| unpause | ✅ | - | - | - |
| set_caps | ✅ | - | - | - |
| upgrade | ✅ | - | - | - |
| set_blend_pool | ✅ | - | - | - |
| set_dex_pool | ✅ | - | - | - |
| transfer_ownership | ✅ | - | - | - |
| accept_ownership | - | - | - | pending owner |
| touch_user_ttl | - | - | - | ✅ |

## Security Best Practices Implemented

1. **Checks-Effects-Interactions Pattern**: All state updates happen before external calls
2. **Auth on Withdrawals**: `require_auth()` ensures users can only access their own funds
3. **Minimum Deposits**: Prevents dust attacks
4. **Deposit Caps**: Limits exposure per user
5. **TVL Caps**: Limits total exposure
6. **Pausable**: Emergency stop functionality

## Owner-Compromise Response Runbook

If the owner keypair is suspected or confirmed to be compromised, follow this
sequence immediately. Every step that requires owner auth is marked **[owner]**.

### Step 1 — Pause the vault (within minutes)

The single fastest action to protect user funds is an emergency pause. No new
deposits or withdrawals can execute while the vault is paused.

```bash
stellar contract invoke \
  --id $VAULT_CONTRACT_ID \
  --source <OWNER_SECRET_KEY> \
  --network mainnet \
  -- pause
```

**Requires**: owner auth **[owner]**

If the owner key is already confirmed compromised and you cannot sign with it,
the authorized AI agent can also trigger `emergency_pause`:

```bash
stellar contract invoke \
  --id $VAULT_CONTRACT_ID \
  --source <AGENT_SECRET_KEY> \
  --network mainnet \
  -- emergency_pause
```

**Requires**: agent auth (use this path only if owner key is inaccessible).

### Step 2 — Assess exposure

Before taking further action, determine what the attacker could have done or
is still doing:

| Check | Command |
|---|---|
| Current paused state | `stellar contract invoke --id $VAULT_CONTRACT_ID --network mainnet -- get_paused` |
| Current owner address | `stellar contract invoke --id $VAULT_CONTRACT_ID --network mainnet -- get_owner` |
| Current agent address | `stellar contract invoke --id $VAULT_CONTRACT_ID --network mainnet -- get_agent` |
| Active protocol (idle/blend/dex) | `stellar contract invoke --id $VAULT_CONTRACT_ID --network mainnet -- get_current_protocol` |
| TVL cap | `stellar contract invoke --id $VAULT_CONTRACT_ID --network mainnet -- get_tvl_cap` |

Owner-only actions an attacker with the key could have taken:
- Called `set_agent` to replace the AI agent with a malicious address.
- Called `set_blend_pool` or `set_dex_pool` to point the vault at a drain contract.
- Called `set_caps` to raise or remove deposit limits.
- Initiated `transfer_ownership` to a new address they control.

**The attacker cannot directly withdraw user funds** — withdrawals require
the *user's* own auth signature, not the owner key.

### Step 3 — Rotate the owner key

Generate a new owner keypair on an air-gapped machine. Then initiate the
two-step ownership transfer from the current (compromised) key while you still
control it:

```bash
# Step 3a — propose new owner [owner]
stellar contract invoke \
  --id $VAULT_CONTRACT_ID \
  --source <CURRENT_OWNER_SECRET_KEY> \
  --network mainnet \
  -- transfer_ownership \
  --new_owner <NEW_OWNER_ADDRESS>

# Step 3b — accept from the new keypair [pending owner]
stellar contract invoke \
  --id $VAULT_CONTRACT_ID \
  --source <NEW_OWNER_SECRET_KEY> \
  --network mainnet \
  -- accept_ownership
```

If the compromised key has already been used to initiate an attacker-controlled
`transfer_ownership`, the pending owner is stored under `DataKey::PendingOwner`.
You must call `accept_ownership` from the *legitimate* new owner before the
attacker does. Check `DataKey::PendingOwner` on-chain immediately.

### Step 4 — Revert any attacker configuration changes

Once the new owner key is in place, audit and reset all owner-controlled state:

```bash
# Reset agent to the legitimate AI agent address [owner]
stellar contract invoke --id $VAULT_CONTRACT_ID --source <NEW_OWNER_KEY> \
  --network mainnet -- set_agent --agent <LEGITIMATE_AGENT_ADDRESS>

# Reset pool addresses to audited contracts [owner]
stellar contract invoke --id $VAULT_CONTRACT_ID --source <NEW_OWNER_KEY> \
  --network mainnet -- set_blend_pool --pool_address <AUDITED_BLEND_POOL>

stellar contract invoke --id $VAULT_CONTRACT_ID --source <NEW_OWNER_KEY> \
  --network mainnet -- set_dex_pool --pool_address <AUDITED_DEX_POOL>

# Restore caps to pre-incident values [owner]
stellar contract invoke --id $VAULT_CONTRACT_ID --source <NEW_OWNER_KEY> \
  --network mainnet -- set_caps \
  --user_deposit_cap <ORIGINAL_CAP> --tvl_cap <ORIGINAL_TVL_CAP>
```

### Step 5 — Restore safe operation

Only unpause once Steps 1–4 are fully complete and verified.

```bash
stellar contract invoke \
  --id $VAULT_CONTRACT_ID \
  --source <NEW_OWNER_SECRET_KEY> \
  --network mainnet \
  -- unpause
```

**Requires**: owner auth **[owner]**

### Step 6 — Post-incident

- Revoke and rotate all credentials that were co-located with the compromised key.
- Publish a post-mortem within 72 hours.
- Consider migrating to a multi-sig owner address before resuming normal operations.

---

## Audit & Mainnet Deployment Checklist

Before any mainnet deployment, you must refer to and complete the formal [Mainnet Deployment Checklist](docs/MAINNET_CHECKLIST.md).

Additionally, ensure:

- [ ] All functions have documented panic conditions
- [ ] All state changes emit events
- [ ] Access control verified for each function
- [ ] Upgrade mechanism tested on testnet
- [ ] Pause/unpause tested
- [ ] Withdrawal flow tested with edge cases
- [ ] Maximum deposit limits enforced
- [ ] TVL cap enforced
- [ ] Integration with USDC token tested
- [ ] Integration with Blend protocol tested (Phase 2)
