# Partial Withdrawal Behavior Under Liquidity Shortage

## Overview

When protocol liquidity is insufficient for a full withdrawal request, the system implements a graceful partial withdrawal mechanism.

## Summary

Users and integrators need to understand what happens when protocol liquidity is insufficient for a full withdrawal. This document defines:
- How partial withdrawals work
- When they are triggered
- What users can expect
- Recovery mechanisms

## Withdrawal Semantics

### Full Withdrawal (Normal Case)
When sufficient liquidity exists, users receive 100% of their requested withdrawal amount:

User Balance: 100 USDC
Withdrawal Request: 100 USDC
Available Liquidity: 200 USDC

Result: 100 USDC transferred immediately ✓

### Partial Withdrawal (Liquidity Shortage)
When liquidity is insufficient, the user receives what's available:

User Balance: 100 USDC
Withdrawal Request: 100 USDC
Available Liquidity: 60 USDC

Result: 60 USDCransferred immediately
        40 USDC remains in withdrawal queue
        User receives withdrawal receipt with both amounts

## Control Flow

### Step 1: Withdrawal Request
User submits withdrawal request
↓
System checks: available_liquidity >= requested_amount?

### Step 2: Decision Point
If YES → Full Withdrawal Path
   Transfer full amount
   
If NO → Partial Withdrawal Path
   Transfer available liquidity
   Queue remaining amount

## User-Visible Behavior

Users receive a detailed withdrawal receipt containing:
- withdrawalId
- requestedAmount
- immediateTransfer
- queuedAmount
- status
- nextClaimTime

## Examples

### Example 1: Shortage During High Demand
Pool State:
  Total Deposits: 1000 USDC
  Current Liquidity: 150 USDC
  
Alice's Withdrawal:
  Request: 200 USDC
  Available: 150 USDC
  
  Outcome:
  - Receives: 150 USDC immediately
  - Queued: 50 USDC

### Example 2: Cascading Partial Withdrawals
Bob withdraws 100 USDC → Receives 100 USDC
Carol withdraws 150 USDC → Receives 100 USDC,0 USDC
Dave withdraws 200 USDC → Receives 0 USDC, Queued: 200 USDC

### Example 3: Liquidity Recovery
Liquidity Injection: 300 USDC received

Auto-Processing:
  ✓ Carol claims 50 USDC
  ✓ Dave claims 200 USDC
  
Queue State: Empty

## Recovery Mechanisms

### Automatic Processing
When new liquidity arrives, the protocol automatically processes queued withdrawals in FIFO order.

### Manual Claim
Users can manually claim their queued withdrawals once liquidity is available.

## State Transitions

WITHDRAWAL_REQUESTED
    ↓
    ├─→ [Sufficient Liquidity] → COMPLETED
    │   
    └─→ [Insufficient Liquidity] → PARTIAL_COMPLETED → QUEUED_WITHDRAWAL → COMPLETED

## Important Notes

- No Funds Lost: All amounts are preserved
- FIFO Processing: Queue is processed in order
- No Expiration: Queued withdrawals do not expire
- Interest Accrual: Queued amounts may accrue interest
- Priority Claims: Users can manually claim when ready
