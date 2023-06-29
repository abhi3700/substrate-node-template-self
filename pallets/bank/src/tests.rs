//! # Tests for the lockable-currency pallet.
//!
//! NOTE: Locking is validated based on success/failure of transfer of funds
//! from one account to another.

#![allow(unused)]

use crate::{mock::*, pallet, Error, Event};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::DispatchError;

//=====getters=====

#[test]
fn get_default_fd_interest_rate() {
	new_test_ext().execute_with(|| {
		assert_eq!(Bank::fd_interest(), None);
	});
}

#[test]
fn get_default_block_duration() {
	new_test_ext().execute_with(|| {
		assert_eq!(Bank::fd_block_duration(), None);
	});
}

#[test]
fn get_default_treasury() {
	new_test_ext().execute_with(|| {
		assert_eq!(Bank::treasury(), None);
	});
}

#[test]
fn get_default_fd_user_id() {
	new_test_ext().execute_with(|| {
		assert_eq!(Bank::fd_user_ids(&ALICE), 0);
		assert_eq!(Bank::fd_user_ids(&BOB), 0);
		assert_eq!(Bank::fd_user_ids(&CHARLIE), 0);
	});
}

//=====set_fd_interest_rate=====

//  -> 🏦 ✅
#[test]
fn only_root_can_set_fd_interest_rate() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bank::set_fd_interest_rate(RuntimeOrigin::root(), 800_000, 100_000));
		System::assert_last_event(
			Event::FDInterestSet { interest: 800_000, scaling_factor: 100_000 }.into(),
		)
	});
}

// 🧍 -> 🏦 ❌
#[test]
fn others_cant_set_fd_interest_rate() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Bank::set_fd_interest_rate(RuntimeOrigin::signed(ALICE), 800_000, 100_000),
			DispatchError::BadOrigin
		);
	});
}
//=====set_fd_block_duration=====

#[test]
fn only_root_can_set_fd_block_duration() {}

#[test]
fn others_cant_set_fd_block_duration() {}

//=====open_fd=====
#[test]
fn open_fd() {}

//=====close_fd=====
#[test]
fn close_fd_before_blocks_limit() {}

#[test]
fn close_fd_after_blocks_limit() {}

//=====lock_for_dao=====

/// 🧍 -> lock 0 (≤ min., < free) ❌
/// 🧍 -> lock 10 (≤ min., < free) ❌
#[test]
fn fails_when_lock_less_for_dao() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(&ALICE), 10_000);
		assert_noop!(
			Bank::lock_for_dao(RuntimeOrigin::signed(ALICE), 0),
			Error::<Test>::LockAmountIsLessThanMinLockAmount
		);

		assert_noop!(
			Bank::lock_for_dao(RuntimeOrigin::signed(ALICE), 19),
			Error::<Test>::LockAmountIsLessThanMinLockAmount
		);
		assert_eq!(Balances::free_balance(&ALICE), 10_000); // no change
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(ALICE), BOB, 10_000)); // transfer 10_000 (all)
	});
}

/// 🧍 -> lock 100_001 (≥ max., > free) ❌
/// 🧍 -> lock u128::MAX (≥ max., > free) ❌
#[test]
fn fails_when_lock_more_for_dao() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(&ALICE), 10_000);
		assert_noop!(
			Bank::lock_for_dao(RuntimeOrigin::signed(ALICE), 100_001),
			Error::<Test>::LockAmountExceedsMaxLockAmount
		);

		assert_noop!(
			Bank::lock_for_dao(RuntimeOrigin::signed(ALICE), u128::MAX),
			Error::<Test>::LockAmountExceedsMaxLockAmount
		);
		assert_eq!(Balances::free_balance(&ALICE), 10_000); // no change
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(ALICE), BOB, 10_000)); // transfer 10_000 (all)
	});
}

/// 🧍 -> lock 21 (≥ min., < free) ✅
/// 🧍 -> lock 100_000 (≤ max., > free) ✅
#[test]
fn lock_valid_amt_for_dao() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(&ALICE), 10_000);
		assert_ok!(Bank::lock_for_dao(RuntimeOrigin::signed(ALICE), 21));
		System::assert_last_event(
			Event::LockedForDAO { user: ALICE, amount: 21, block: System::block_number() }.into(),
		);

		assert_ok!(Bank::lock_for_dao(RuntimeOrigin::signed(ALICE), 100_000));
		System::assert_last_event(
			Event::LockedForDAO { user: ALICE, amount: 100_000, block: System::block_number() }
				.into(),
		);
		assert_eq!(Balances::free_balance(&ALICE), 10_000); // no change
		assert_noop!(
			Balances::transfer(RuntimeOrigin::signed(ALICE), BOB, 10_000),
			pallet_balances::Error::<Test>::LiquidityRestrictions
		); // transfer 10_000 (all)
	});
}

//=====unlock=====
#[test]
fn unlock() {}
