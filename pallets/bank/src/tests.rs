use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok, sp_runtime::Permill};

use sp_runtime::{
	traits::{checked_pow, CheckedAdd, CheckedMul, CheckedSub},
	DispatchError::{BadOrigin, Token},
	FixedU128,
	TokenError::Frozen,
};

// suppress warnings for declared variables, but not used.
// Block wise assumptions for corresponding time, assuming 1 BLOCK = 6 seconds
const _ONE_DAY: u32 = 14_400;
const _ONE_MONTH: u32 = 432_000;
const _ONE_QUARTER_YEAR: u32 = 1_296_000;
const _HALF_YEAR: u32 = 2_592_000;
const THREE_QUARTER_YEAR: u32 = 3_888_000;
const ONE_YEAR: u32 = 5_184_000;

// TODO: Create a macro that takes in the following parameters and creates a FD with those parameters.
// The objective is to create multiple FDs with different parameters and test them with different users.
// During assertions, we can check the maturity amount with the formula and check the maturity amount with the actual value (computed one).
const PRINCIPAL_AMOUNT: Balance = 1e10 as u128 * 5000; // representing 5000$ in 1e10 units (as decimals)
const INTEREST_RATE: Permill = Permill::from_percent(2); // 2%	or Permill::from_parts(20_000)
const PENALTY_RATE: Permill = Permill::from_parts(5_000); // 0.5%, NOTE: can't represent 0.5 inside parenthesis.
const COMPOUND_FREQUENCY: u16 = 1; // 1 time per fd_epoch (1 year)
const FD_EPOCH: u32 = ONE_YEAR; // 1 year
const MATURITY_PERIOD: u32 = 3 * ONE_YEAR; // 3 years

//=====getters=====

/// NOTE: this function is to check the Compound Interest Formula before inserting into the pallet (src/lib.rs)
/// Why is this function not in `lib.rs` file? Because in `lib.rs`, the suggestions are not working properly in terms
/// of speed & accuracy. Hence, this function is created to check the formula.
///
// #[test]
#[allow(dead_code)]
fn get_maturity_amt_in_compound_interest() {
	let interest_rate_in_percent = INTEREST_RATE.deconstruct();
	println!("interest_rate_in_percent: {:?}", interest_rate_in_percent); // interest_rate_in_percent: 20000

	// r/n
	// Source: https://substrate.stackexchange.com/questions/680/from-float-function-or-associated-item-not-found-in-fixedu128
	// let k = FixedU128::from_float(interest_rate_in_percent as f64 / 1000000f64);	// ❌ can't work in `lib.rs` file.
	// here, inside from_inner, we are multiplying the previous value as 🔝 with 1e18. Hence, 1e12 is used.
	let k = FixedU128::from_inner(interest_rate_in_percent as u128 * 1e12 as u128);
	println!("k: {:?}", k); // k: FixedU128(0.020000000000000000)

	// 1 + r/n
	let l = FixedU128::from(1).checked_add(&k).unwrap();
	println!("l: {:?}", l); // l: FixedU128(1.020000000000000000)

	// n * t
	let compound_frequency_u32 = COMPOUND_FREQUENCY as u32;
	let nt = compound_frequency_u32 * MATURITY_PERIOD / FD_EPOCH;
	println!("nt: {:?}", nt); // nt: 3

	// (1 + r/n) ^ (n * t)
	let cp = checked_pow(l, nt as usize).unwrap();
	println!("m: {:?}", cp); // cp: FixedU128(1.061208000000000000)
	let cp_minus_one = cp.checked_sub(&FixedU128::from_u32(1)).unwrap_or_default();
	println!("cp-1: {:?}", cp_minus_one);
	let cp_minus_one_u128 = cp_minus_one.into_inner();
	println!("cp-1_u128: {:?}", cp_minus_one_u128);

	let p_fixedu128 = FixedU128::from(PRINCIPAL_AMOUNT);
	println!("p_fixedu128: {:?}", p_fixedu128); // p_fixedu128: FixedU128(50000000000000.000000000000000000)

	// p * (1 + r/n) ^ (n * t)
	let ma = p_fixedu128.checked_mul(&cp).unwrap();
	println!("ma: {:?}", ma); // ma: FixedU128(53060400000000.000000000000000000)

	let ma_inner = ma.into_inner();
	println!("ma_inner: {:?}", ma.into_inner()); // ma_inner: 53060400000000000000000000000000

	let ma_actual = ma_inner / 1e18 as u128;
	println!("ma_actual: {:?}", ma_actual); // ma_actual: 53060400000000
}

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

//=====set_fd_params=====

// Bank -> 🏦 ✅
#[test]
fn only_root_can_set_fd_params() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bank::set_fd_params(
			RuntimeOrigin::root(),
			INTEREST_RATE,
			PENALTY_RATE,
			COMPOUND_FREQUENCY,
			FD_EPOCH,
		));
		System::assert_last_event(
			Event::FDParamsSet {
				interest_rate: INTEREST_RATE,
				penalty_rate: PENALTY_RATE,
				fd_epoch: FD_EPOCH,
			}
			.into(),
		)
	});
}

// 🧍 -> 🏦 ❌
#[test]
fn others_cant_set_fd_params() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Bank::set_fd_params(
				RuntimeOrigin::signed(ALICE),
				INTEREST_RATE,
				PENALTY_RATE,
				COMPOUND_FREQUENCY,
				FD_EPOCH,
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
			Event::TreasurySet { account: TREASURY, block_num: System::block_number() }.into(),
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
			Bank::open_fd(RuntimeOrigin::signed(ALICE), 0, MATURITY_PERIOD),
			Error::<Test>::ZeroAmountWhenOpeningFD
		);
	});
}

#[test]
fn open_fd_fail_when_amount_less_than_min_fd_amt() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Bank::open_fd(RuntimeOrigin::signed(ALICE), MinFDAmount::get() - 1, MATURITY_PERIOD),
			Error::<Test>::FDAmountOutOfRangeWhenOpening
		);
	});
}

#[test]
fn open_fd_fail_when_amount_more_than_max_fd_amt() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Bank::open_fd(RuntimeOrigin::signed(ALICE), MaxFDAmount::get() + 1, MATURITY_PERIOD),
			Error::<Test>::FDAmountOutOfRangeWhenOpening
		);
	});
}

#[test]
fn open_fd_fail_when_treasury_not_set() {
	new_test_ext().execute_with(|| {
		assert_eq!(Bank::treasury(), None);
		assert_noop!(
			Bank::open_fd(RuntimeOrigin::signed(ALICE), PRINCIPAL_AMOUNT, MATURITY_PERIOD),
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
			Bank::open_fd(RuntimeOrigin::signed(ALICE), PRINCIPAL_AMOUNT, MATURITY_PERIOD),
			Error::<Test>::FDParamsNotSet
		);
	});
}

#[test]
fn open_fd_fail_when_zero_maturity_period() {
	new_test_ext().execute_with(|| {
		// set interest details
		assert_ok!(Bank::set_fd_params(
			RuntimeOrigin::root(),
			INTEREST_RATE,
			PENALTY_RATE,
			COMPOUND_FREQUENCY,
			FD_EPOCH,
		));
		assert_eq!(Bank::fd_params().is_some(), true);

		// set treasury
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));
		assert_eq!(Bank::treasury().is_some(), true);
		assert_noop!(
			Bank::open_fd(RuntimeOrigin::signed(ALICE), PRINCIPAL_AMOUNT, 0),
			Error::<Test>::FDMaturityPeriodOutOfRangeWhenOpening
		);
	});
}

#[test]
fn open_fd_fail_when_maturity_period_less_than_fd_epoch() {
	new_test_ext().execute_with(|| {
		// set interest details
		assert_ok!(Bank::set_fd_params(
			RuntimeOrigin::root(),
			INTEREST_RATE,
			PENALTY_RATE,
			COMPOUND_FREQUENCY,
			FD_EPOCH,
		));
		assert_eq!(Bank::fd_params().is_some(), true);

		// set treasury
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));
		assert_eq!(Bank::treasury().is_some(), true);
		assert_noop!(
			Bank::open_fd(RuntimeOrigin::signed(ALICE), PRINCIPAL_AMOUNT, FD_EPOCH - 1),
			Error::<Test>::FDMaturityPeriodOutOfRangeWhenOpening
		);
	});
}

#[test]
fn open_fd_fail_when_maturity_period_more_than_max_maturity_period() {
	new_test_ext().execute_with(|| {
		// set interest details
		assert_ok!(Bank::set_fd_params(
			RuntimeOrigin::root(),
			INTEREST_RATE,
			PENALTY_RATE,
			COMPOUND_FREQUENCY,
			FD_EPOCH,
		));
		assert_eq!(Bank::fd_params().is_some(), true);

		// set treasury
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));
		assert_eq!(Bank::treasury().is_some(), true);
		assert_noop!(
			Bank::open_fd(
				RuntimeOrigin::signed(ALICE),
				PRINCIPAL_AMOUNT,
				MaxFDMaturityPeriod::get() + 1
			),
			Error::<Test>::FDMaturityPeriodOutOfRangeWhenOpening
		);
	});
}

#[test]
fn open_fd() {
	new_test_ext().execute_with(|| {
		// set interest details
		assert_ok!(Bank::set_fd_params(
			RuntimeOrigin::root(),
			INTEREST_RATE,
			PENALTY_RATE,
			COMPOUND_FREQUENCY,
			FD_EPOCH,
		));

		// set treasury
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));

		// get the pre balance
		let pre_balance = Balances::free_balance(&ALICE);

		// get the FD id before opening FD
		let fd_id_pre = Bank::fd_user_details(&ALICE).0;

		// open fd
		assert_ok!(Bank::open_fd(RuntimeOrigin::signed(ALICE), PRINCIPAL_AMOUNT, MATURITY_PERIOD));
		System::assert_last_event(
			Event::FDOpened {
				user: ALICE,
				amount: PRINCIPAL_AMOUNT,
				block: System::block_number(),
			}
			.into(),
		);

		// get the post balance
		let post_balance = Balances::free_balance(&ALICE);

		// check the post balance if decreased by the FD amount
		assert_eq!(pre_balance - post_balance, PRINCIPAL_AMOUNT);

		// check the reserved balance of user is the FD amount
		assert_eq!(Balances::reserved_balance(&ALICE), PRINCIPAL_AMOUNT);

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
		assert_ok!(Bank::set_fd_params(
			RuntimeOrigin::root(),
			INTEREST_RATE,
			PENALTY_RATE,
			COMPOUND_FREQUENCY,
			FD_EPOCH,
		));
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));
		assert_ok!(Bank::open_fd(RuntimeOrigin::signed(ALICE), PRINCIPAL_AMOUNT, MATURITY_PERIOD));

		Bank::reset_treasury();

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
		assert_ok!(Bank::set_fd_params(
			RuntimeOrigin::root(),
			INTEREST_RATE,
			PENALTY_RATE,
			COMPOUND_FREQUENCY,
			FD_EPOCH,
		));
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));
		assert_ok!(Bank::open_fd(RuntimeOrigin::signed(ALICE), PRINCIPAL_AMOUNT, MATURITY_PERIOD));

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
		assert_ok!(Bank::set_fd_params(
			RuntimeOrigin::root(),
			INTEREST_RATE,
			PENALTY_RATE,
			COMPOUND_FREQUENCY,
			FD_EPOCH,
		));

		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));
		assert_ok!(Bank::open_fd(RuntimeOrigin::signed(ALICE), PRINCIPAL_AMOUNT, MATURITY_PERIOD));

		// set the block number to (3/4)th year worth of blocks
		System::set_block_number(THREE_QUARTER_YEAR as u64);

		// get the pre balance
		let pre_balance = Balances::free_balance(&ALICE);

		// get the Treasury balance
		let treasury_balance_pre = Balances::free_balance(&TREASURY);

		let principal_amt: u128 = PRINCIPAL_AMOUNT;

		// calculate the penalty
		let (_, penalty_rate, _, _) = Bank::get_fd_params();
		let mut penalty_amt = penalty_rate * principal_amt;
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
				block: System::block_number(),
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
		assert_ok!(Bank::set_fd_params(
			RuntimeOrigin::root(),
			INTEREST_RATE,
			PENALTY_RATE,
			COMPOUND_FREQUENCY,
			FD_EPOCH,
		));
		assert_ok!(Bank::set_treasury(RuntimeOrigin::root(), TREASURY));
		assert_ok!(Bank::open_fd(RuntimeOrigin::signed(ALICE), PRINCIPAL_AMOUNT, MATURITY_PERIOD));

		// set the block number to post Maturity period
		System::set_block_number((MATURITY_PERIOD + 1) as u64);

		// get the pre balance
		let pre_balance = Balances::free_balance(&ALICE);

		// get the treasury pre balance
		let treasury_pre_balance = Balances::free_balance(&TREASURY);

		// calculate the interest
		let (interest_rate, _, compound_frequency, fd_epoch) = Bank::get_fd_params();
		// get simple interest
		// let annual_interest_amt = interest_rate * PRINCIPAL_AMOUNT;
		// let tot_interest_amt = annual_interest_amt
		// 	.checked_mul(MATURITY_PERIOD as u128)
		// 	.and_then(|i| i.checked_div(fd_epoch as u128))
		// 	.unwrap();
		let tot_interest_amt = Bank::get_compound_interest(
			PRINCIPAL_AMOUNT,
			interest_rate,
			compound_frequency,
			fd_epoch,
			MATURITY_PERIOD,
		)
		.ok()
		.unwrap();

		// println!("tot_interest_amt: {:?}", tot_interest_amt);

		// close fd w maturity
		assert_ok!(Bank::close_fd(RuntimeOrigin::signed(ALICE), 1, 1));
		System::assert_last_event(
			Event::FDClosed {
				maturity: true,
				user: ALICE,
				principal: PRINCIPAL_AMOUNT,
				interest: tot_interest_amt,
				penalty: 0,
				block: System::block_number(),
			}
			.into(),
		);

		// get the post balance
		let post_balance = Balances::free_balance(&ALICE);

		// TODO: check the post balance if increased by the FD amount
		// assert_eq!(post_balance - pre_balance, PRINCIPAL_AMOUNT + tot_interest_amt);
		assert!(post_balance > pre_balance);

		// check the reserved balance of user is zero
		assert_eq!(Balances::reserved_balance(&ALICE), 0);

		// TODO: check the treasury post balance if increased by the interest
		let treasury_post_balance = Balances::free_balance(&TREASURY);
		assert_eq!(treasury_pre_balance - treasury_post_balance, tot_interest_amt);
	});
}

//=====lock_for_membership=====

/// 🧍 -> lock 0 (≤ min., < free) ❌
/// 🧍 -> lock 10 (≤ min., < free) ❌
#[test]
fn fails_when_lock_less_for_membership() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(&ALICE), 10_000 * 1e10 as Balance);
		assert_noop!(
			Bank::lock_for_membership(RuntimeOrigin::signed(ALICE), 0),
			Error::<Test>::LockAmountIsLessThanMinLockAmount
		);

		assert_noop!(
			Bank::lock_for_membership(RuntimeOrigin::signed(ALICE), 19 * 1e10 as Balance),
			Error::<Test>::LockAmountIsLessThanMinLockAmount
		);
		assert_eq!(Balances::free_balance(&ALICE), 10_000 * 1e10 as u128); // no change
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(ALICE), BOB, 10_000 * 1e10 as u128));
		// transfer 10_000 (all)
	});
}

/// 🧍 -> lock 100_001 (≥ max., > free) ❌
/// 🧍 -> lock u128::MAX (≥ max., > free) ❌
#[test]
fn fails_when_lock_more_for_membership() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(&ALICE), 10_000 * 1e10 as u128);
		assert_noop!(
			Bank::lock_for_membership(RuntimeOrigin::signed(ALICE), 100_001 * 1e10 as u128),
			Error::<Test>::LockAmountExceedsMaxLockAmount
		);

		assert_noop!(
			Bank::lock_for_membership(RuntimeOrigin::signed(ALICE), u128::MAX),
			Error::<Test>::LockAmountExceedsMaxLockAmount
		);
		assert_eq!(Balances::free_balance(&ALICE), 10_000 * 1e10 as u128); // no change
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(ALICE), BOB, 10_000 * 1e10 as u128));
		// transfer 10_000 (all)
	});
}

/// 🧍 -> lock 21 (≥ min., < free) ✅
/// 🧍 -> lock 100_000 (≤ max., > free) ✅
#[test]
fn lock_valid_amt_for_membership() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(&ALICE), 10_000 * 1e10 as u128);
		assert_ok!(Bank::lock_for_membership(RuntimeOrigin::signed(ALICE), 21 * 1e10 as u128));
		System::assert_last_event(
			Event::LockedForMembership {
				user: ALICE,
				amount: 21 * 1e10 as Balance,
				block: System::block_number(),
			}
			.into(),
		);

		assert_ok!(Bank::lock_for_membership(RuntimeOrigin::signed(ALICE), 100_000 * 1e10 as u128));
		System::assert_last_event(
			Event::LockedForMembership {
				user: ALICE,
				amount: 100_000 * 1e10 as u128,
				block: System::block_number(),
			}
			.into(),
		);
		assert_eq!(Balances::free_balance(&ALICE), 10_000 * 1e10 as u128); // no change
		assert_noop!(
			Balances::transfer(RuntimeOrigin::signed(ALICE), BOB, 10_000 * 1e10 as u128),
			Token(Frozen)
		);
		// transfer 10_000 (all)
	});
}

//=====unlock=====
/// 🧍 -> lock 21 (≥ min., < free) ✅
/// 🧍 -> lock 100_000 (≤ max., > free) ✅
#[test]
fn unlock_works_when_locked_successfully() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(&ALICE), 10_000 * 1e10 as Balance);
		assert_ok!(Bank::lock_for_membership(RuntimeOrigin::signed(ALICE), 21 * 1e10 as Balance));
		System::assert_last_event(
			Event::LockedForMembership {
				user: ALICE,
				amount: 21 * 1e10 as Balance,
				block: System::block_number(),
			}
			.into(),
		);

		assert_ok!(Bank::unlock_for_membership(RuntimeOrigin::signed(ALICE)));
		System::assert_last_event(
			Event::UnlockedForMembership { user: ALICE, block: System::block_number() }.into(),
		);
		assert_eq!(Balances::free_balance(&ALICE), 10_000 * 1e10 as Balance); // no change
		assert_ok!(Balances::transfer(RuntimeOrigin::signed(ALICE), BOB, 10_000 * 1e10 as Balance));
		// transfer 10_000 (all)
	});
}
