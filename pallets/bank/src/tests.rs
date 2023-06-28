//! # Tests for the lockable-currency pallet.
//!
//! NOTE: Locking is validated based on success/failure of transfer of funds
//! from one account to another.

use crate::{mock::*, /* Error, */ Event};
use frame_support::{assert_noop, assert_ok};

//=====set_fd_interest_rate=====

#[test]
fn only_root_can_set_fd_interest_rate() {}

#[test]
fn others_cant_set_fd_interest_rate() {}

//=====set_fd_blocks_limit=====

#[test]
fn only_root_can_set_fd_blocks_limit() {}

#[test]
fn others_cant_set_fd_blocks_limit() {}

//=====open_fd=====
#[test]
fn open_fd() {}

//=====close_fd=====
#[test]
fn close_fd_before_blocks_limit() {}

#[test]
fn close_fd_after_blocks_limit() {}

//=====lock_for_dao=====

#[test]
fn lock_zero_for_dao() {}

#[test]
fn lock_less_than_fbal_for_dao() {}

#[test]
fn lock_more_than_fbal_for_dao() {}

//=====unlock=====
#[test]
fn unlock() {}
