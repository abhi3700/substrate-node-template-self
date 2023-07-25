//! Offchain tutorial source: https://docs.substrate.io/tutorials/build-application-logic/add-offchain-workers/

#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::sp_runtime::traits::BlockNumberProvider;
use frame_support::sp_runtime::traits::Saturating;
use frame_support::traits::Get;
use frame_system::offchain::{
	AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
	SignedPayload, Signer, SigningTypes, SubmitTransaction,
};
use sp_core::crypto::KeyTypeId;
// use sp_std::vec::Vec;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"demo");

pub mod crypto {
	use super::KEY_TYPE;
	use frame_support::sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};
	use sp_core::sr25519::Signature as Sr25519Signature;
	app_crypto!(sr25519, KEY_TYPE);

	pub struct TestAuthId;

	// implemented for runtime
	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	// implemented for mock runtime in test
	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for TestAuthId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{log, pallet_prelude::*};

	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;

		/// Maximum number of prices.
		#[pallet::constant]
		type MaxPrices: Get<u32>;
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Offchain worker entry point.
		///
		/// By implementing `fn offchain_worker` you declare a new offchain worker.
		/// This function will be called when the node is fully synced and a new best block is
		/// successfully imported.
		/// Note that it's not guaranteed for offchain workers to run on EVERY block, there might
		/// be cases where some blocks are skipped, or for some the worker runs twice (re-orgs),
		/// so the code should be able to handle that.
		fn offchain_worker(block_number: T::BlockNumber) {
			log::info!("Hello from pallet-ocw.");
			// The entry point of your code called by offchain worker

			// for creating a signer with capability to send signed & unsigned (w payload) txs.
			let signer = Signer::<T, T::AuthorityId>::all_accounts();

			// Using `send_signed_transaction` associated type we create and submit a transaction
			// representing the call we've just created.
			// `send_signed_transaction()` return type is `Option<(Account<T>, Result<(), ()>)>`. It is:
			//	 - `None`: no account is available for sending transaction
			//	 - `Some((account, Ok(())))`: transaction is successfully sent
			//	 - `Some((account, Err(())))`: error occurred when sending the transaction
			let results =
				signer.send_signed_transaction(|_account| Call::submit_price { price: 42 });

			for (acc, res) in &results {
				match res {
					Ok(()) => log::info!("[{:?}] Submitted a transaction.", acc.id),
					Err(e) => log::error!("[{:?}] Failed to submit transaction: {:?}", acc.id, e),
				}
			}
		}
		// ...
	}

	#[pallet::storage]
	#[pallet::getter(fn something)]
	pub type Prices<T: Config> = StorageValue<_, BoundedVec<u32, T::MaxPrices>, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New price added
		NewPrice { price: u32, who_maybe: Option<T::AccountId> },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		/// error in calculating avg price
		AvgPriceCalculationError,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::call_index(0)]
		#[pallet::weight({10_000})]
		pub fn submit_price(origin: OriginFor<T>, price: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Self::add_price(Some(who), price);

			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}
	}

	impl<T: Config> BlockNumberProvider for Pallet<T> {
		type BlockNumber = T::BlockNumber;

		fn current_block_number() -> Self::BlockNumber {
			<frame_system::Pallet<T>>::block_number()
		}
	}
}

enum TransactionType {
	Signed,
	UnsignedForAny,
	UnsignedForAll,
	Raw,
	None,
}

impl<T: Config> Pallet<T> {
	fn add_price(who_maybe: Option<T::AccountId>, price: u32) {
		frame_support::log::info!("Adding price: {}", price);
		// update the price, calcualate the average.
		<Prices<T>>::mutate(|prices| {
			if prices.try_push(price).is_err() {
				prices[(price % T::MaxPrices::get()) as usize] = price;
			}
		});

		let avg_price = Self::average_price().expect("error in Calculation of avg price");
		frame_support::log::info!("Average price: {}", avg_price);

		// Emit an event.
		Self::deposit_event(Event::NewPrice { price, who_maybe });
	}

	fn average_price() -> Option<u32> {
		let prices = <Prices<T>>::get();
		if prices.is_empty() {
			None
		} else {
			Some(prices.iter().fold(0, |acc, x| acc.saturating_add(*x) / prices.len() as u32))
		}
	}
}
