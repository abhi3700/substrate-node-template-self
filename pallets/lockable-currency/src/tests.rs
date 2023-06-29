//! # Tests for the lockable-currency pallet.
//!
//! NOTE: Locking is validated based on success/failure of transfer of funds
//! from one account to another.

#![allow(unused)]

use crate::{mock::*, /* Error, */ Event};
use frame_support::{assert_noop, assert_ok};

//=====lock_capital=====

/// Here,
/// 🧍 -> lock 0 (< free)
#[test]
fn lock_zero_amt() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(1), 10000);
		assert_ok!(LockableCurrency::lock_capital(RuntimeOrigin::signed(1), 0));
		System::assert_last_event(Event::Locked { user: 1, amount: 0 }.into());
		assert_eq!(Balances::free_balance(1), 10000); // free_balance is still 10000
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(1), 2, 10000)); // transfer all free_balance
	});
}

/// Here,
/// 🧍 -> lock 100 (< free)
#[test]
fn lock_some_amt() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(1), 10000);
		assert_ok!(LockableCurrency::lock_capital(RuntimeOrigin::signed(1), 100));
		System::assert_last_event(Event::Locked { user: 1, amount: 100 }.into());
		assert_eq!(Balances::free_balance(1), 10000); // free_balance is still 10000
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(1), 2, 9900)); // transfer 9900 (remaining 100 is locked)
	});
}

/// Here,
/// 🧍 -> lock 10_000 (= free)
#[test]
fn lock_all_amt() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(1), 10_000);
		assert_ok!(LockableCurrency::lock_capital(RuntimeOrigin::signed(1), 10_000));
		System::assert_last_event(Event::Locked { user: 1, amount: 10_000 }.into());
		assert_eq!(Balances::free_balance(1), 10_000); // free_balance is still 10_000
		assert_noop!(
			Balances::transfer(RuntimeOrigin::signed(1), 2, 10), // transfer some
			pallet_balances::Error::<Test, _>::LiquidityRestrictions
		);
	});
}

/// Here,
/// 🧍 -> lock 10_001 (> free)
#[test]
fn lock_amt_that_exceeds_free_bal() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(1), 10_000);
		assert_ok!(LockableCurrency::lock_capital(RuntimeOrigin::signed(1), 10_001));
		System::assert_last_event(Event::Locked { user: 1, amount: 10_001 }.into());
		assert_eq!(Balances::free_balance(1), 10_000); // free_balance is still 10_000
		assert_noop!(
			Balances::transfer(RuntimeOrigin::signed(1), 2, 10), // transfer some
			pallet_balances::Error::<Test, _>::LiquidityRestrictions
		);
	});
}

//=====extend_lock=====

/// Here,
/// 🧍 -> lock 0
/// 🧍 -> extend lock 0
#[test]
fn extend_lock_zero_after_zero_locked() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(1), 10_000);
		assert_ok!(LockableCurrency::lock_capital(RuntimeOrigin::signed(1), 0));
		assert_ok!(LockableCurrency::extend_lock(RuntimeOrigin::signed(1), 0));
		System::assert_last_event(Event::ExtendedLock { user: 1, amount: 0 }.into());
		assert_eq!(Balances::free_balance(1), 10_000); // free_balance is still 10_000
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(1), 2, 10_000)); // transfer all
	});
}

/// Here,
/// 🧍 -> lock 100
/// 🧍 -> extend lock 100
///
/// take max(100, 100) as locked amount
#[test]
fn extend_lock_same_after_some_locked() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(1), 10_000);
		assert_ok!(LockableCurrency::lock_capital(RuntimeOrigin::signed(1), 100));
		System::assert_last_event(Event::Locked { user: 1, amount: 100 }.into());
		assert_ok!(LockableCurrency::extend_lock(RuntimeOrigin::signed(1), 100));
		System::assert_last_event(Event::ExtendedLock { user: 1, amount: 100 }.into());
		assert_eq!(Balances::free_balance(1), 10_000); // free_balance is still 10_000
		assert_noop!(
			Balances::transfer(RuntimeOrigin::signed(1), 2, 10_000), // fail in transfer of 10_000 free balance
			pallet_balances::Error::<Test, _>::LiquidityRestrictions
		);
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(1), 2, 9900)); // success in transfer of (10_000 - 100)
	});
}

/// Here,
/// 🧍 -> lock 100
/// 🧍 -> extend lock 99
///
/// take max(100, 99) as locked amount
#[test]
fn extend_lock_less_after_some_locked() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(1), 10_000);
		assert_ok!(LockableCurrency::lock_capital(RuntimeOrigin::signed(1), 100));
		System::assert_last_event(Event::Locked { user: 1, amount: 100 }.into());
		assert_ok!(LockableCurrency::extend_lock(RuntimeOrigin::signed(1), 99));
		System::assert_last_event(Event::ExtendedLock { user: 1, amount: 99 }.into());
		assert_eq!(Balances::free_balance(1), 10_000); // free_balance is still 10_000
		assert_noop!(
			Balances::transfer(RuntimeOrigin::signed(1), 2, 10_000), // fail in transfer of 10_000 free balance
			pallet_balances::Error::<Test, _>::LiquidityRestrictions
		);
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(1), 2, 9_900)); // success in transfer of (10_000 - 100)
	});
}

/// Here,
/// 🧍 -> lock 100
/// 🧍 -> extend lock 101
///
/// take max(100, 101) as locked amount
#[test]
fn extend_lock_more_after_some_locked() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(1), 10_000);
		assert_ok!(LockableCurrency::lock_capital(RuntimeOrigin::signed(1), 100));
		System::assert_last_event(Event::Locked { user: 1, amount: 100 }.into());
		assert_ok!(LockableCurrency::extend_lock(RuntimeOrigin::signed(1), 101));
		System::assert_last_event(Event::ExtendedLock { user: 1, amount: 101 }.into());
		assert_eq!(Balances::free_balance(1), 10_000); // free_balance is still 10_000
		assert_noop!(
			Balances::transfer(RuntimeOrigin::signed(1), 2, 10_000), // fail in transfer of 10_000 free balance
			pallet_balances::Error::<Test, _>::LiquidityRestrictions
		);
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(1), 2, 9_899)); // success in transfer of (10_000 - 101)
	});
}

//=====unlock_capital=====

/// Here, unlocked after no lock operation
/// 🧍 -> unlock_all
#[test]
fn unlocked_after_no_lock_op() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(1), 10_000);
		assert_ok!(LockableCurrency::unlock_all(RuntimeOrigin::signed(1)));
		System::assert_last_event(Event::Unlocked { user: 1 }.into());
		assert_eq!(Balances::free_balance(1), 10_000); // free_balance is still 10_000
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(1), 2, 10_000)); // success in transfer of 10_000 free balance
	});
}

/// Here,
/// 🧍 -> lock 0
/// 🧍 -> unlock_all
#[test]
fn unlocked_after_zero_locked() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(1), 10_000);
		assert_ok!(LockableCurrency::lock_capital(RuntimeOrigin::signed(1), 0));
		assert_ok!(LockableCurrency::unlock_all(RuntimeOrigin::signed(1)));
		System::assert_last_event(Event::Unlocked { user: 1 }.into());
		assert_eq!(Balances::free_balance(1), 10_000); // free_balance is still 10_000
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(1), 2, 10_000)); // success in transfer of 10_000 free balance
	});
}

/// Here,
/// 🧍 -> lock 100
/// 🧍 -> unlock_all
#[test]
fn unlocked_after_some_locked() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(1), 10_000);
		assert_ok!(LockableCurrency::lock_capital(RuntimeOrigin::signed(1), 100));
		assert_ok!(LockableCurrency::unlock_all(RuntimeOrigin::signed(1)));
		System::assert_last_event(Event::Unlocked { user: 1 }.into());
		assert_eq!(Balances::free_balance(1), 10_000); // free_balance is still 10_000
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(1), 2, 10_000)); // success in transfer of 10_000 free balance
	});
}

/// Here,
/// 🧍 -> lock 10_000 (all)
/// 🧍 -> unlock_all
#[test]
fn unlocked_after_all_locked() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(1), 10_000);
		assert_ok!(LockableCurrency::lock_capital(RuntimeOrigin::signed(1), 10_000));
		assert_ok!(LockableCurrency::unlock_all(RuntimeOrigin::signed(1)));
		System::assert_last_event(Event::Unlocked { user: 1 }.into());
		assert_eq!(Balances::free_balance(1), 10_000); // free_balance is still 10_000
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(1), 2, 10_000)); // success in transfer of 10_000 free balance
	});
}

/// Here,
/// 🧍 -> lock 100
/// 🧍 -> extend lock 100
/// 🧍 -> unlock_all
#[test]
fn unlocked_after_some_locked_and_then_extended_same() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(1), 10_000);
		assert_ok!(LockableCurrency::lock_capital(RuntimeOrigin::signed(1), 100));
		assert_ok!(LockableCurrency::extend_lock(RuntimeOrigin::signed(1), 100));
		assert_ok!(LockableCurrency::unlock_all(RuntimeOrigin::signed(1)));
		System::assert_last_event(Event::Unlocked { user: 1 }.into());
		assert_eq!(Balances::free_balance(1), 10_000); // free_balance is still 10_000
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(1), 2, 10_000)); // success in transfer of 10_000 free balance
	});
}

/// Here,
/// 🧍 -> lock 100
/// 🧍 -> extend lock 99
/// 🧍 -> unlock_all
#[test]
fn unlocked_after_some_locked_and_then_extended_less() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(1), 10_000);
		assert_ok!(LockableCurrency::lock_capital(RuntimeOrigin::signed(1), 100));
		assert_ok!(LockableCurrency::extend_lock(RuntimeOrigin::signed(1), 99));
		assert_ok!(LockableCurrency::unlock_all(RuntimeOrigin::signed(1)));
		System::assert_last_event(Event::Unlocked { user: 1 }.into());
		assert_eq!(Balances::free_balance(1), 10_000); // free_balance is still 10_000
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(1), 2, 10_000)); // success in transfer of 10_000 free balance
	});
}

/// Here,
/// 🧍 -> lock 100
/// 🧍 -> extend lock 101
/// 🧍 -> unlock_all
#[test]
fn unlocked_after_some_locked_and_then_extended_more() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(1), 10_000);
		assert_ok!(LockableCurrency::lock_capital(RuntimeOrigin::signed(1), 100));
		assert_ok!(LockableCurrency::extend_lock(RuntimeOrigin::signed(1), 101));
		assert_ok!(LockableCurrency::unlock_all(RuntimeOrigin::signed(1)));
		System::assert_last_event(Event::Unlocked { user: 1 }.into());
		assert_eq!(Balances::free_balance(1), 10_000); // free_balance is still 10_000
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(1), 2, 10_000)); // success in transfer of 10_000 free balance
	});
}
