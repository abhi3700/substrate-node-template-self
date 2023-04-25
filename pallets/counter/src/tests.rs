use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok};

// ======set_value=====
#[test]
fn fails_when_value_set_as_zero() {
	new_test_ext().execute_with(|| {
		assert_eq!(Counter::count(), None); // ensuring the value is not set
		assert_noop!(Counter::set(RuntimeOrigin::signed(1), 0), Error::<Test>::InvalidInputValue);
	});
}

#[test]
fn succeeds_when_value_set_as_non_zero() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Counter::set(RuntimeOrigin::signed(1), 10));
		assert_eq!(Counter::count(), Some(10));
		System::assert_last_event(Event::ValueStored { value: 10, who: 1 }.into());
	});
}

#[test]
fn fails_when_value_is_set_twice() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Counter::set(RuntimeOrigin::signed(1), 10));
		assert_eq!(Counter::count(), Some(10));
		System::assert_last_event(Event::ValueStored { value: 10, who: 1 }.into());

		// fails when set twice
		assert_noop!(Counter::set(RuntimeOrigin::signed(1), 20), Error::<Test>::ValueAlreadyStored);
		assert_eq!(Counter::count(), Some(10));
	});
}

// ======increment======
#[test]
fn fails_when_notset_value_incremented() {
	new_test_ext().execute_with(|| {
		assert_eq!(Counter::count(), None); // ensuring the value is not set
		assert_noop!(
			Counter::increment(RuntimeOrigin::signed(1), 5),
			Error::<Test>::NoneValueStored
		);
	});
}

#[test]
fn fails_when_value_incremented_by_zero() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Counter::increment(RuntimeOrigin::signed(1), 0),
			Error::<Test>::InvalidInputValue
		);
	});
}

#[test]
fn fails_when_max_value_incremented() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Counter::set(RuntimeOrigin::signed(1), u32::MAX));
		assert_eq!(Counter::count(), Some(u32::MAX));
		System::assert_last_event(Event::ValueStored { value: u32::MAX, who: 1 }.into());

		// fails when the max u32 value is incremented => Arithmetic value
		assert_noop!(
			Counter::increment(RuntimeOrigin::signed(1), 1),
			Error::<Test>::StorageOverflow
		);
	});
}

#[test]
fn succeeds_when_alreadyset_value_incremented() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Counter::set(RuntimeOrigin::signed(1), 10));
		assert_eq!(Counter::count(), Some(10));
		System::assert_last_event(Event::ValueStored { value: 10, who: 1 }.into());

		assert_ok!(Counter::increment(RuntimeOrigin::signed(1), 5));
		assert_eq!(Counter::count(), Some(15));
		System::assert_last_event(Event::ValueIncremented { old: 10, new: 15, who: 1 }.into());
	});
}

// ======decrement======
#[test]
fn fails_when_notset_value_decremented() {
	new_test_ext().execute_with(|| {
		assert_eq!(Counter::count(), None); // ensuring the value is not set
		assert_noop!(
			Counter::decrement(RuntimeOrigin::signed(1), 5),
			Error::<Test>::NoneValueStored
		);
	});
}

#[test]
fn fails_when_value_decremented_by_zero() {
	new_test_ext().execute_with(|| {
		assert_eq!(Counter::count(), None); // ensuring the value is not set
		assert_noop!(
			Counter::decrement(RuntimeOrigin::signed(1), 0),
			Error::<Test>::InvalidInputValue
		);
	});
}

#[test]
fn fails_when_min_value_decremented() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		// NOTE: as the min. value '0' can't be set in this pallet logic. So, setting '1' &
		// then decrementing by '2' to cause arithmetic overflow.
		assert_ok!(Counter::set(RuntimeOrigin::signed(1), 1));
		assert_eq!(Counter::count(), Some(1));
		System::assert_last_event(Event::ValueStored { value: 1, who: 1 }.into());

		// fails when the min u32 value is decremented => Arithmetic value
		assert_noop!(
			Counter::decrement(RuntimeOrigin::signed(1), 2),
			Error::<Test>::StorageOverflow
		);
	});
}

#[test]
fn succeeds_when_alreadyset_value_decremented() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Counter::set(RuntimeOrigin::signed(1), 10));
		assert_eq!(Counter::count(), Some(10));
		System::assert_last_event(Event::ValueStored { value: 10, who: 1 }.into());

		assert_ok!(Counter::decrement(RuntimeOrigin::signed(1), 5));
		assert_eq!(Counter::count(), Some(5));
		System::assert_last_event(Event::ValueDecremented { old: 10, new: 5, who: 1 }.into());
	});
}

// ======reset======
#[test]
fn reset_fails_when_value_notset() {
	new_test_ext().execute_with(|| {
		assert_eq!(Counter::count(), None);
		assert_noop!(Counter::reset(RuntimeOrigin::signed(1)), Error::<Test>::NoneValueStored);
	});
}

#[test]
fn reset_succeeds_when_nonzero_value_stored() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Counter::set(RuntimeOrigin::signed(1), 1));
		assert_eq!(Counter::count(), Some(1));
		System::assert_last_event(Event::ValueStored { value: 1, who: 1 }.into());

		assert_ok!(Counter::reset(RuntimeOrigin::signed(1)));
		assert_eq!(Counter::count(), Some(0));
	})
}

/// reset fails when the stored value is zero i.e. just reset it twice to fail
#[test]
fn reset_fails_when_reset_twice() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Counter::set(RuntimeOrigin::signed(1), 1));
		assert_eq!(Counter::count(), Some(1));
		System::assert_last_event(Event::ValueStored { value: 1, who: 1 }.into());

		assert_ok!(Counter::reset(RuntimeOrigin::signed(1)));
		assert_eq!(Counter::count(), Some(0));
		System::assert_last_event(Event::ValueReset { old: 1, who: 1 }.into());

		// reset again i.e. already zero value
		assert_noop!(Counter::reset(RuntimeOrigin::signed(1)), Error::<Test>::ZeroValueStored);
	});
}
