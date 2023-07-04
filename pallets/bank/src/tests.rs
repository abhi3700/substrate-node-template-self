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
		assert_eq!(Bank::fd_interest_rate(), None);
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
		assert_eq!(Bank::fd_user_last_id(&ALICE), 0);
		assert_eq!(Bank::fd_user_last_id(&BOB), 0);
		assert_eq!(Bank::fd_user_last_id(&CHARLIE), 0);
	});
}

//=====set_fd_interest_rate=====

// Bank -> 🏦 ✅
#[test]
fn only_root_can_set_fd_interest_rate() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bank::set_fd_interest_rate(RuntimeOrigin::root(), 8_000, 100_000, 100));
		System::assert_last_event(
			Event::FDInterestSet { interest: 8_000, scaling_factor: 100_000, fd_epoch: 100 }.into(),
		)
	});
}

// 🧍 -> 🏦 ❌
#[test]
fn others_cant_set_fd_interest_rate() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Bank::set_fd_interest_rate(RuntimeOrigin::signed(ALICE), 8_000, 100_000, 100),
			DispatchError::BadOrigin
		);
	});
}

//=====set_treasury=====
#[test]
fn only_root_can_set_treasury() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));
		System::assert_last_event(
			Event::TreasurySet {
				account: TREASURY,
				block_num: <frame_system::Pallet<Test>>::block_number(),
			}
			.into(),
		)
	});
}

#[test]
fn others_cant_set_treasury() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Bank::set_treasury(RuntimeOrigin::signed(ALICE), TREASURY),
			DispatchError::BadOrigin
		);
	});
}

//=====open_fd=====
#[test]
fn open_fd_fail_for_zero_amount() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Bank::open_fd(RuntimeOrigin::signed(ALICE), 0),
			Error::<Test>::ZeroAmountWhenOpeningFD
		);
	});
}

#[test]
fn open_fd_fail_when_treasury_not_set() {
	new_test_ext().execute_with(|| {
		assert_eq!(Bank::treasury(), None);
		assert_noop!(
			Bank::open_fd(RuntimeOrigin::signed(ALICE), 100),
			Error::<Test>::TreasuryNotSet
		);
	});
}

#[test]
fn open_fd_fail_when_interest_not_set() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));
		assert_eq!(Bank::fd_interest_rate(), None);
		assert_noop!(
			Bank::open_fd(RuntimeOrigin::signed(ALICE), 100),
			Error::<Test>::FDInterestNotSet
		);
	});
}

#[test]
fn open_fd() {
	new_test_ext().execute_with(|| {
		// set interest details
		assert_ok!(Bank::set_fd_interest_rate(RuntimeOrigin::root(), 8_000, 100_000, 100));

		// set treasury
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));

		// get the pre balance
		let pre_balance = Balances::free_balance(&ALICE);

		// get the FD id before opening FD
		let fd_id_pre = Bank::fd_user_last_id(&ALICE);

		// open fd
		assert_ok!(Bank::open_fd(RuntimeOrigin::signed(ALICE), 100));
		System::assert_last_event(
			Event::FDOpened {
				user: ALICE,
				amount: 100,
				block: <frame_system::Pallet<Test>>::block_number(),
			}
			.into(),
		);

		// get the post balance
		let post_balance = Balances::free_balance(&ALICE);

		// check the post balance if decreased by the FD amount
		assert_eq!(pre_balance - post_balance, 100);

		// check the reserved balance of user is the FD amount
		assert_eq!(Balances::reserved_balance(&ALICE), 100);

		// check the next fd id of user is more than the FD id by 1
		let fd_id_post = Bank::fd_user_last_id(&ALICE);
		assert_eq!(fd_id_post - fd_id_pre, 1);
	});
}

//=====close_fd=====

#[test]
fn close_fd_fails_for_zero_id() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Bank::close_fd(RuntimeOrigin::signed(ALICE), 0),
			Error::<Test>::ZeroIdWhenClosingFD
		);
	});
}

#[test]
fn close_fd_fails_when_fd_not_opened() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Bank::close_fd(RuntimeOrigin::signed(ALICE), 1),
			Error::<Test>::FDNotExistsWithIdWhenClosingFD
		);
	});
}

#[test]
fn close_fd_fails_when_treasury_not_set() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bank::set_fd_interest_rate(RuntimeOrigin::root(), 8_000, 100_000, 100));
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));
		assert_ok!(Bank::open_fd(RuntimeOrigin::signed(ALICE), 100));

		assert_ok!(Bank::reset_treasury(RuntimeOrigin::root()));

		assert_eq!(Bank::treasury(), None);
		assert_noop!(
			Bank::close_fd(RuntimeOrigin::signed(ALICE), 1),
			Error::<Test>::TreasuryNotSet
		);
	});
}

#[test]
fn close_fd_fails_for_invalid_user() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bank::set_fd_interest_rate(RuntimeOrigin::root(), 8_000, 100_000, 100));
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));
		assert_ok!(Bank::open_fd(RuntimeOrigin::signed(ALICE), 100));

		assert_noop!(
			Bank::close_fd(RuntimeOrigin::signed(BOB), 1),
			Error::<Test>::FDNotExistsWithIdWhenClosingFD
		);
	});
}

#[test]
fn close_fd_fails_for_fd_not_matured() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bank::set_fd_interest_rate(RuntimeOrigin::root(), 8_000, 100_000, 100));
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));
		assert_ok!(Bank::open_fd(RuntimeOrigin::signed(ALICE), 100));

		// set the block number to 50
		System::set_block_number(50);

		assert_noop!(
			Bank::close_fd(RuntimeOrigin::signed(ALICE), 1),
			Error::<Test>::FDNotMaturedYet
		);
	});
}

#[test]
fn close_fd() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bank::set_fd_interest_rate(RuntimeOrigin::root(), 8_000, 100_000, 100));
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));
		assert_ok!(Bank::open_fd(RuntimeOrigin::signed(ALICE), 100));

		// set the block number to 62
		System::set_block_number(62);

		// get the pre balance
		let pre_balance = Balances::free_balance(&ALICE);

		// get the treasury pre balance
		let treasury_pre_balance = Balances::free_balance(&TREASURY);

		// close fd
		assert_ok!(Bank::close_fd(RuntimeOrigin::signed(ALICE), 1));
		System::assert_last_event(
			Event::FDClosed { user: ALICE, block: <frame_system::Pallet<Test>>::block_number() }
				.into(),
		);

		// get the post balance
		let post_balance = Balances::free_balance(&ALICE);

		// check the post balance if increased by the FD amount
		// TODO: assert_eq!(post_balance - pre_balance, 100 + interest);
		assert!(post_balance > pre_balance);

		// check the reserved balance of user is zero
		assert_eq!(Balances::reserved_balance(&ALICE), 0);

		// check the treasury post balance if increased by the interest
		let treasury_post_balance = Balances::free_balance(&TREASURY);
		// assert_eq!(treasury_pre_balance - treasury_post_balance, interest_amount);
		assert!(treasury_pre_balance > treasury_post_balance);
	});
}

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
