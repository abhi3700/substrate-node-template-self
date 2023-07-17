//! # Bank Pallet
//!
//! A pallet for handling financial systems of investment, loans, etc.
//!
//! - [`Config`]
//! - [`Call`]
//!
//! ## Overview
//!
//! Anyone can open FD (Fixed Deposit) by reserving some amount of currency with allowed maturity period. The FD principal amount
//! has to be within the range of `min_fd_amount` & `max_fd_amount` (set by admin). The FD amount is reserved from the user's `free_balance`.
//!
//! During the FD period, the reserved amount cannot be used that's why need to be freed from the `free_balance`.
//! In order to receive interest, FD can only be closed after the `fd_epoch` (set by admin) is elapsed, else the reserved amount is returned
//! to the user without any interest as per the premature withdrawal facility and a penalty (0.5-1%) is charged. The `penalty_rate` is data
//! persistent & set by the root origin.
//!
//! But, if the FD is closed after individual FD vault's `maturity_period` (set during opening), then the reserved amount is returned to the user with
//! accrued interest. The `interest_rate` is stored & set by the root origin.
//!
//! The accrued interest comes from a treasury 💎 account which is funded by the root origin. And the treasury account is funded via network's
//! inflation or balance slashing of the user in case of malicious activity.
//!
//! NOTE: The runtime must include the `Balances` pallet to handle the accounts and balances for your chain. It has been
//! shown as a [dev-dependencies] in the `Cargo.toml` file.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `set_fd_params`
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
			traits::{checked_pow, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Zero},
			DispatchError, FixedU128, Permill,
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
		type MinFDAmount: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MaxFDAmount: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MinLockValue: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MaxLockValue: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MaxFDMaturityPeriod: Get<u32>;
	}

	#[pallet::storage]
	#[pallet::getter(fn fd_params)]
	// in percentage i.e. 0.5% = 0.005 => represented as 1e6 (scaling_factor using Permill) => 5_000
	// NOTE: We can put this scaling factor as high as possible i.e. 1e9 (scaling_factor using Perbill)
	// but then during division it would lose the precision. Hence, choose as small as possible.
	// Hence, keep the scaling_factor as low as possible. Now, one can directly multiply the rate (interest/penalty) with the amount.
	//
	// E.g. If the interest rate is 0.005% per year, then the interest (in decimal * scaling_factor) is 0.005e6 = 5000
	// If the interest rate is 10%, then the interest set here as (0.1 * 1e6) = 100_000
	//
	// (Permill, Permill, u16, u32) tuple represents (interest_rate, penalty_rate, compound_frequency, fd_epoch)
	// `compound_frequency`: the number of times that interest is compounded per year
	// `fd_epoch` is the duration in blocks for which the interest is applicable like 8% per year (this is the fd_epoch whether
	// it should be a year or 2). So, here 8% is the interest per fd_epoch. Normally it should be 1 year.
	pub type FDParams<T: Config> = StorageValue<_, (Permill, Permill, u16, u32)>;

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
		/// Set FD Interest Rate, Penalty Rate, FD Epoch
		FDParamsSet { interest_rate: Permill, penalty_rate: Permill, fd_epoch: u32 },

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
		ZeroFDInterestRate,
		/// Zero Compound Frequency
		ZeroFDCompoundFrequency,
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
		FDMaturityPeriodOutOfRangeWhenOpening,
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
		/// FD Amount Out Of Range When Opening
		FDAmountOutOfRangeWhenOpening,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set FD Interest Rate, Scaling Factor, Per_Duration (EPOCH)
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_fd_params())]
		pub fn set_fd_params(
			origin: OriginFor<T>,
			interest_rate: Permill,
			penalty_rate: Permill,
			compound_frequency: u16,
			fd_epoch: u32,
		) -> DispatchResult {
			// ensure the root origin signed
			ensure_root(origin)?;

			// ensure positive interest
			ensure!(interest_rate > Permill::zero(), Error::<T>::ZeroFDInterestRate);

			// ensure penalty is not zero
			ensure!(penalty_rate > Permill::zero(), Error::<T>::ZeroFDPenalty);

			// ensure compound frequency is not zero
			ensure!(compound_frequency > 0, Error::<T>::ZeroFDCompoundFrequency);

			// ensure per duration is not zero
			ensure!(fd_epoch > 0, Error::<T>::ZeroFDEpoch);

			// set the FD params
			FDParams::<T>::put((interest_rate, penalty_rate, compound_frequency, fd_epoch));

			// emit the event
			Self::deposit_event(Event::FDParamsSet { interest_rate, penalty_rate, fd_epoch });

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

		/// Open FD
		#[pallet::call_index(3)]
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

			// ensure that the amount is within the range of min. & max. FD value
			ensure!(
				amount >= T::MinFDAmount::get() && amount <= T::MaxFDAmount::get(),
				Error::<T>::FDAmountOutOfRangeWhenOpening
			);

			// ensure the treasury is set
			ensure!(Treasury::<T>::get().is_some(), Error::<T>::TreasuryNotSet);

			// ensure the FD details set
			ensure!(<FDParams<T>>::exists(), Error::<T>::FDParamsNotSet);

			// ensure the maturity_period is greater than fd_epoch at least
			ensure!(
				maturity_period >= FDParams::<T>::get().unwrap().3
					&& maturity_period <= T::MaxFDMaturityPeriod::get(),
				Error::<T>::FDMaturityPeriodOutOfRangeWhenOpening
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

		#[pallet::call_index(4)]
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
			let (interest_rate, penalty_rate, compound_frequency, fd_epoch) =
				FDParams::<T>::get().ok_or(Error::<T>::FDInterestNotSet)?;

			// get the current block number
			let current_block_num = <frame_system::Pallet<T>>::block_number();

			// get the block difference if any
			let staked_duration = current_block_num
				.checked_sub(&block_num_opened_at)
				.ok_or(Error::<T>::ArithmeticUnderflow)?;
			// log::info!(target: TARGET, "Staked duration: {:?}", interest);

			// Here, maturity_period is considered for calculation due to FD,
			// Otherwise, in case of RD, the staked_duration is considered for calculation, although it has lesser
			// interest rate than FD.
			if staked_duration >= maturity_period.into() && has_matured == 1 {
				// if the FD is open for min. duration i.e. `FDEpoch`, then calculate the interest
				// & transfer the (principal_amount + interest) from the treasury account to the FD holder;
				// else transfer the amount only from the treasury account to the caller
				// calculate the interest directly
				let total_interest: BalanceOf<T> = Self::get_compound_interest(
					principal_amount,
					interest_rate,
					compound_frequency,
					fd_epoch,
					maturity_period,
				)?;

				log::info!(target: TARGET, "Interest: {:?}", total_interest);
				// println!("Interest on post-mature withdrawal: {:?}", interest); // for testing only

				// check the treasury's free_balance is greater than the interest
				ensure!(
					T::MyCurrency::free_balance(&treasury) > total_interest,
					Error::<T>::InsufficientFreeBalanceForInterest
				);

				// TODO: Calculate the Investment Score (IS) for the user
				// Investment Score (IS) = 1000 * log10(1 + (A/D)), Here,
				// let investment_score = Self::calculate_investment_score(&user, &interest);

				// transfer the interest from the treasury account to the user
				let _ = T::MyCurrency::transfer(&treasury, &user, total_interest, AllowDeath);

				// remove the FD details from the storage for the user
				<FDVaults<T>>::remove(&user, id);

				// unreserve the principal_amount from the user
				T::MyCurrency::unreserve(&user, principal_amount);

				// emit the event
				Self::deposit_event(Event::FDClosed {
					maturity: true,
					user,
					principal: principal_amount,
					interest: total_interest,
					penalty: Zero::zero(),
					block: current_block_num,
				});

				Ok(())
			} else if staked_duration < maturity_period.into() && has_matured == 0 {
				// calculate the penalty
				let penalty = Self::get_penalty(principal_amount, penalty_rate);

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

		#[pallet::call_index(5)]
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

		#[pallet::call_index(6)]
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
		// function to convert balance to u32
		pub fn balance_to_u32(input: BalanceOf<T>) -> Option<u32> {
			TryInto::<u32>::try_into(input).ok()
		}

		// function to convert balance to u128
		pub fn balance_to_u128(input: BalanceOf<T>) -> Option<u128> {
			TryInto::<u128>::try_into(input).ok()
		}

		// NOTE: prefer this to avoid truncating during arithmetic operations
		pub fn u32_to_balance(input: u32) -> Option<BalanceOf<T>> {
			TryInto::<BalanceOf<T>>::try_into(input).ok()
		}

		// function to convert u128 to balance
		pub fn u128_to_balance(input: u128) -> Option<BalanceOf<T>> {
			TryInto::<BalanceOf<T>>::try_into(input).ok()
		}

		// Get the FD params
		pub fn get_fd_params() -> (Permill, Permill, u16, u32) {
			let (interest_rate, penalty_rate, compound_frequency, fd_epoch) =
				FDParams::<T>::get().unwrap();

			(interest_rate, penalty_rate, compound_frequency, fd_epoch)
		}

		// As per the plan the IS ∈ [0, 1000) following Log curve (increasing) ⎛
		// NOTE: As logarithm can't be calculated on blockchain as its a floating point operation (indeterministic)
		// & blockchain only supports deterministic operations.
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

		// Get simple interest
		// NOTE: No compounding of interest, interest is calculated on the principal amount
		// only based on staked duration
		#[allow(dead_code)]
		fn get_simple_interest(
			principal_amount: BalanceOf<T>,
			interest_rate: Permill,
			fd_epoch: u32,
			maturity_period: u32,
		) -> Result<BalanceOf<T>, &'static str> {
			// calc_simple_interest
			let annual_interest = interest_rate * principal_amount;
			let total_interest = annual_interest
				.checked_mul(&maturity_period.into())
				.and_then(|v| v.checked_div(&fd_epoch.into()))
				.ok_or("Simple Interest calculation failed")?;
			Ok(total_interest)
		}

		// get penalty amount for FD maturity period i.e. if FD closed < maturity period.
		fn get_penalty(principal_amount: BalanceOf<T>, penalty_rate: Permill) -> BalanceOf<T> {
			let mut penalty = penalty_rate * principal_amount;

			if penalty == Zero::zero() {
				penalty = Permill::from_percent(1) * principal_amount;
			}

			penalty
		}

		// get total interest amount for FD maturity period
		// ```txt
		// A = P * (1 + r/n)^(nt)
		//
		// A = the future value of the investment (i.e. principal amount), including interest
		// P = the principal investment amount (the initial deposit)
		// r = the annual interest rate (decimal)
		// n = the number of times that interest is compounded per year
		// t = the number of years the money is invested
		// ```
		pub fn get_compound_interest(
			principal_amount: BalanceOf<T>,
			interest_rate: Permill,
			compound_frequency: u16,
			fd_epoch: u32,
			maturity_period: u32,
		) -> Result<BalanceOf<T>, &'static str> {
			//
			let interest_rate_in_percent = interest_rate.deconstruct();

			// r/n
			// = interest_rate / compound_frequency
			// NOTE: here, fd_epoch is generic so that any financial institution can
			// set this based on their own duration of consideration instead of default 1 year.
			// For 1 year, fd_epoch = 5_184_000 blocks, assuming 1 block = 6s.
			let k = FixedU128::from_inner(interest_rate_in_percent as u128 * 1e12 as u128);

			// 1 + r/n
			let l = FixedU128::from(1).checked_add(&k).unwrap();

			// n * t
			let compound_frequency_u32 = compound_frequency as u32;
			let nt = compound_frequency_u32 * maturity_period / fd_epoch;
			// println!("nt: {:?}", nt);

			// (1 + r/n) ^ (n * t)
			let cp: FixedU128 = checked_pow(l, nt as usize).unwrap();

			// CI = MA - PA
			// CI_factor = [(1 + r/n) ^ (n * t) - 1]
			let cp_minus_one: FixedU128 =
				cp.checked_sub(&FixedU128::from_u32(1)).unwrap_or_default();

			let p_u128: u128 = Self::balance_to_u128(principal_amount).unwrap();
			let p_fixedu128: FixedU128 = FixedU128::from(p_u128);

			let total_interest_fixedu128: FixedU128 =
				cp_minus_one.checked_mul(&p_fixedu128).unwrap_or_default();
			let total_interest_u128 = total_interest_fixedu128.into_inner() / 1e18 as u128;
			let total_interest: BalanceOf<T> =
				TryInto::<BalanceOf<T>>::try_into(total_interest_u128)
					.map_err(|_| "Compound Interest calculation failed")?;

			Ok(total_interest)
		}

		// suppress warnings for defined code that aren't not used yet, but will be used in the future.
		#[allow(dead_code)]
		// calculate the investment score for the given maturity_amount and difficulty_factor
		// formula: `IS = 1000 * (1 - (1 / (1 + MA / DF)))`
		fn calculate_investment_score(
			maturity_amount: FixedU128,
			difficulty_factor: FixedU128,
		) -> FixedU128 {
			let one = FixedU128::from(1);
			let thousand = FixedU128::from(1000);

			// Calculate the ratio of maturity_amount to difficulty_factor
			maturity_amount
				.checked_div(&difficulty_factor)
				// Add 1 to the ratio
				.and_then(|ratio| ratio.checked_add(&one))
				// Calculate the reciprocal of the incremented ratio
				.and_then(|incremented_ratio| one.checked_div(&incremented_ratio))
				// Subtract the reciprocal from 1
				.and_then(|reciprocal| one.checked_sub(&reciprocal))
				// Multiply the result by 1000
				.and_then(|subtracted| subtracted.checked_mul(&thousand))
				.unwrap_or_default()
		}

		// Required for testing
		/// Reset Treasury account from where the interest will be paid.
		pub fn reset_treasury() {
			// set the treasury
			Treasury::<T>::kill();

			// emit the event
			Self::deposit_event(Event::TreasuryReset {
				block_num: <frame_system::Pallet<T>>::block_number(),
			});
		}
	}
}
