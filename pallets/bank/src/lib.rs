//! # Bank Pallet
//!
//! A pallet for handling accounts and balances & different types of deposits based on
//!
//! - [`Config`]
//! - [`Call`]
//!
//! ## Overview
//!
//! Anyone can open FD (Fixed Deposit) by reserving some amount of currency.
//!
//! During the FD period, the reserved amount cannot be used that's why need to be freed from the `free_balance`.
//! In order to receive interest, FD can only be closed after the `MinFDPeriod` is elapsed, else the reserved amount is returned
//! to the user without any interest as per the premature withdrawal facility. The penalty (0.5-1%) is stored & set by the root origin.
//!
//! But, if the FD is closed after `MinFDPeriod`, then the reserved amount is returned to the user with
//! some interest. The interest is stored & set by the root origin.
//!
//! TODO:
//! - [ ] We can also add the functionality of auto_maturity of FDs using hooks.
//! - [ ] After every few blocks, some balance is transferred to the TREASURY account.
//! 	- L0 chain's inflation is transferred to the TREASURY account.
//!
//! The interest comes from a treasury 💎 account which is funded by the root origin.
//!
//! NOTE: The runtime must include the `Balances` pallet to handle the accounts and balances for your chain.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! TODO: set interest rate, blocks limit in the runtime's next immediate
//! block after the pallet deployment. Can be done by Root or Inherent or something else.
//!
//! - `set_fd_interest_rate`
//! - `set_treasury`
//! - `open_fd`
//! - `close_fd`
//! - `lock_for_membership`
//! - `unlock_for_membership`

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// TODO: add benchmarking & weights
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

/// The log target.
const TARGET: &'static str = "pallet_bank::close_fd";

#[frame_support::pallet]
pub mod pallet {

	use super::*;
	use frame_support::{
		log,
		pallet_prelude::*,
		sp_runtime::{
			traits::{CheckedDiv, CheckedMul, CheckedSub, One, Zero},
			DispatchError,
		},
		traits::{
			Currency, ExistenceRequirement::AllowDeath, LockIdentifier, LockableCurrency,
			NamedReservableCurrency, ReservableCurrency, WithdrawReasons,
		},
		Blake2_128Concat,
	};
	use frame_system::pallet_prelude::*;

	const ID1: LockIdentifier = *b"Invest__";

	type AccountOf<T> = <T as frame_system::Config>::AccountId; // optional
	type BalanceOf<T> = <<T as Config>::MyCurrency as Currency<AccountOf<T>>>::Balance;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
		/// MyCurrency type for this pallet. Here, we could have used `Currency` trait.
		/// But, we need to use `reserved_balance` function which is not available in `Currency` trait.
		/// That's why `ReservableCurrency` trait is used.
		type MyCurrency: ReservableCurrency<Self::AccountId>
			+ LockableCurrency<Self::AccountId>
			+ NamedReservableCurrency<Self::AccountId>;

		#[pallet::constant]
		type MinFDValue: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MaxFDValue: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MinLockValue: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MaxLockValue: Get<BalanceOf<Self>>;

		// in blocks
		#[pallet::constant]
		type MinFDPeriod: Get<u32>;
	}

	#[pallet::storage]
	#[pallet::getter(fn fd_params)]
	// in percentage i.e. 0.5% = 0.005 => represented as 1e5 (scaling_factor) => 500
	// NOTE: We can put this scaling factor as high as possible i.e. 1e18,
	// but then during division it would lose the precision. Hence, choose as small as possible.
	// Hence, keep the scaling_factor as low as possible.
	// make sure during arithmetic, you divide by 1e5
	//
	// E.g. If the interest rate is 0.005% per year, then the interest (in decimal * scaling_factor) is 0.005e5 = 500
	// If the interest rate is 10%, then the interest set here as (0.1 * 1e5) = 10_000
	//
	// (u32, u32, u32, u32) tuple represents (interest, scaling_factor, fd_epoch, penalty)
	// NOTE: type of `interest` has been kept as `u32` covering the whole range of possible percentages as high as 10%
	// & as low as 0.00001%
	// It is recommended to set the scaling factor 1e5 in general.
	// `fd_epoch` is the duration in blocks for which the interest is applicable like 8% per year. So, here 8% is the interest
	// & 1 year is the `fd_epoch`.
	pub type FDParams<T: Config> = StorageValue<_, (u32, u32, u32, u32)>;

	#[pallet::storage]
	#[pallet::getter(fn treasury)]
	// Treasury account.
	pub type Treasury<T: Config> = StorageValue<_, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn fd_user_details)]
	// last FD User IDs for each user, except 0
	// User --> (fd_user_last_id, investment_score)
	pub type FDUserDetails<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, (u32, u16), ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn fd_vault)]
	// NOTE: can also use `AccountOf<T>` instead of `T::AccountId` here.
	// user -> id -> (amount, opened_at_block_number, maturity_period_in_blocks)
	// NOTE: Normally, maturity_period is 5 years.
	pub type FDVaults<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		u32,
		(BalanceOf<T>, T::BlockNumber, u32),
		// ValuQuery, // optional
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Set FD Interest Rate & Scaling Factor
		FDParamsSet { interest: u32, scaling_factor: u32, fd_epoch: u32, penalty: u32 },

		/// Treasury account set
		TreasurySet { account: T::AccountId, block_num: T::BlockNumber },

		/// Treasury account reset
		TreasuryReset { block_num: T::BlockNumber },

		/// FD Opened
		FDOpened {
			user: T::AccountId, // can also use `AccountOf<T>`
			amount: BalanceOf<T>,
			block: T::BlockNumber,
		},

		/// FD Closed with/without maturity
		FDClosed {
			maturity: bool,
			user: T::AccountId, // can also use `AccountOf<T>`
			principal: BalanceOf<T>,
			interest: BalanceOf<T>,
			penalty: BalanceOf<T>,
			block: T::BlockNumber,
		},

		/// Locked for Membership
		LockedForMembership {
			user: T::AccountId, // can also use `AccountOf<T>`
			amount: BalanceOf<T>,
			block: T::BlockNumber,
		},

		/// Unlocked for Membership
		UnlockedForMembership {
			user: T::AccountId, // can also use `AccountOf<T>`
			block: T::BlockNumber,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Zero Interest Rate
		ZeroInterestRate,
		/// Zero Scaling Factor
		ZeroScalingFactor,
		/// Zero FD Epoch
		ZeroFDEpoch,
		/// Zero Penalty
		ZeroFDPenalty,
		/// Interest Not Set
		FDInterestNotSet,
		/// FD Params Not Set
		FDParamsNotSet,
		/// FD Block Duration Not Set
		MinFDPeriodNotSet,
		/// FD Vault Does Not Exist
		FDVaultDoesNotExist,
		/// FD Maturity Must Be Greater Than FD Epoch
		FDMaturityMustBeGreaterThanFDEpoch,
		/// Zero Amount When Opening FD
		ZeroAmountWhenOpeningFD,
		/// Insufficient Free Balance When Opening FD
		InsufficientFreeBalanceWhenOpeningFD,
		/// FD Already Exists With Id When Opening FD
		FDAlreadyExistsWithIdWhenOpeningFD,
		/// Invalid Maturity Status
		InvalidMaturityStatus,
		/// Zero Id When Closing FD
		ZeroIdWhenClosingFD,
		/// Insufficient Free Balance For Interest
		InsufficientFreeBalanceForInterest,
		/// Insufficient Free Balance For Penalty
		InsufficientFreeBalanceForPenalty,
		/// FD Does Not Exist With Id When Closing FD
		FDNotExistsWithIdWhenClosingFD,
		/// Invalid Close FD Combination
		InvalidCloseFDCombination,
		/// Treasury Not Set
		TreasuryNotSet,
		/// Arithmetic Underflow
		ArithmeticUnderflow,
		/// Lock Amount is Less Than Min Lock Amount
		LockAmountIsLessThanMinLockAmount,
		/// Lock Amount is Greater Than Max Lock Amount
		LockAmountExceedsMaxLockAmount,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set FD Interest Rate, Scaling Factor, Per_Duration (EPOCH)
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_fd_interest_rate())]
		pub fn set_fd_interest_rate(
			origin: OriginFor<T>,
			interest: u32,
			scaling_factor: u32,
			fd_epoch: u32,
			penalty: u32,
		) -> DispatchResult {
			// ensure the root origin signed
			ensure_root(origin)?;

			// ensure positive interest
			ensure!(interest > 0, Error::<T>::ZeroInterestRate);

			// ensure scaling factor is not zero
			ensure!(scaling_factor > 0, Error::<T>::ZeroScalingFactor);

			// ensure per duration is not zero
			ensure!(fd_epoch > 0, Error::<T>::ZeroFDEpoch);

			// ensure penalty is not zero
			ensure!(penalty > 0, Error::<T>::ZeroFDPenalty);

			// set the FD params
			FDParams::<T>::put((interest, scaling_factor, fd_epoch, penalty));

			// emit the event
			Self::deposit_event(Event::FDParamsSet { interest, scaling_factor, fd_epoch, penalty });

			Ok(())
		}

		/// Set Treasury account from where the interest will be paid.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::set_treasury())]
		pub fn set_treasury(origin: OriginFor<T>, treasury: T::AccountId) -> DispatchResult {
			// ensure the root origin signed
			ensure_root(origin)?;

			// set the treasury
			Treasury::<T>::put(&treasury);

			// emit the event
			Self::deposit_event(Event::TreasurySet {
				account: treasury.clone(),
				block_num: <frame_system::Pallet<T>>::block_number(),
			});

			Ok(())
		}

		/// Reset Treasury account from where the interest will be paid.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::reset_treasury())]
		pub fn reset_treasury(origin: OriginFor<T>) -> DispatchResult {
			// ensure the root origin signed
			ensure_root(origin)?;

			// check treasury is set
			ensure!(Treasury::<T>::get().is_some(), Error::<T>::TreasuryNotSet);

			// set the treasury
			Treasury::<T>::kill();

			// emit the event
			Self::deposit_event(Event::TreasuryReset {
				block_num: <frame_system::Pallet<T>>::block_number(),
			});

			Ok(())
		}

		/// Open FD
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::open_fd())]
		pub fn open_fd(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
			maturity_period: u32,
		) -> DispatchResult {
			// ensure signed origin
			let user = ensure_signed(origin)?;

			// ensure the amount is not zero
			ensure!(amount > Zero::zero(), Error::<T>::ZeroAmountWhenOpeningFD);

			// ensure the treasury is set
			ensure!(Treasury::<T>::get().is_some(), Error::<T>::TreasuryNotSet);

			// ensure the FD details set
			ensure!(<FDParams<T>>::exists(), Error::<T>::FDParamsNotSet);

			// ensure the maturity_period is greater than fd_epoch at least
			ensure!(
				maturity_period >= FDParams::<T>::get().unwrap().2,
				Error::<T>::FDMaturityMustBeGreaterThanFDEpoch
			);

			// get the next fd id for the user
			let (last_fd_id, last_investment_score) = FDUserDetails::<T>::get(&user);

			let next_fd_id = last_fd_id + 1;

			// ensure there is no FD with the id received [REDUNDANT]
			ensure!(
				!FDVaults::<T>::contains_key(&user, next_fd_id),
				Error::<T>::FDAlreadyExistsWithIdWhenOpeningFD
			);

			// NOTE: inherently checked for sufficient free balance
			// reserve the token as supposed to be deducted from free_balance.
			T::MyCurrency::reserve(&user, amount)?;

			let current_block_number = <frame_system::Pallet<T>>::block_number();

			// store the FD details in the storage for the user
			FDVaults::<T>::insert(
				&user,
				next_fd_id,
				(amount, current_block_number, maturity_period),
			);

			// update the next fd id for the user
			FDUserDetails::<T>::insert(&user, (next_fd_id, last_investment_score));

			// emit the event
			Self::deposit_event(Event::FDOpened { user, amount, block: current_block_number });

			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::close_fd())]
		pub fn close_fd(origin: OriginFor<T>, id: u32, has_matured: u8) -> DispatchResult {
			// ensure signed origin
			let user = ensure_signed(origin)?;

			// ensure the id is non-zero
			ensure!(id > 0, Error::<T>::ZeroIdWhenClosingFD);

			// ensure the maturity is either 0 (No) or 1 (Yes)
			ensure!(has_matured == 0 || has_matured == 1, Error::<T>::InvalidMaturityStatus);

			// get the FD vault details & check for the valid ID.
			let (principal_amount, block_num_opened_at, maturity_period) =
				FDVaults::<T>::get(&user, id).ok_or(Error::<T>::FDNotExistsWithIdWhenClosingFD)?;
			// println!(
			// 	"FD w Principal amount: {:?}, opened at block no.: {:?}, w maturity period: {:?} ",
			// 	principal_amount, block_num_opened_at, maturity_period
			// ); // for testing only

			// ensure there is a treasury account set & get that if exists
			let treasury = <Treasury<T>>::get().ok_or(Error::<T>::TreasuryNotSet)?;

			// get the interest if exists
			let (interest_rate, scaling_factor, fd_epoch, penalty_rate) =
				FDParams::<T>::get().ok_or(Error::<T>::FDInterestNotSet)?;

			// get the current block number
			let current_block_num = <frame_system::Pallet<T>>::block_number();

			// get the block difference if any
			let staked_duration = current_block_num
				.checked_sub(&block_num_opened_at)
				.ok_or(Error::<T>::ArithmeticUnderflow)?;
			// log::info!(target: TARGET, "Staked duration: {:?}", interest);

			// get the min_fd_period
			let _min_fd_period = T::MinFDPeriod::get();

			// Here, maturity_period is considered for calculation due to FD,
			// Otherwise, in case of RD, the staked_duration is considered for calculation, although it has lesser
			// interest rate than FD.
			if staked_duration >= maturity_period.into() && has_matured == 1 {
				// if the FD is open for min. duration i.e. `MinFDPeriod`, then calculate the interest
				// & transfer the (principal_amount + interest) from the treasury account to the FD holder;
				// else transfer the amount only from the treasury account to the caller
				// calculate the interest directly
				// let interest = principal_amount
				// 	.checked_mul(&interest_rate.into())
				// 	.and_then(|v| v.checked_mul(&maturity_period.into()))
				// 	.and_then(|v| v.checked_div(&fd_epoch.into()))
				// 	.and_then(|v| v.checked_div(&scaling_factor.into()))
				// 	.ok_or("Interest calculation failed")?;

				let interest = principal_amount
					.checked_mul(&Self::u32_to_balance(interest_rate).unwrap())
					.and_then(|v| v.checked_mul(&maturity_period.into()))
					.and_then(|v| v.checked_div(&fd_epoch.into()))
					.and_then(|v| v.checked_div(&scaling_factor.into()))
					.ok_or("Interest calculation failed")?;

				log::info!(target: TARGET, "Interest: {:?}", interest);
				// println!("Interest on post-mature withdrawal: {:?}", interest); // for testing only

				// check the treasury's free_balance is greater than the interest
				ensure!(
					T::MyCurrency::free_balance(&treasury) > interest,
					Error::<T>::InsufficientFreeBalanceForInterest
				);

				// TODO: Calculate the Investment Score (IS) for the user
				// Investment Score (IS) = 1000 * log10(1 + (A/D)), Here,
				// let investment_score = Self::calculate_investment_score(&user, &interest);

				// transfer the interest from the treasury account to the user
				let _ = T::MyCurrency::transfer(&treasury, &user, interest, AllowDeath);

				// remove the FD details from the storage for the user
				<FDVaults<T>>::remove(&user, id);

				// unreserve the principal_amount from the user
				T::MyCurrency::unreserve(&user, principal_amount);

				// emit the event
				Self::deposit_event(Event::FDClosed {
					maturity: true,
					user,
					principal: principal_amount,
					interest,
					penalty: Zero::zero(),
					block: current_block_num,
				});

				Ok(())
			} else if staked_duration < maturity_period.into() && has_matured == 0 {
				// calculate the penalty
				let mut penalty = principal_amount
					.checked_mul(&penalty_rate.into())
					.and_then(|v| v.checked_div(&scaling_factor.into()))
					.ok_or("Penalty calculation failed")?;

				if penalty == Zero::zero() {
					penalty = One::one();
				}
				log::info!(target: TARGET, "Penalty: {:?}", penalty); // for runtime debugging

				// println!("Penalty on pre-mature withdrawal: {:?}", penalty); // for testing only

				// check the user's free_balance is greater than the penalty
				ensure!(
					T::MyCurrency::free_balance(&user) > penalty,
					Error::<T>::InsufficientFreeBalanceForPenalty
				);

				// transfer the penalty from the user to the treasury account
				let _ = T::MyCurrency::transfer(&user, &treasury, penalty, AllowDeath);

				// remove the FD details from the storage for the user
				<FDVaults<T>>::remove(&user, id);

				// unreserve the principal_amount from the user
				T::MyCurrency::unreserve(&user, principal_amount);

				// emit the event
				Self::deposit_event(Event::FDClosed {
					maturity: false,
					user,
					principal: principal_amount,
					interest: Zero::zero(),
					penalty,
					block: current_block_num,
				});

				Ok(())
			} else {
				return Err(Error::<T>::InvalidCloseFDCombination.into());
			}
		}

		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::lock_for_membership())]
		pub fn lock_for_membership(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			// ensure signed origin
			let user = ensure_signed(origin)?;

			// ensure that the amount is at least min. lock amount
			ensure!(
				amount >= T::MinLockValue::get(),
				Error::<T>::LockAmountIsLessThanMinLockAmount
			);

			// ensure that the amount is < max lock amount
			ensure!(amount <= T::MaxLockValue::get(), Error::<T>::LockAmountExceedsMaxLockAmount);

			// lock amount
			T::MyCurrency::set_lock(ID1, &user, amount, WithdrawReasons::all());

			// emit the event
			Self::deposit_event(Event::LockedForMembership {
				user,
				amount,
				block: <frame_system::Pallet<T>>::block_number(),
			});

			Ok(())
		}

		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::unlock_for_membership())]
		pub fn unlock_for_membership(origin: OriginFor<T>) -> DispatchResult {
			// ensure signed origin
			let user = ensure_signed(origin)?;

			// unlock amount
			T::MyCurrency::remove_lock(ID1, &user);

			// emit the event
			Self::deposit_event(Event::UnlockedForMembership {
				user,
				block: <frame_system::Pallet<T>>::block_number(),
			});

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		//function to convert balance to u32
		pub fn balance_to_u32(input: BalanceOf<T>) -> Option<u32> {
			TryInto::<u32>::try_into(input).ok()
		}

		// NOTE: prefer this to avoid truncating during arithmetic operations
		pub fn u32_to_balance(input: u32) -> Option<BalanceOf<T>> {
			TryInto::<BalanceOf<T>>::try_into(input).ok()
		}

		pub fn get_fd_params() -> (u32, u32, u32, u32) {
			let (interest_rate, scaling_factor, fd_epoch, penalty_rate) =
				FDParams::<T>::get().unwrap();

			(interest_rate, scaling_factor, fd_epoch, penalty_rate)
		}

		// As per the plan the IS ∈ [0, 1000) following Log curve (increasing) ⎛
		pub fn get_investment_score(user: &T::AccountId) -> u16 {
			let (_, investment_score) = FDUserDetails::<T>::get(user);
			investment_score
		}

		// Get the FD Vault details of the user for the given FD id
		pub fn get_fd_vault_details(
			user: &T::AccountId,
			id: u32,
		) -> Result<(BalanceOf<T>, T::BlockNumber, u32), DispatchError> {
			let (principal_amount, opened_at_block_number, expiry_duration) =
				FDVaults::<T>::get(user, id).ok_or(Error::<T>::FDVaultDoesNotExist)?;
			Ok((principal_amount, opened_at_block_number, expiry_duration))
		}

		// TODO: Create public function for SDK use case
		// pub fn get_interest(principal_amount: BalanceOf<T>, interest_rate: u32, scaling_factor: u32, fd_epoch: u32, maturity_period: u32) {}

		// TODO: Create public function for SDK use case
		// pub fn get_penalty(principal_amount: BalanceOf<T>, penalty_rate: u32, scaling_factor: u32, fd_epoch: u32) {}
	}
}
