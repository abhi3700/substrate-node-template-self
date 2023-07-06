use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok};

use sp_runtime::{
	DispatchError::{BadOrigin, Token},
	TokenError::Frozen,
};

// Block wise assumptions for corresponding time, assuming 1 BLOCK = 6 seconds
const ONE_DAY: u32 = 14_400;
const ONE_MONTH: u32 = 432_000;
const ONE_QUARTER_YEAR: u32 = 1_296_000;
const HALF_YEAR: u32 = 2_592_000;
const THREE_QUARTER_YEAR: u32 = 3_888_000;
const ONE_YEAR: u32 = 5_184_000;

const INTEREST: u32 = 8_000; // 8%
const SCALING_FACTOR: u32 = 100_000; // 100%
const FD_EPOCH: u32 = ONE_YEAR as u32; // 1 year
const PENALTY: u32 = 500; // 0.5%

//=====getters=====

#[test]
fn get_default_fd_params() {
	new_test_ext().execute_with(|| {
		assert_eq!(Bank::fd_params(), None);
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
		assert_eq!(Bank::fd_user_details(&ALICE), (0, 0));
		assert_eq!(Bank::fd_user_details(&BOB), (0, 0));
		assert_eq!(Bank::fd_user_details(&CHARLIE), (0, 0));
	});
}

//=====set_fd_interest_rate=====

// Bank -> 🏦 ✅
#[test]
fn only_root_can_set_fd_interest_rate() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bank::set_fd_interest_rate(
			RuntimeOrigin::root(),
			INTEREST,
			SCALING_FACTOR,
			FD_EPOCH,
			PENALTY
		));
		System::assert_last_event(
			Event::FDParamsSet {
				interest: INTEREST,
				scaling_factor: SCALING_FACTOR,
				fd_epoch: FD_EPOCH,
				penalty: PENALTY,
			}
			.into(),
		)
	});
}

// 🧍 -> 🏦 ❌
#[test]
fn others_cant_set_fd_interest_rate() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Bank::set_fd_interest_rate(
				RuntimeOrigin::signed(ALICE),
				INTEREST,
				SCALING_FACTOR,
				FD_EPOCH,
				PENALTY
			),
			BadOrigin
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
		assert_noop!(Bank::set_treasury(RuntimeOrigin::signed(ALICE), TREASURY), BadOrigin);
	});
}

//=====open_fd=====
#[test]
fn open_fd_fail_for_zero_amount() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Bank::open_fd(RuntimeOrigin::signed(ALICE), 0, ONE_YEAR),
			Error::<Test>::ZeroAmountWhenOpeningFD
		);
	});
}

#[test]
fn open_fd_fail_when_treasury_not_set() {
	new_test_ext().execute_with(|| {
		assert_eq!(Bank::treasury(), None);
		assert_noop!(
			Bank::open_fd(RuntimeOrigin::signed(ALICE), 100, ONE_YEAR),
			Error::<Test>::TreasuryNotSet
		);
	});
}

#[test]
fn open_fd_fail_when_interest_not_set() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));
		assert_eq!(Bank::fd_params(), None);
		assert_noop!(
			Bank::open_fd(RuntimeOrigin::signed(ALICE), 100, ONE_YEAR),
			Error::<Test>::FDParamsNotSet
		);
	});
}

#[test]
fn open_fd() {
	new_test_ext().execute_with(|| {
		// set interest details
		assert_ok!(Bank::set_fd_interest_rate(
			RuntimeOrigin::root(),
			INTEREST,
			SCALING_FACTOR,
			FD_EPOCH,
			PENALTY
		));

		// set treasury
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));

		// get the pre balance
		let pre_balance = Balances::free_balance(&ALICE);

		// get the FD id before opening FD
		let fd_id_pre = Bank::fd_user_details(&ALICE).0;

		// open fd
		assert_ok!(Bank::open_fd(RuntimeOrigin::signed(ALICE), 100, ONE_YEAR));
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
		let fd_id_post = Bank::fd_user_details(&ALICE).0;
		assert_eq!(fd_id_post - fd_id_pre, 1);
	});
}

//=====close_fd=====

#[test]
fn close_fd_fails_for_zero_id() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Bank::close_fd(RuntimeOrigin::signed(ALICE), 0, 1),
			Error::<Test>::ZeroIdWhenClosingFD
		);
	});
}

#[test]
fn close_fd_fails_when_fd_not_opened() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Bank::close_fd(RuntimeOrigin::signed(ALICE), 1, 1),
			Error::<Test>::FDNotExistsWithIdWhenClosingFD
		);
	});
}

#[test]
fn close_fd_fails_when_treasury_not_set() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bank::set_fd_interest_rate(
			RuntimeOrigin::root(),
			INTEREST,
			SCALING_FACTOR,
			FD_EPOCH,
			PENALTY
		));
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));
		assert_ok!(Bank::open_fd(RuntimeOrigin::signed(ALICE), 100, ONE_YEAR));

		assert_ok!(Bank::reset_treasury(RuntimeOrigin::root()));

		assert_eq!(Bank::treasury(), None);

		assert_noop!(
			Bank::close_fd(RuntimeOrigin::signed(ALICE), 1, 1),
			Error::<Test>::TreasuryNotSet
		);
	});
}

// 🧍--id--> 🏦 ❌
#[test]
fn close_fd_fails_for_invalid_user() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bank::set_fd_interest_rate(
			RuntimeOrigin::root(),
			INTEREST,
			SCALING_FACTOR,
			FD_EPOCH,
			PENALTY
		));
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));
		assert_ok!(Bank::open_fd(RuntimeOrigin::signed(ALICE), 100, ONE_YEAR));

		assert_noop!(
			Bank::close_fd(RuntimeOrigin::signed(BOB), 1, 1),
			Error::<Test>::FDNotExistsWithIdWhenClosingFD
		);
	});
}

// 🧍--penalty 💰--> [TREASURY]
// 🧍<--principal_amount 💰 (unreserved)-- 🏦
#[test]
fn close_fd_wo_maturity() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bank::set_fd_interest_rate(
			RuntimeOrigin::root(),
			INTEREST,
			SCALING_FACTOR,
			FD_EPOCH,
			PENALTY
		));

		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));
		assert_ok!(Bank::open_fd(RuntimeOrigin::signed(ALICE), 100, ONE_YEAR));

		// set the block number to (3/4)th year worth of blocks
		System::set_block_number(THREE_QUARTER_YEAR as u64);

		// get the pre balance
		let pre_balance = Balances::free_balance(&ALICE);

		// get the Treasury balance
		let treasury_balance_pre = Balances::free_balance(&TREASURY);

		let principal_amt: u128 = 100;

		// calculate the penalty
		let (_, scaling_factor, fd_epoch, penalty_rate) = Bank::get_fd_params();
		let mut penalty_amt = principal_amt
			.checked_mul(penalty_rate as u128)
			.and_then(|p| p.checked_div(scaling_factor as u128))
			.unwrap();
		if penalty_amt == 0 {
			penalty_amt = 1;
		}

		// close the FD w/o maturity
		assert_ok!(Bank::close_fd(RuntimeOrigin::signed(ALICE), 1, 0));
		System::assert_last_event(
			Event::FDClosed {
				maturity: false,
				user: ALICE,
				principal: principal_amt,
				interest: 0,
				penalty: penalty_amt,
				block: <frame_system::Pallet<Test>>::block_number(),
			}
			.into(),
		);

		// get the post balance
		let post_balance = Balances::free_balance(&ALICE);

		assert_eq!(
			post_balance - pre_balance,
			principal_amt.checked_sub(penalty_amt).unwrap() as u128
		);

		// get the Treasury balance
		let treasury_balance_post = Balances::free_balance(&TREASURY);

		assert_eq!(treasury_balance_post - treasury_balance_pre, penalty_amt as u128);
	});
}

// 🧍<--interest 💰-- [TREASURY]
// 🧍<--principal_amount 💰 (unreserved)-- 🏦
#[test]
fn close_fd_w_maturity() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bank::set_fd_interest_rate(
			RuntimeOrigin::root(),
			INTEREST,
			SCALING_FACTOR,
			FD_EPOCH,
			PENALTY
		));
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));
		assert_ok!(Bank::open_fd(RuntimeOrigin::signed(ALICE), 100, ONE_YEAR));

		let maturity_period = ONE_YEAR;

		// set the block number to 62
		System::set_block_number((maturity_period + 1) as u64);

		// get the pre balance
		let pre_balance = Balances::free_balance(&ALICE);

		// get the treasury pre balance
		let treasury_pre_balance = Balances::free_balance(&TREASURY);

		let principal_amt: u128 = 100;

		// calculate the interest
		let (interest_rate, scaling_factor, fd_epoch, _) = Bank::get_fd_params();
		let interest_amt = principal_amt
			.checked_mul(interest_rate as u128)
			.and_then(|i| i.checked_mul(maturity_period as u128))
			.and_then(|i| i.checked_div(scaling_factor as u128))
			.and_then(|i| i.checked_div(fd_epoch as u128))
			.unwrap();

		// close fd w maturity
		assert_ok!(Bank::close_fd(RuntimeOrigin::signed(ALICE), 1, 1));
		System::assert_last_event(
			Event::FDClosed {
				maturity: true,
				user: ALICE,
				principal: 100,
				interest: interest_amt,
				penalty: 0,
				block: <frame_system::Pallet<Test>>::block_number(),
			}
			.into(),
		);

		// get the post balance
		let post_balance = Balances::free_balance(&ALICE);

		// check the post balance if increased by the FD amount
		assert_eq!(post_balance - pre_balance, 100 + interest_amt);
		// assert!(post_balance > pre_balance);

		// check the reserved balance of user is zero
		assert_eq!(Balances::reserved_balance(&ALICE), 0);

		// check the treasury post balance if increased by the interest
		let treasury_post_balance = Balances::free_balance(&TREASURY);
		assert_eq!(treasury_pre_balance - treasury_post_balance, interest_amt);
	});
}

//=====lock_for_membership=====

/// 🧍 -> lock 0 (≤ min., < free) ❌
/// 🧍 -> lock 10 (≤ min., < free) ❌
#[test]
fn fails_when_lock_less_for_membership() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(&ALICE), 10_000);
		assert_noop!(
			Bank::lock_for_membership(RuntimeOrigin::signed(ALICE), 0),
			Error::<Test>::LockAmountIsLessThanMinLockAmount
		);

		assert_noop!(
			Bank::lock_for_membership(RuntimeOrigin::signed(ALICE), 19),
			Error::<Test>::LockAmountIsLessThanMinLockAmount
		);
		assert_eq!(Balances::free_balance(&ALICE), 10_000); // no change
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(ALICE), BOB, 10_000)); // transfer 10_000 (all)
	});
}

/// 🧍 -> lock 100_001 (≥ max., > free) ❌
/// 🧍 -> lock u128::MAX (≥ max., > free) ❌
#[test]
fn fails_when_lock_more_for_membership() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(&ALICE), 10_000);
		assert_noop!(
			Bank::lock_for_membership(RuntimeOrigin::signed(ALICE), 100_001),
			Error::<Test>::LockAmountExceedsMaxLockAmount
		);

		assert_noop!(
			Bank::lock_for_membership(RuntimeOrigin::signed(ALICE), u128::MAX),
			Error::<Test>::LockAmountExceedsMaxLockAmount
		);
		assert_eq!(Balances::free_balance(&ALICE), 10_000); // no change
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(ALICE), BOB, 10_000)); // transfer 10_000 (all)
	});
}

/// 🧍 -> lock 21 (≥ min., < free) ✅
/// 🧍 -> lock 100_000 (≤ max., > free) ✅
#[test]
fn lock_valid_amt_for_membership() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(&ALICE), 10_000);
		assert_ok!(Bank::lock_for_membership(RuntimeOrigin::signed(ALICE), 21));
		System::assert_last_event(
			Event::LockedForMembership { user: ALICE, amount: 21, block: System::block_number() }
				.into(),
		);

		assert_ok!(Bank::lock_for_membership(RuntimeOrigin::signed(ALICE), 100_000));
		System::assert_last_event(
			Event::LockedForMembership {
				user: ALICE,
				amount: 100_000,
				block: System::block_number(),
			}
			.into(),
		);
		assert_eq!(Balances::free_balance(&ALICE), 10_000); // no change
		assert_noop!(Balances::transfer(RuntimeOrigin::signed(ALICE), BOB, 10_000), Token(Frozen));
		// transfer 10_000 (all)
	});
}

//=====unlock=====
#[test]
fn unlock() {}
