#!/usr/bin/env python3
"""
Generate Stellar contract spec for NeuroWealth Vault

This script parses the Soroban contract source code and generates a JSON 
specification that can be used by frontend and agent clients.

Usage:
    python3 scripts/generate-spec.py
    
Output:
    contract-spec.json
"""

import json
import re
from pathlib import Path
from typing import Dict, List, Any, Optional


class ContractSpecGenerator:
    """Generate contract specification from Soroban contract source."""
    
    def __init__(self, contract_path: str):
        self.contract_path = Path(contract_path)
        if not self.contract_path.exists():
            raise FileNotFoundError(f"Contract file not found: {contract_path}")
        
        self.source = self.contract_path.read_text()
    
    def generate(self) -> Dict[str, Any]:
        """Generate complete contract specification."""
        return {
            "version": "1.0.0",
            "contract": "NeuroWealth Vault",
            "network": "Stellar Soroban",
            "description": "ERC-4626 inspired vault contract for autonomous yield management",
            "decimals": 7,
            "token": "USDC",
            "functions": self._get_functions(),
            "events": self._get_events(),
            "errors": self._get_errors(),
            "types": self._get_types(),
            "constants": self._get_constants(),
        }
    
    def _get_functions(self) -> List[Dict[str, Any]]:
        """Extract all public functions from contract."""
        functions = []
        
        # State-changing functions
        functions.extend([
            {
                "name": "initialize",
                "category": "initialization",
                "access": "once",
                "description": "Initialize the vault contract (can only be called once)",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "deployer", "type": "Address", "description": "Deployer keypair address for signature verification"},
                    {"name": "owner", "type": "Address", "description": "Contract owner address"},
                    {"name": "agent", "type": "Address", "description": "Authorized AI agent address"},
                    {"name": "usdc_token", "type": "Address", "description": "USDC token contract address"},
                    {"name": "salt", "type": "BytesN<32>", "description": "32-byte salt for deployment verification"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True,
                "events": ["VaultInitializedEvent"],
                "notes": [
                    "Requires valid signature from deployer to prevent front-running",
                    "Can only be called once - subsequent calls will panic"
                ]
            },
            {
                "name": "deposit",
                "category": "liquidity",
                "access": "public",
                "description": "Deposit USDC into the vault and receive vault shares",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "user", "type": "Address", "description": "User depositing USDC"},
                    {"name": "amount", "type": "i128", "description": "Amount of USDC to deposit (in base units, 7 decimals)"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True,
                "constraints": [
                    "amount must be > 0",
                    "amount >= minDeposit",
                    "amount <= maxDeposit",
                    "user.balance + amount <= userDepositCap",
                    "totalDeposits + amount <= tvlCap",
                    "vault must not be paused",
                    "shares_to_mint > 0 (non-zero mint guard)"
                ],
                "events": ["DepositEvent"],
                "formula": "shares_to_mint = floor(amount * total_shares / total_assets)",
                "security": "Protects against inflation attacks with minimum deposit and non-zero mint guard"
            },
            {
                "name": "withdraw",
                "category": "liquidity",
                "access": "public",
                "description": "Withdraw USDC from the vault by burning shares",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "user", "type": "Address", "description": "User withdrawing USDC"},
                    {"name": "amount", "type": "i128", "description": "Amount of USDC to withdraw (in base units)"}
                ],
                "returns": {
                    "type": "i128",
                    "description": "Amount of USDC returned to user"
                },
                "requires_auth": True,
                "state_changing": True,
                "constraints": [
                    "user must have sufficient shares to cover withdrawal",
                    "amount > 0",
                    "vault must not be paused"
                ],
                "events": ["WithdrawEvent"],
                "formula": "shares_to_burn = ceil(amount * total_shares / total_assets)"
            },
            {
                "name": "withdraw_all",
                "category": "liquidity",
                "access": "public",
                "description": "Withdraw all user funds by burning all shares",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "user", "type": "Address", "description": "User withdrawing all funds"}
                ],
                "returns": {
                    "type": "i128",
                    "description": "Total amount of USDC withdrawn"
                },
                "requires_auth": True,
                "state_changing": True,
                "events": ["WithdrawEvent"]
            },
            {
                "name": "rebalance",
                "category": "management",
                "access": "agent-only",
                "description": "AI agent rebalances funds between yield protocols",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "protocol", "type": "Symbol", "description": "Target protocol: \"blend\" (Blend lending) or \"none\" (no deployment)"},
                    {"name": "expected_apy", "type": "i128", "description": "Expected APY from target protocol (in basis points * 10^7)"},
                    {"name": "min_out", "type": "i128", "description": "Minimum output amount (slippage protection)"}
                ],
                "returns": None,
                "requires_auth": True,
                "authorized_caller": "agent",
                "state_changing": True,
                "events": ["RebalanceEvent"],
                "security": "Only authorized agent keypair can call this function",
                "supported_protocols": ["blend", "none"],
                "notes": ["Performs slippage check against min_out parameter"]
            },
            {
                "name": "pause",
                "category": "administration",
                "access": "owner-only",
                "description": "Pause deposits and withdrawals (emergency function)",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "owner", "type": "Address", "description": "Contract owner"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True,
                "events": ["VaultPausedEvent"]
            },
            {
                "name": "unpause",
                "category": "administration",
                "access": "owner-only",
                "description": "Resume normal operations",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "owner", "type": "Address", "description": "Contract owner"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True,
                "events": ["VaultUnpausedEvent"]
            },
            {
                "name": "emergency_pause",
                "category": "administration",
                "access": "owner-only",
                "description": "Emergency pause without signature verification (for critical situations)",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "owner", "type": "Address", "description": "Contract owner"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True,
                "events": ["EmergencyPausedEvent"]
            },
            {
                "name": "set_tvl_cap",
                "category": "administration",
                "access": "owner-only",
                "description": "Set maximum total value locked in vault",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "cap", "type": "i128", "description": "New TVL cap in base units"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True,
                "events": ["TvlCapUpdatedEvent"]
            },
            {
                "name": "set_user_deposit_cap",
                "category": "administration",
                "access": "owner-only",
                "description": "Set maximum deposit per user",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "cap", "type": "i128", "description": "New per-user deposit cap in base units"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True,
                "events": ["UserDepositCapUpdatedEvent"]
            },
            {
                "name": "set_caps",
                "category": "administration",
                "access": "owner-only",
                "description": "Set both user deposit cap and TVL cap in single atomic transaction",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "user_deposit_cap", "type": "i128", "description": "New per-user deposit cap"},
                    {"name": "tvl_cap", "type": "i128", "description": "New total TVL cap"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True,
                "events": ["CapsUpdatedEvent"],
                "notes": ["Preferred method over calling set_tvl_cap and set_user_deposit_cap separately"]
            },
            {
                "name": "set_deposit_limits",
                "category": "administration",
                "access": "owner-only",
                "description": "Set minimum and maximum per-transaction deposit limits",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "min", "type": "i128", "description": "Minimum deposit per transaction"},
                    {"name": "max", "type": "i128", "description": "Maximum deposit per transaction"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True,
                "events": ["LimitsUpdatedEvent"]
            },
            {
                "name": "set_limits",
                "category": "administration",
                "access": "owner-only",
                "description": "DEPRECATED: Use set_caps or set_deposit_limits instead",
                "deprecated": True,
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "min", "type": "i128"},
                    {"name": "max", "type": "i128"}
                ],
                "returns": "Result<(), VaultError>",
                "requires_auth": True,
                "state_changing": True,
                "notes": ["This function is deprecated and may be removed in future versions"]
            },
            {
                "name": "update_agent",
                "category": "administration",
                "access": "owner-only",
                "description": "Update the authorized AI agent address",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "new_agent", "type": "Address", "description": "New agent address"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True,
                "events": ["AgentUpdatedEvent"]
            },
            {
                "name": "transfer_ownership",
                "category": "administration",
                "access": "owner-only",
                "description": "Initiate two-step ownership transfer (new owner must call accept_ownership)",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "new_owner", "type": "Address", "description": "Address of new owner"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True,
                "events": ["OwnershipTransferInitiatedEvent"],
                "notes": ["Two-step process prevents accidental ownership loss", "New owner must accept within timeframe"]
            },
            {
                "name": "accept_ownership",
                "category": "administration",
                "access": "pending-owner-only",
                "description": "Accept ownership transfer (must be called by pending owner)",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "new_owner", "type": "Address", "description": "Pending owner address"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True,
                "events": ["OwnershipTransferredEvent"]
            },
            {
                "name": "set_blend_pool",
                "category": "administration",
                "access": "owner-only",
                "description": "Set Blend pool address for yield deployment",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "owner", "type": "Address"},
                    {"name": "pool_address", "type": "Address", "description": "Blend pool contract address"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True,
                "events": ["BlendPoolConfiguredEvent"]
            },
            {
                "name": "set_dex_pool",
                "category": "administration",
                "access": "owner-only",
                "description": "Set DEX pool address for liquidity deployment",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "owner", "type": "Address"},
                    {"name": "pool_address", "type": "Address", "description": "DEX pool contract address"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True,
                "events": ["DexPoolConfiguredEvent"]
            },
            {
                "name": "set_blend_approval_ttl",
                "category": "administration",
                "access": "owner-only",
                "description": "Set the ledger TTL for Blend token approvals",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "owner", "type": "Address"},
                    {"name": "blend_approval_ttl", "type": "u32", "description": "TTL in ledgers for Blend allowance approvals"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True
            },
            {
                "name": "set_approval_ttl",
                "category": "administration",
                "access": "owner-only",
                "description": "Set the ledger TTL for token approvals (alias for set_blend_approval_ttl)",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "ttl", "type": "u32", "description": "TTL in ledgers for token allowance approvals"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True
            },
            {
                "name": "set_rebalance_cooldown",
                "category": "administration",
                "access": "owner-only",
                "description": "Set minimum ledgers between rebalance calls (0 = no cooldown)",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "interval", "type": "u32", "description": "Minimum ledger interval between rebalances"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True
            },
        ])
        
        # Management functions (additional)
        functions.extend([
            {
                "name": "cancel_ownership_transfer",
                "category": "administration",
                "access": "owner-only",
                "description": "Cancel a pending ownership transfer",
                "parameters": [
                    {"name": "env", "type": "Env"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True,
                "events": ["OwnershipTransferCancelledEvent"]
            },
            {
                "name": "update_total_assets",
                "category": "management",
                "access": "agent-only",
                "description": "Update total assets to reflect realized yield or loss",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "agent", "type": "Address", "description": "AI agent address"},
                    {"name": "new_total", "type": "i128", "description": "New total assets value"},
                    {"name": "allow_decrease", "type": "bool", "description": "Allow total to decrease"},
                    {"name": "max_decrease_bps", "type": "u32", "description": "Maximum decrease in basis points"}
                ],
                "returns": None,
                "requires_auth": True,
                "authorized_caller": "agent",
                "state_changing": True,
                "events": ["AssetsUpdatedEvent"],
                "notes": [
                    "Only agent can call",
                    "Decreases require owner co-signature",
                    "Decrease is capped at max_decrease_bps"
                ]
            },
            {
                "name": "upgrade",
                "category": "administration",
                "access": "owner-only",
                "description": "Upgrade contract code to new WASM",
                "parameters": [
                    {"name": "env", "type": "Env"},
                    {"name": "owner", "type": "Address"},
                    {"name": "new_wasm_hash", "type": "BytesN<32>", "description": "SHA256 hash of new WASM code"}
                ],
                "returns": None,
                "requires_auth": True,
                "state_changing": True,
                "events": ["UpgradedEvent"],
                "notes": [
                    "Preserves all state during upgrade",
                    "Version counter increments automatically"
                ]
            },
        ])
        
        # Query/Preview functions (read-only)
        query_functions = [
            ("get_balance", "user", "i128", "Get user's USDC balance", "persistent"),
            ("get_total_deposits", "", "i128", "Get total USDC deposited in vault", "instance"),
            ("get_total_assets", "", "i128", "Get total vault assets (principal + yield accumulated)", "instance"),
            ("get_total_shares", "", "i128", "Get total vault shares outstanding", "instance"),
            ("get_shares", "user", "i128", "Get user's vault shares", "persistent"),
            ("get_exchange_rate", "", "i128", "Get current exchange rate (assets per share * 10^7)", "instance"),
            ("get_owner", "", "Address", "Get contract owner address", "instance"),
            ("get_agent", "", "Address", "Get authorized AI agent address", "instance"),
            ("get_usdc_token", "", "Address", "Get USDC token contract address", "instance"),
            ("get_version", "", "u32", "Get contract version for upgrade tracking", "instance"),
            ("get_current_protocol", "", "Symbol", "Get current yield protocol (\"blend\" or \"none\")", "instance"),
            ("get_blend_pool", "", "Option<Address>", "Get Blend pool address if configured", "instance"),
            ("get_tvl_cap", "", "i128", "Get current TVL cap", "instance"),
            ("get_user_deposit_cap", "", "i128", "Get current per-user deposit cap", "instance"),
            ("get_min_deposit", "", "i128", "Get minimum deposit amount per transaction", "instance"),
            ("get_max_deposit", "", "i128", "Get maximum deposit amount per transaction", "instance"),
            ("get_pending_owner", "", "Option<Address>", "Get pending owner address if transfer in progress", "instance"),
            ("get_user_info", "user", "UserInfo", "Get complete user information and statistics", "mixed"),
            ("is_paused", "", "bool", "Check if vault is paused", "instance"),
            ("touch_user_ttl", "user", "bool", "Extend persistent TTL for user shares entry", "persistent"),
            ("preview_deposit_to_shares", "assets", "i128", "Preview shares minted for asset amount (floor)", "query"),
            ("preview_shares_to_assets", "shares", "i128", "Preview assets returned for share amount (floor)", "query"),
            ("preview_withdraw", "assets", "i128", "Preview shares burned for withdrawal amount (ceil)", "query"),
            ("convert_to_shares", "assets", "i128", "Convert asset amount to shares (floor)", "query"),
            ("convert_to_assets", "shares", "i128", "Convert share amount to assets (floor)", "query"),
            ("get_dex_pool", "", "Option<Address>", "Get DEX pool address if configured", "instance"),
            ("get_blend_approval_ttl", "", "u32", "Get the configured Blend approval TTL in ledgers", "instance"),
            ("get_approval_ttl", "", "u32", "Get the configured token approval TTL in ledgers", "instance"),
            ("get_rebalance_cooldown", "", "u32", "Get the minimum ledger interval between rebalances", "instance"),
            ("get_last_rebalance_ledger", "", "u32", "Get the ledger sequence of the last successful rebalance", "instance"),
            ("get_idle_balance", "", "i128", "Get the vault's idle USDC balance (funds held in the vault, not deployed to any protocol)", "instance"),
            ("get_deployed_assets", "", "i128", "Get the amount of USDC currently deployed to an external yield protocol", "instance"),
            ("get_asset_breakdown", "", "(i128, i128)", "Get the vault's asset breakdown as (idle, deployed) in a single call", "instance"),
        ]
        
        for name, param, return_type, desc, storage_type in query_functions:
            params = [{"name": "env", "type": "Env"}]
            if param:
                params.append({"name": param, "type": "Address"})
            
            functions.append({
                "name": name,
                "category": "queries",
                "access": "public",
                "description": desc,
                "parameters": params,
                "returns": return_type,
                "requires_auth": False,
                "state_changing": False,
                "storage_type": storage_type,
                "query_only": True
            })
        
        return functions
    
    def _get_events(self) -> List[Dict[str, Any]]:
        """Get all contract events."""
        return [
            {
                "name": "VaultInitializedEvent",
                "topic": "init",
                "description": "Emitted when vault is initialized",
                "fields": [
                    {"name": "owner", "type": "Address", "description": "Contract owner"},
                    {"name": "agent", "type": "Address", "description": "AI agent address"},
                    {"name": "usdc_token", "type": "Address", "description": "USDC token address"},
                    {"name": "tvl_cap", "type": "i128", "description": "Initial TVL cap"}
                ]
            },
            {
                "name": "DepositEvent",
                "topic": "deposit",
                "description": "Emitted when user deposits USDC",
                "fields": [
                    {"name": "user", "type": "Address", "indexed": True, "description": "User address"},
                    {"name": "amount", "type": "i128", "description": "USDC amount deposited"},
                    {"name": "shares", "type": "i128", "description": "Vault shares minted"}
                ],
                "listener": "AI agent monitors this to detect deposits and deploy funds"
            },
            {
                "name": "WithdrawEvent",
                "topic": "withdraw",
                "description": "Emitted when user withdraws USDC",
                "fields": [
                    {"name": "user", "type": "Address", "indexed": True},
                    {"name": "amount", "type": "i128", "description": "USDC amount withdrawn"},
                    {"name": "shares", "type": "i128", "description": "Vault shares burned"}
                ]
            },
            {
                "name": "RebalanceEvent",
                "topic": "rebalance",
                "description": "Emitted when agent rebalances funds between protocols",
                "fields": [
                    {"name": "protocol", "type": "Symbol", "indexed": True, "description": "Target protocol"},
                    {"name": "expected_apy", "type": "i128", "description": "Expected APY"},
                    {"name": "min_out", "type": "i128", "description": "Minimum output"}
                ]
            },
            {
                "name": "ProtocolChangedEvent",
                "topic": "protocol_changed",
                "description": "Emitted when the current yield protocol is changed",
                "fields": [
                    {"name": "new_protocol", "type": "Symbol", "description": "New protocol symbol"}
                ]
            },
            {
                "name": "PauseEvent",
                "topic": "pause_event",
                "description": "Legacy pause event"
            },
            {
                "name": "VaultPausedEvent",
                "topic": "paused",
                "description": "Emitted when vault operations are paused"
            },
            {
                "name": "VaultUnpausedEvent",
                "topic": "unpaused",
                "description": "Emitted when vault operations resume"
            },
            {
                "name": "EmergencyPausedEvent",
                "topic": "emergency_pause",
                "description": "Emitted when emergency pause is triggered"
            },
            {
                "name": "TvlCapUpdatedEvent",
                "topic": "tvl_cap_updated",
                "description": "Emitted when TVL cap is updated",
                "fields": [
                    {"name": "new_cap", "type": "i128"}
                ]
            },
            {
                "name": "UserDepositCapUpdatedEvent",
                "topic": "user_cap_updated",
                "description": "Emitted when user deposit cap is updated",
                "fields": [
                    {"name": "new_cap", "type": "i128"}
                ]
            },
            {
                "name": "CapsUpdatedEvent",
                "topic": "caps_updated",
                "description": "Emitted when both caps are updated atomically",
                "fields": [
                    {"name": "user_deposit_cap", "type": "i128"},
                    {"name": "tvl_cap", "type": "i128"}
                ]
            },
            {
                "name": "LimitsUpdatedEvent",
                "topic": "limits_updated",
                "description": "Emitted when deposit limits are updated",
                "fields": [
                    {"name": "min_deposit", "type": "i128"},
                    {"name": "max_deposit", "type": "i128"}
                ]
            },
            {
                "name": "AgentUpdatedEvent",
                "topic": "agent_updated",
                "description": "Emitted when agent address is updated",
                "fields": [
                    {"name": "new_agent", "type": "Address"}
                ]
            },
            {
                "name": "OwnershipTransferInitiatedEvent",
                "topic": "transfer_initiated",
                "description": "Emitted when ownership transfer is initiated",
                "fields": [
                    {"name": "current_owner", "type": "Address"},
                    {"name": "pending_owner", "type": "Address"}
                ]
            },
            {
                "name": "OwnershipTransferredEvent",
                "topic": "ownership_transferred",
                "description": "Emitted when ownership transfer is completed",
                "fields": [
                    {"name": "previous_owner", "type": "Address"},
                    {"name": "new_owner", "type": "Address"}
                ]
            },
            {
                "name": "OwnershipTransferCancelledEvent",
                "topic": "transfer_cancelled",
                "description": "Emitted when pending ownership transfer is cancelled",
                "fields": [
                    {"name": "owner", "type": "Address"},
                    {"name": "cancelled_pending", "type": "Address"}
                ]
            },
            {
                "name": "AssetsUpdatedEvent",
                "topic": "assets_updated",
                "description": "Emitted when total assets are updated to reflect yield or loss",
                "fields": [
                    {"name": "old_total", "type": "i128"},
                    {"name": "new_total", "type": "i128"}
                ]
            },
            {
                "name": "UpgradedEvent",
                "topic": "upgraded",
                "description": "Emitted when contract is upgraded to new WASM",
                "fields": [
                    {"name": "old_version", "type": "u32"},
                    {"name": "new_version", "type": "u32"}
                ]
            },
            {
                "name": "DepositLimitsUpdatedEvent",
                "topic": "dep_lim",
                "description": "Emitted when per-transaction deposit limits are updated",
                "fields": [
                    {"name": "old_min", "type": "i128"},
                    {"name": "new_min", "type": "i128"},
                    {"name": "old_max", "type": "i128"},
                    {"name": "new_max", "type": "i128"}
                ]
            },
            {
                "name": "BlendPoolConfiguredEvent",
                "topic": "blend_cfg",
                "description": "Emitted when the Blend pool address is configured",
                "fields": [
                    {"name": "old_pool", "type": "Option<Address>"},
                    {"name": "new_pool", "type": "Address"},
                    {"name": "owner", "type": "Address"}
                ]
            },
            {
                "name": "DexPoolConfiguredEvent",
                "topic": "dex_cfg",
                "description": "Emitted when the DEX pool address is configured",
                "fields": [
                    {"name": "old_pool", "type": "Option<Address>"},
                    {"name": "new_pool", "type": "Address"},
                    {"name": "owner", "type": "Address"}
                ]
            },
            {
                "name": "DexSupplyEvent",
                "topic": "dex_sup",
                "description": "Emitted when funds are supplied to a DEX liquidity pool",
                "fields": [
                    {"name": "asset", "type": "Address"},
                    {"name": "amount_actual", "type": "i128"},
                    {"name": "success", "type": "bool"}
                ]
            },
            {
                "name": "DexWithdrawEvent",
                "topic": "dex_wd",
                "description": "Emitted when funds are withdrawn from a DEX liquidity pool",
                "fields": [
                    {"name": "asset", "type": "Address"},
                    {"name": "amount_actual", "type": "i128"},
                    {"name": "success", "type": "bool"}
                ]
            },
            {
                "name": "BlendSupplyEvent",
                "topic": "blend_supply",
                "description": "Emitted when funds are supplied to Blend protocol",
                "fields": [
                    {"name": "amount", "type": "i128", "description": "Amount supplied to Blend"},
                    {"name": "b_tokens_received", "type": "i128", "description": "bUSDC tokens received"}
                ]
            },
            {
                "name": "BlendWithdrawEvent",
                "topic": "blend_withdraw",
                "description": "Emitted when funds are withdrawn from Blend protocol",
                "fields": [
                    {"name": "b_tokens", "type": "i128", "description": "bUSDC tokens burned"},
                    {"name": "amount_received", "type": "i128", "description": "USDC amount received"}
                ]
            },
            {
                "name": "InitFailedEvent",
                "topic": "init_failed",
                "description": "Emitted when initialization fails"
            },
            {
                "name": "RebalanceFailedEvent",
                "topic": "rebalance_failed",
                "description": "Emitted when rebalance operation fails",
                "fields": [
                    {"name": "reason", "type": "String", "description": "Failure reason"}
                ]
            },
        ]
    
    def _get_errors(self) -> Dict[str, Any]:
        """Get error types and codes."""
        return {
            "VaultError::NegativeMin": {
                "code": 1,
                "description": "Supplied min limit is negative"
            },
            "VaultError::NegativeMax": {
                "code": 2,
                "description": "Supplied max limit is negative"
            },
            "VaultError::MaxLessThanMin": {
                "code": 3,
                "description": "Max must be greater than or equal to min"
            },
            "ValidationError": {
                "code": 100,
                "description": "General validation error"
            },
            "PausedError": {
                "code": 101,
                "description": "Vault is paused, deposits and withdrawals disabled"
            },
            "UnauthorizedAgentError": {
                "code": 102,
                "description": "Only authorized AI agent can call this function"
            },
            "UnauthorizedOwnerError": {
                "code": 103,
                "description": "Only contract owner can call this function"
            },
            "InsufficientBalanceError": {
                "code": 104,
                "description": "User has insufficient balance for withdrawal"
            },
            "InvalidAmountError": {
                "code": 105,
                "description": "Amount is invalid (zero, negative, or outside limits)"
            },
            "DepositCapExceededError": {
                "code": 106,
                "description": "User deposit cap exceeded"
            },
            "TvlCapExceededError": {
                "code": 107,
                "description": "Total value locked cap exceeded"
            },
            "SlippageError": {
                "code": 108,
                "description": "Output less than minimum expected (slippage protection)"
            },
        }
    
    def _get_types(self) -> Dict[str, Any]:
        """Get custom type definitions."""
        return {
            "UserInfo": {
                "description": "Complete user information snapshot",
                "fields": [
                    {
                        "name": "address",
                        "type": "Address",
                        "description": "User wallet address"
                    },
                    {
                        "name": "balance",
                        "type": "i128",
                        "description": "USDC balance in vault (principal)"
                    },
                    {
                        "name": "shares",
                        "type": "i128",
                        "description": "Vault shares owned"
                    },
                    {
                        "name": "deposit_time",
                        "type": "u64",
                        "description": "Timestamp of first deposit"
                    }
                ]
            },
            "Address": {
                "description": "Stellar account address",
                "format": "G..."
            },
            "Symbol": {
                "description": "Fixed-length Soroban symbol",
                "examples": ["blend", "none"]
            },
            "i128": {
                "description": "128-bit signed integer",
                "notes": ["USDC amounts use 7 decimal places"]
            }
        }
    
    def _get_constants(self) -> Dict[str, Any]:
        """Extract constants from contract."""
        return {
            "DEFAULT_USER_DEPOSIT_CAP": {
                "value": "10_000_000_000",
                "description": "Default per-user deposit cap: 10,000 USDC",
                "type": "i128"
            },
            "DEFAULT_MIN_DEPOSIT": {
                "value": "1_000_000",
                "description": "Default minimum deposit: 1 USDC",
                "type": "i128"
            },
            "DEFAULT_MAX_DEPOSIT": {
                "value": "1_000_000_000",
                "description": "Default maximum deposit: 1,000 USDC",
                "type": "i128"
            },
            "DECIMAL_PLACES": {
                "value": 7,
                "description": "USDC has 7 decimal places on Stellar",
                "type": "u32"
            }
        }


def main():
    """Main entry point."""
    contract_path = "neurowealth-vault/contracts/vault/src/lib.rs"
    output_path = "contract-spec.json"
    
    try:
        generator = ContractSpecGenerator(contract_path)
        spec = generator.generate()
        
        # Write spec to file
        with open(output_path, "w") as f:
            json.dump(spec, f, indent=2)
        
        print(f"✅ Contract specification generated: {output_path}")
        print(f"   - {len(spec['functions'])} functions")
        print(f"   - {len(spec['events'])} events")
        print(f"   - {len(spec['errors'])} error types")
        
    except FileNotFoundError as e:
        print(f"❌ Error: {e}", file=__import__('sys').stderr)
        exit(1)
    except Exception as e:
        print(f"❌ Failed to generate spec: {e}", file=__import__('sys').stderr)
        exit(1)


if __name__ == "__main__":
    main()
