#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>

#[warn(unused_imports)]
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

// This code defines a substrate pallet using the `frame_support` and `frame_system` frameworks.
#[frame_support::pallet]
pub mod pallet {
	// The following lines bring in necessary dependencies from the `frame_support` and `frame_system` crates.
	use frame_support::log;
	use frame_support::pallet_prelude::*;
	use frame_support::sp_runtime::print;
	use frame_system::pallet_prelude::*;
	use scale_info::prelude::string::String;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomeoneSaysHello {
			who: T::AccountId,
		},
		SomeoneSaysAny {
			wish: String,
			who: T::AccountId,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// The string can't initiate with 'Hello'
		HelloPrefixed,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn say_hello(origin: OriginFor<T>) -> DispatchResult {
			// Ensure that the caller is a regular keypair account
			let who = ensure_signed(origin)?;

			print("Hello world");

			log::info!("{:?} said hello", who); // Error: transaction has a bad signature

			// Emit an event
			Self::deposit_event(Event::SomeoneSaysHello { who });

			// Return a successful DispatchResult
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn say_any(origin: OriginFor<T>, wish: String) -> DispatchResult {
			let who = ensure_signed(origin)?;

			if wish.starts_with("hello") {
				return Err(Error::<T>::HelloPrefixed.into());
			}

			print("Says Anything");

			// Emits an event
			Self::deposit_event(Event::SomeoneSaysAny { wish, who });

			Ok(())
		}
	}
}
