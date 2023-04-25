#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {

	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	// The pallet's runtime storage items.
	// https://docs.substrate.io/main-docs/build/runtime-storage/
	#[pallet::storage]
	#[pallet::getter(fn count)]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/main-docs/build/runtime-storage/#declaring-storage-items
	pub type Count<T> = StorageValue<_, u32>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ValueStored { value: u32, who: T::AccountId },
		ValueIncremented { old: u32, new: u32, who: T::AccountId },
		ValueDecremented { old: u32, new: u32, who: T::AccountId },
		ValueReset { old: u32, who: T::AccountId },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// No Value is stored.
		NoneValueStored,
		/// Already Value is stored.
		ValueAlreadyStored,
		/// Zero value is stored.
		ZeroValueStored,
		// Storage Overflow
		StorageOverflow,
		// Invalid Value parsed
		InvalidInputValue,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set Some non-zero value
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn set(origin: OriginFor<T>, value: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// input sanitization for input value
			if value == 0 {
				Err(Error::<T>::InvalidInputValue)?
			}

			// Read value from storage
			match <Count<T>>::get() {
				None => {
					// Put the value of Count
					<Count<T>>::put(value);

					// emit the event
					Self::deposit_event(Event::ValueStored { value, who });

					Ok(())
				},
				Some(_) => Err(Error::<T>::ValueAlreadyStored)?,
			}
		}

		/// Increment dispatchable for incrementing the count
		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
		pub fn increment(origin: OriginFor<T>, by: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// input sanitization for input value
			if by == 0 {
				Err(Error::<T>::InvalidInputValue)?
			}

			// Read a value from storage.
			match <Count<T>>::get() {
				// Return an error if the value has not been set.
				None => Err(Error::<T>::NoneValueStored)?,
				Some(old) => {
					// Increment the value read from storage; will error in the event of overflow.
					let new = old.checked_add(by).ok_or(Error::<T>::StorageOverflow)?;
					// Update the value in storage with the incremented result.
					<Count<T>>::put(new);

					// emit the event
					Self::deposit_event(Event::ValueIncremented { old, new, who });

					// return none
					Ok(())
				},
			}
		}

		/// Decrement dispatchable for decrementing the count
		#[pallet::call_index(2)]
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
		pub fn decrement(origin: OriginFor<T>, by: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// input sanitization for input value
			if by == 0 {
				Err(Error::<T>::InvalidInputValue)?
			}

			// Read value from storage
			match <Count<T>>::get() {
				// Return an error if the value has not been set.
				None => Err(Error::<T>::NoneValueStored)?,
				Some(old) => {
					let new = old.checked_sub(by).ok_or(Error::<T>::StorageOverflow)?;

					// Update the value in storage with the decremented result.
					<Count<T>>::put(new);

					// emit the event
					Self::deposit_event(Event::ValueDecremented { old, new, who });

					// return None
					Ok(())
				},
			}
		}

		/// Reset dispatchable for resetting the count
		#[pallet::call_index(3)]
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
		pub fn reset(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Read value from storage
			match <Count<T>>::get() {
				None => Err(Error::<T>::NoneValueStored)?,
				Some(old) => {
					if old == 0 {
						Err(Error::<T>::ZeroValueStored)?;
					}
					// reset the value
					<Count<T>>::put(0);

					// emit the event
					Self::deposit_event(Event::ValueReset { old, who });

					Ok(())
				},
			}
		}
	}
}
