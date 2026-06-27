//! Event topic constants for the NeuroWealth Vault contract.
//!
//! This module serves as the single source of truth for all event topics
//! emitted by the vault. Symbols are limited to 9 characters.

#![allow(missing_docs)]

use soroban_sdk::{symbol_short, Symbol};

pub const INIT: Symbol = symbol_short!("init");
pub const DEPOSIT: Symbol = symbol_short!("deposit");
pub const WITHDRAW: Symbol = symbol_short!("withdraw");
pub const REBALANCE: Symbol = symbol_short!("rebalance");
pub const PAUSED: Symbol = symbol_short!("paused");
pub const UNPAUSED: Symbol = symbol_short!("unpaused");
pub const EMERGENCY: Symbol = symbol_short!("emergency");
pub const TVL_CAP: Symbol = symbol_short!("tvl_cap");
pub const USER_CAP: Symbol = symbol_short!("user_cap");
pub const LIMITS: Symbol = symbol_short!("limits");
pub const CAPS: Symbol = symbol_short!("caps");
pub const AGENT: Symbol = symbol_short!("agent");
pub const OWN_INIT: Symbol = symbol_short!("own_init");
pub const OWN_XFER: Symbol = symbol_short!("own_xfer");
pub const OWN_CNCL: Symbol = symbol_short!("own_cncl");
pub const ASSETS: Symbol = symbol_short!("assets");
pub const UPGRADED: Symbol = symbol_short!("upgraded");
pub const BLEND_SUP: Symbol = symbol_short!("blend_sup");
pub const BLEND_WD: Symbol = symbol_short!("blend_wd");
pub const USER_STRATEGY: Symbol = symbol_short!("usr_strat");
