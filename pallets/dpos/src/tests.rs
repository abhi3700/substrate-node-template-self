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
const ONE_DAY: u32 = 14_400;
const ONE_MONTH: u32 = 432_000;
const ONE_QUARTER_YEAR: u32 = 1_296_000;
const HALF_YEAR: u32 = 2_592_000;
const THREE_QUARTER_YEAR: u32 = 3_888_000;
const ONE_YEAR: u32 = 5_184_000;

// ===== helpers =====

// ===== getters =====

// ===== setters =====
