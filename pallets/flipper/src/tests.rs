use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok};

// ======set_value=====
#[test]
fn succeeds_when_value_set_as_true() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		// Dispatch a signed extrinsic.
		assert_ok!(Flipper::set_value(RuntimeOrigin::signed(1), true));
		// Read pallet storage and assert an expected result.
		assert_eq!(Flipper::value(), Some(true));
		// Assert that the correct event was deposited
		System::assert_last_event(Event::ValueSet { value: true, who: 1 }.into());
	});
}

// OPTIONAL
#[test]
fn succeeds_when_value_set_as_false() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		// Dispatch a signed extrinsic.
		assert_ok!(Flipper::set_value(RuntimeOrigin::signed(1), false));
		// Read pallet storage and assert an expected result.
		assert_eq!(Flipper::value(), Some(false));
		// Assert that the correct event was deposited
		System::assert_last_event(Event::ValueSet { value: false, who: 1 }.into());
	});
}

#[test]
fn fails_when_already_set_value_is_set() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		// Dispatch a signed extrinsic.
		assert_ok!(Flipper::set_value(RuntimeOrigin::signed(1), true));
		// Read pallet storage and assert an expected result.
		assert_eq!(Flipper::value(), Some(true));
		// Assert that the correct event was deposited
		System::assert_last_event(Event::ValueSet { value: true, who: 1 }.into());

		// fails when trying to set the value (true/false) when already set.
		assert_noop!(
			Flipper::set_value(RuntimeOrigin::signed(1), false),
			Error::<Test>::AlreadySet
		);
		// Read pallet storage and assert an expected result.
		assert_eq!(Flipper::value(), Some(true));
	});
}

// ======flip_value=====
#[test]
fn fails_when_value_not_set_is_flipped() {
	new_test_ext().execute_with(|| {
		// fails when trying to flip value that is not set yet
		assert_noop!(Flipper::flip_value(RuntimeOrigin::signed(1)), Error::<Test>::NoneSet);
	});
}

#[test]
fn succeeds_when_value_is_flipped() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		let val = true;

		// Dispatch `set_value` extrinsic
		assert_ok!(Flipper::set_value(RuntimeOrigin::signed(1), val));
		assert_eq!(Flipper::value(), Some(val));
		System::assert_last_event(Event::ValueSet { value: val, who: 1 }.into());

		// Dispatch `flip_value` extrinsic
		assert_ok!(Flipper::flip_value(RuntimeOrigin::signed(1)));
		assert_eq!(Flipper::value(), Some(!val));
		// Assert that the correct event is emitted
		System::assert_last_event(Event::ValueFlipped { new: !val, who: 1 }.into());
	});
}
