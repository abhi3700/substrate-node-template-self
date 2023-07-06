#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
	}

	// The pallet's runtime storage items.
	// https://docs.substrate.io/main-docs/build/runtime-storage/
	#[pallet::storage]
	#[pallet::getter(fn value)]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/main-docs/build/runtime-storage/#declaring-storage-items
	pub type Value<T> = StorageValue<_, bool>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ValueSet { value: bool, who: T::AccountId },
		ValueFlipped { new: bool, who: T::AccountId },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Value not set
		NoneSet,
		/// Value already Set
		AlreadySet,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set value: true/false
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn set_value(origin: OriginFor<T>, value: bool) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/main-docs/build/origins/
			let who = ensure_signed(origin)?;

			// Read a value from storage.
			match <Value<T>>::get() {
				Some(_) => Err(Error::<T>::AlreadySet)?,
				None => {
					// Update storage.
					<Value<T>>::put(value);

					// Emit an event.
					Self::deposit_event(Event::ValueSet { value, who });
					Ok(())
				},
			}
		}

		/// Flip stored value (any: true/false)
		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
		pub fn flip_value(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Read a value from storage.
			match <Value<T>>::get() {
				Some(old) => {
					// flip the value read from storage if already set
					let new = !old;
					// Update storage with new value
					<Value<T>>::put(new);

					// Emit an event
					Self::deposit_event(Event::ValueFlipped { new, who });

					Ok(())
				},
				// Return an error if the value has not been set.
				None => Err(Error::<T>::NoneSet)?,
			}
		}
	}
}
