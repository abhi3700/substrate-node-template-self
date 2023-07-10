use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok, sp_runtime::Permill};

#[test]
fn it_works_for_default_value() {
	new_test_ext().execute_with(|| {
		let expected_something = (10000 * 2 * 8 / 100) / 365;

		// Dispatch a signed extrinsic.
		assert_ok!(Arithmetic::do_something(
			RuntimeOrigin::signed(1),
			10000,
			Permill::from_parts(5_000), // 0.5%, can't be represented using `from_percent()`
			2
		));
		// Read pallet storage and assert an expected result.
		assert_eq!(Arithmetic::something(), Some(expected_something));
		// Assert that the correct event was deposited
		System::assert_last_event(
			Event::SomethingStored { something: expected_something, who: 1 }.into(),
		);
	});
}

#[test]
fn correct_error_for_none_value() {
	new_test_ext().execute_with(|| {
		// Ensure the expected error is thrown when no value is present.
		assert_noop!(Arithmetic::cause_error(RuntimeOrigin::signed(1)), Error::<Test>::NoneValue);
	});
}
