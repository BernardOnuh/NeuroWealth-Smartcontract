//! Event topic constants for the NeuroWealth Vault contract.
//!
//! This module is the single source of truth for every event topic emitted
//! by the vault. `lib.rs` imports these constants directly rather than
//! redefining its own copies, so the on-chain symbols, this file, and
//! `EVENTS.md` cannot drift apart. Symbols are limited to 9 characters
//! (the `symbol_short!` limit).

#![allow(missing_docs)]

use soroban_sdk::{symbol_short, Symbol};

pub const TOPIC_INIT: Symbol = symbol_short!("init");
pub const TOPIC_DEPOSIT: Symbol = symbol_short!("deposit");
pub const TOPIC_WITHDRAW: Symbol = symbol_short!("withdraw");
pub const TOPIC_REBALANCE: Symbol = symbol_short!("rebalance");
pub const TOPIC_PAUSED: Symbol = symbol_short!("paused");
pub const TOPIC_UNPAUSED: Symbol = symbol_short!("unpaused");
pub const TOPIC_EMERGENCY_PAUSED: Symbol = symbol_short!("emerg");
pub const TOPIC_TVL_CAP_UPDATED: Symbol = symbol_short!("tvl_cap");
pub const TOPIC_USER_CAP_UPDATED: Symbol = symbol_short!("user_cap");
pub const TOPIC_LIMITS_UPDATED: Symbol = symbol_short!("l_upd");
pub const TOPIC_DEPOSIT_LIMITS_UPDATED: Symbol = symbol_short!("dep_lim");
pub const TOPIC_CAPS_UPDATED: Symbol = symbol_short!("caps_upd");
pub const TOPIC_AGENT_UPDATED: Symbol = symbol_short!("agent");
pub const TOPIC_OWNERSHIP_INITIATED: Symbol = symbol_short!("own_init");
pub const TOPIC_OWNERSHIP_TRANSFERRED: Symbol = symbol_short!("own_xfer");
pub const TOPIC_OWNERSHIP_CANCELLED: Symbol = symbol_short!("own_cncl");
pub const TOPIC_ASSETS_UPDATED: Symbol = symbol_short!("assets");
pub const TOPIC_UPGRADED: Symbol = symbol_short!("upgraded");
pub const TOPIC_BLEND_SUPPLY: Symbol = symbol_short!("blend_sup");
pub const TOPIC_BLEND_WITHDRAW: Symbol = symbol_short!("blend_wd");
pub const TOPIC_BLEND_POOL_CONFIGURED: Symbol = symbol_short!("blend_cfg");
pub const TOPIC_DEX_SUPPLY: Symbol = symbol_short!("dex_sup");
pub const TOPIC_DEX_WITHDRAW: Symbol = symbol_short!("dex_wd");
pub const TOPIC_DEX_POOL_CONFIGURED: Symbol = symbol_short!("dex_cfg");
pub const TOPIC_PROTOCOL_CHANGED: Symbol = symbol_short!("proto_chg");
pub const TOPIC_USER_STRATEGY_UPDATED: Symbol = symbol_short!("usr_strat");
pub const TOPIC_REBALANCE_FAILED: Symbol = symbol_short!("reb_fail");
pub const TOPIC_AGENT_UPDATE_PROPOSED: Symbol = symbol_short!("agt_prop");
pub const TOPIC_AGENT_UPDATE_CONFIRMED: Symbol = symbol_short!("agt_conf");
pub const TOPIC_AGENT_UPDATE_CANCELLED: Symbol = symbol_short!("agt_cncl");
pub const TOPIC_UPGRADE_SCHEDULED: Symbol = symbol_short!("upg_sched");
pub const TOPIC_UPGRADE_CANCELLED: Symbol = symbol_short!("upg_cncl");
