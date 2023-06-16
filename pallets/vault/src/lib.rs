//! # Vault Pallet
//!
//! - [`Config`]
//! - [`Call`]
//!
//! ## Overview
//!
//! The Vault pallet provides functionality for handling staking of tokens by users.
//!
//! When a user stakes tokens, they are added to the total staked tokens of the vault & individual vault.
//! And when they unstake tokens, they are removed from the total staked tokens of the vault & individual vault.
//!
//! The accrued interest is calculated till the unstake timestamp.
//!
//! ### Terminology
//!
//! - **Vault**: where users stake their tokens.
//! - **Stake**: lock tokens by users.
//! - **Unstake**: unlock tokens unlocked by users.
//! - **Total Staked Tokens**: total tokens staked by all users.
//! - **Individual Staked Tokens**: tokens staked by a user.
//! - **Accrued Interest**: interest earned by a user.
//! - **Claimable Amount**: amount of tokens a user can claim including accrued interest.
//! - **Stake Timestamp**: timestamp when a user stakes its tokens.
//! - **Unstake Timestamp**: timestamp when a user unstakes its tokens.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! #### Public
//!
//! These calls can be made any externally held account that is capable of creating
//! a signed extrinsic.
//!
//! Actions:
//!
//! - `deposit`: Deposit tokens to its vault.
//! - `unstake`: Unstake tokens from its vault.
//! - `withdraw`: Withdraw tokens from its vault.
//!
//! #### Root
//! - `set_apy`: Set APY for the vault.
//!

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::log;
	use frame_support::traits::{Currency, ReservableCurrency};
	use frame_support::{pallet_prelude::*, Blake2_128Concat};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	type AccountOf<T> = <T as frame_system::Config>::AccountId; // optional
	type BalanceOf<T> = <<T as Config>::MyCurrency as Currency<AccountOf<T>>>::Balance;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// MyCurrency type for this pallet. Here, we could have used `Currency` trait.
		/// But, we need to use `reserved_balance` function which is not available in `Currency` trait.
		/// That's why `ReservableCurrency` trait is used.
		type MyCurrency: ReservableCurrency<Self::AccountId>;
	}

	#[derive(
		Clone, Encode, Decode, Eq, PartialEq, TypeInfo, RuntimeDebug, Default, MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct DiffBalances<T: Config> {
		free_balance: BalanceOf<T>,
		reserved_balance: BalanceOf<T>,
		total_balance: BalanceOf<T>,
	}

	// The pallet's runtime storage items.
	// https://docs.substrate.io/main-docs/build/runtime-storage/
	#[pallet::storage]
	#[pallet::getter(fn get_balance)]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/main-docs/build/runtime-storage/#declaring-storage-items
	// can also use `AccountOf<T>` instead of `T::AccountId` here.
	pub type SomeBalance<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, DiffBalances<T>>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Total balance set.
		BalanceSet {
			who: T::AccountId, // can also use `AccountOf<T>`
			total_balance: BalanceOf<T>,
			current_block: T::BlockNumber,
		},

		/// Total balance updated.
		BalanceUpdated {
			who: T::AccountId, // can also use `AccountOf<T>`
			old_total_balance: BalanceOf<T>,
			new_total_balance: BalanceOf<T>,
			current_block: T::BlockNumber,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Balances Not set.
		BalancesNotSet,
		/// Insufficient reserves.
		InsufficientReserves,
		/// Old Total balance is greater.
		OldTotalBalanceIsGreater,
	}

	// All these functions mentioned here are callable by external user.
	// And each function cost some weight.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set total balance
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn set_balance(origin: OriginFor<T>) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/main-docs/build/origins/
			let who = ensure_signed(origin)?;

			// get the diff balances of the caller. [Total = free + reserved]
			let free_balance = T::MyCurrency::free_balance(&who);
			let reserved_balance = T::MyCurrency::reserved_balance(&who);
			let total_balance = T::MyCurrency::total_balance(&who);

			let diff_balances = DiffBalances { free_balance, reserved_balance, total_balance };

			// ensure the balance is not set
			ensure!(<SomeBalance<T>>::get(&who) == None, Error::<T>::BalancesNotSet);

			// Update storage.
			<SomeBalance<T>>::insert(&who, diff_balances);

			// Emit an event.
			Self::deposit_event(Event::BalanceSet {
				who,
				total_balance,
				current_block: <frame_system::Pallet<T>>::block_number(),
			});

			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		/// Update balance if it is greater than the old balance.
		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
		pub fn update_balance(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let current_tot_balance = T::MyCurrency::total_balance(&who);
			let min_balance = T::MyCurrency::minimum_balance();

			log::info!("current_tot_balance: {:?}", current_tot_balance);
			log::info!("min_balance: {:?}", min_balance);
			log::debug!("current_tot_balance: {:?}", current_tot_balance);
			log::debug!("min_balance: {:?}", min_balance);
			ensure!(current_tot_balance > min_balance, Error::<T>::InsufficientReserves);

			// Read a value from storage.
			match <SomeBalance<T>>::get(&who) {
				// Return an error if the value has not been set.
				None => return Err(Error::<T>::BalancesNotSet.into()),
				Some(old_diff_balances) => {
					ensure!(
						current_tot_balance > old_diff_balances.total_balance,
						Error::<T>::OldTotalBalanceIsGreater
					);

					// get the diff balances of the caller. [Total = free + reserved]
					let new_free_balance = T::MyCurrency::free_balance(&who);
					let new_reserved_balance = T::MyCurrency::reserved_balance(&who);
					let new_total_balance = T::MyCurrency::total_balance(&who);

					let new_diff_balances = DiffBalances {
						free_balance: new_free_balance,
						reserved_balance: new_reserved_balance,
						total_balance: new_total_balance,
					};

					// update the storage
					<SomeBalance<T>>::insert(&who, new_diff_balances);

					// Emit an event.
					Self::deposit_event(Event::BalanceUpdated {
						who,
						old_total_balance: old_diff_balances.total_balance,
						new_total_balance,
						current_block: <frame_system::Pallet<T>>::block_number(),
					});

					Ok(())
				},
			}
		}
	}
}
