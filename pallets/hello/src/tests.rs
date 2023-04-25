use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok};

#[test]
fn succeeds_for_say_hello() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		// Dispatch a signed extrinsic.
		assert_ok!(Hello::say_hello(RuntimeOrigin::signed(1)));
		// Assert that the correct event was deposited
		System::assert_last_event(Event::SomeoneSaysHello { who: 1 }.into());
	});
}

#[test]
fn fails_for_wish_start_w_hello() {
	new_test_ext().execute_with(|| {
		// Ensure the expected error is thrown when no value is present.
		assert_noop!(
			Hello::say_any(RuntimeOrigin::signed(1), "hello".to_string()),
			Error::<Test>::HelloPrefixed
		);
	});
}

#[test]
fn succeeds_for_say_any() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		// Dispatch a signed extrinsic
		assert_ok!(Hello::say_any(RuntimeOrigin::signed(1), "Good morning!".to_string()));
		// Assert that the correct event was deposited
		System::assert_last_event(
			Event::SomeoneSaysAny { wish: "Good morning!".to_string(), who: 1 }.into(),
		);
	});
}
