# Balance Storage Deprecation - Shares-Only Accounting Migration

## Problem Statement

The contract currently maintains both Balance(user) and Shares(user). These can drift after yield accrual.

## Solution

Derive balance from shares: `balance = (shares * sharePrice) / PRECISION`

## Scope

- Update deposit/withdraw paths
- Remove redundant Balance storage
- All getters derive from shares
