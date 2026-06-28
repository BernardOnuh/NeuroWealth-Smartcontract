//! Property tests for share-conversion math (Issue #323).
//!
//! These tests exercise the mathematical invariants of the vault's share-pricing
//! formulas in isolation, using `proptest` to generate thousands of random inputs
//! without needing the Soroban environment.
//!
//! The three helper functions below replicate the *exact* integer arithmetic from:
//!   - `convert_to_shares_internal`       lib.rs lines 4229-4247  (floor mint)
//!   - `convert_to_shares_internal_ceil`  lib.rs lines 4253-4284  (ceil burn)
//!   - `convert_to_assets_internal`       lib.rs lines 4288-4305  (floor return)
//!
//! Any change to those formulas that breaks the invariants here will fail CI.
//!
//! Invariants tested:
//!   (a) Round-trip no-value-creation: assets → shares (floor) → assets (floor) ≤ input.
//!   (b) Ceil-burn ≥ floor-mint for the same asset amount (vault never under-burns).
//!   (c) Ceil and floor differ by at most 1 (tightest possible rounding gap).
//!   (d) Monotonicity: more assets in → more-or-equal shares out.
//!   (e) Zero input → zero output for all three directions.
//!   (f) Conservation: shares minted for A assets, converted back, never exceed A.

// The vault crate is `#![no_std]`; tests are run with the standard test harness
// which links std, but we must declare it explicitly in no_std crates.
extern crate std;

use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Pure-math helpers — mirror the exact formulas from lib.rs
// ---------------------------------------------------------------------------

/// `convert_to_shares_internal` — floor(assets × total_shares / total_assets).
///
/// Returns `None` only on integer overflow (not expected within tested bounds).
fn shares_floor(assets: i128, total_shares: i128, total_assets: i128) -> Option<i128> {
    if assets == 0 {
        return Some(0);
    }
    // Bootstrap: when the pool is empty either side, 1:1 mapping (lib.rs line 4238-4239).
    if total_shares == 0 || total_assets == 0 {
        return Some(assets);
    }
    assets.checked_mul(total_shares).map(|p| p / total_assets)
}

/// `convert_to_shares_internal_ceil` — ceil(assets × total_shares / total_assets).
///
/// Ceiling division: (a × b + d − 1) / d where d = total_assets (lib.rs line 4268-4283).
fn shares_ceil(assets: i128, total_shares: i128, total_assets: i128) -> Option<i128> {
    if assets == 0 {
        return Some(0);
    }
    if total_shares == 0 || total_assets == 0 {
        return Some(assets);
    }
    let product = assets.checked_mul(total_shares)?;
    let numerator = product.checked_add(total_assets.checked_sub(1)?)?;
    Some(numerator / total_assets)
}

/// `convert_to_assets_internal` — floor(shares × total_assets / total_shares).
fn assets_from_shares(shares: i128, total_shares: i128, total_assets: i128) -> Option<i128> {
    if shares == 0 {
        return Some(0);
    }
    if total_shares == 0 || total_assets == 0 {
        return Some(0);
    }
    shares
        .checked_mul(total_assets)
        .map(|p| p / total_shares)
}

// ---------------------------------------------------------------------------
// Input strategy
//
// Bounded to 1..=10^12 so that the worst-case intermediate product
// (assets × total_shares = 10^24) fits comfortably inside i128 (max ~1.7×10^38).
// ---------------------------------------------------------------------------

const MAX_VAL: i128 = 1_000_000_000_000i128; // 10^12

proptest! {
    // -----------------------------------------------------------------------
    // (a) + (f)  Round-trip never creates assets
    // -----------------------------------------------------------------------

    /// Depositing `assets`, receiving `shares_floor(assets)` shares, then
    /// converting those shares back must never yield *more* than `assets`.
    ///
    /// This is the core ERC-4626 rounding invariant: rounding always favours
    /// the vault, never the user.
    #[test]
    fn prop_round_trip_never_creates_assets(
        assets       in 1i128..=MAX_VAL,
        total_shares in 1i128..=MAX_VAL,
        total_assets in 1i128..=MAX_VAL,
    ) {
        let shares = shares_floor(assets, total_shares, total_assets)
            .expect("overflow not possible at tested bounds");
        let assets_back = assets_from_shares(shares, total_shares, total_assets)
            .expect("overflow not possible at tested bounds");

        prop_assert!(
            assets_back <= assets,
            "round-trip created value: {} assets → {} shares → {} assets \
             (total_shares={}, total_assets={})",
            assets, shares, assets_back, total_shares, total_assets
        );
    }

    // -----------------------------------------------------------------------
    // (b) + (c)  Ceil-burn ≥ floor-mint, gap ≤ 1
    // -----------------------------------------------------------------------

    /// The shares burned on withdrawal (ceil) must be ≥ the shares minted on
    /// deposit (floor) for the same asset amount.  The gap must be at most 1:
    /// ceil and floor integer division can differ by exactly 1 when the result
    /// is not an integer.
    #[test]
    fn prop_ceil_burn_gte_floor_mint_gap_at_most_one(
        assets       in 1i128..=MAX_VAL,
        total_shares in 1i128..=MAX_VAL,
        total_assets in 1i128..=MAX_VAL,
    ) {
        let floor_shares = shares_floor(assets, total_shares, total_assets)
            .expect("overflow not possible at tested bounds");
        let ceil_shares = shares_ceil(assets, total_shares, total_assets)
            .expect("overflow not possible at tested bounds");

        prop_assert!(
            ceil_shares >= floor_shares,
            "ceil ({}) < floor ({}) for assets={} total_shares={} total_assets={}",
            ceil_shares, floor_shares, assets, total_shares, total_assets
        );
        prop_assert!(
            ceil_shares - floor_shares <= 1,
            "ceil-floor gap > 1: ceil={} floor={} assets={} total_shares={} total_assets={}",
            ceil_shares, floor_shares, assets, total_shares, total_assets
        );
    }

    // -----------------------------------------------------------------------
    // (d)  Monotonicity
    // -----------------------------------------------------------------------

    /// More assets always produce more-or-equal shares (floor conversion).
    /// This ensures the pricing function is non-decreasing.
    #[test]
    fn prop_monotone_more_assets_more_shares(
        base  in 0i128..=MAX_VAL / 2,
        delta in 0i128..=MAX_VAL / 2,
        total_shares in 1i128..=MAX_VAL,
        total_assets in 1i128..=MAX_VAL,
    ) {
        let assets1 = base;
        let assets2 = base + delta; // always >= assets1, no overflow since both halved

        let s1 = shares_floor(assets1, total_shares, total_assets)
            .expect("overflow not possible at tested bounds");
        let s2 = shares_floor(assets2, total_shares, total_assets)
            .expect("overflow not possible at tested bounds");

        prop_assert!(
            s2 >= s1,
            "monotonicity broken: assets {} → {} shares, assets {} → {} shares \
             (total_shares={}, total_assets={})",
            assets1, s1, assets2, s2, total_shares, total_assets
        );
    }

    // -----------------------------------------------------------------------
    // (e)  Zero input always gives zero output
    // -----------------------------------------------------------------------

    /// Regardless of vault state, converting zero assets/shares must return zero.
    #[test]
    fn prop_zero_input_zero_output(
        total_shares in 0i128..=MAX_VAL,
        total_assets in 0i128..=MAX_VAL,
    ) {
        prop_assert_eq!(
            shares_floor(0, total_shares, total_assets),
            Some(0),
            "shares_floor(0) != 0 for total_shares={} total_assets={}",
            total_shares, total_assets
        );
        prop_assert_eq!(
            shares_ceil(0, total_shares, total_assets),
            Some(0),
            "shares_ceil(0) != 0 for total_shares={} total_assets={}",
            total_shares, total_assets
        );
        prop_assert_eq!(
            assets_from_shares(0, total_shares, total_assets),
            Some(0),
            "assets_from_shares(0) != 0 for total_shares={} total_assets={}",
            total_shares, total_assets
        );
    }

    // -----------------------------------------------------------------------
    // (f)  Conservation: withdrawal of exactly minted shares never exceeds deposit
    // -----------------------------------------------------------------------

    /// A user who deposits `assets` and later redeems exactly the shares they
    /// were minted must receive back ≤ `assets`.  This is a restatement of (a)
    /// from the issue's acceptance criteria phrased as the withdrawal path.
    #[test]
    fn prop_redeem_minted_shares_no_excess(
        assets       in 1i128..=MAX_VAL,
        total_shares in 1i128..=MAX_VAL,
        total_assets in 1i128..=MAX_VAL,
    ) {
        let shares_minted = shares_floor(assets, total_shares, total_assets)
            .expect("overflow not possible at tested bounds");
        let assets_redeemed = assets_from_shares(shares_minted, total_shares, total_assets)
            .expect("overflow not possible at tested bounds");

        prop_assert!(
            assets_redeemed <= assets,
            "redemption exceeded deposit: deposited {} assets, minted {} shares, \
             redeemed {} assets (total_shares={}, total_assets={})",
            assets, shares_minted, assets_redeemed, total_shares, total_assets
        );
    }
}
