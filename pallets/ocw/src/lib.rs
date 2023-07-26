//! Offchain tutorial source: https://docs.substrate.io/tutorials/build-application-logic/add-offchain-workers/

#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, Encode};
use frame_support::{
	log,
	sp_runtime::{
		offchain::{
			http,
			storage::{MutateStorageError, StorageValueRef},
		},
		traits::{BlockNumberProvider, Get, Zero},
		transaction_validity::{
			InvalidTransaction, TransactionPriority, TransactionValidity, ValidTransaction,
		},
		RuntimeDebug,
	},
};
use frame_system::{
	limits::BlockLength,
	offchain::{
		AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
		SignedPayload, Signer, SigningTypes, SubmitTransaction,
	},
	pallet_prelude::BlockNumberFor,
};
use lite_json::{parse_json, JsonValue};
use sp_core::{crypto::KeyTypeId, offchain::Duration};
use sp_std::vec::Vec;

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

		/// cool-down period after an unsigned tx before the next tx
		/// [unsigned-tx-1]-----(cool-down-period)-----[unsigned-tx-2]
		#[pallet::constant]
		type GracePeriod: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type UnsignedInterval: Get<BlockNumberFor<Self>>;

		/// Maximum number of prices.
		#[pallet::constant]
		type MaxPrices: Get<u32>;

		/// to decide the transaction priority
		type UnsignedPriority: Get<TransactionPriority>;
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

			let should_send = Self::choose_transaction_type(block_number);
			let res = match should_send {
				TransactionType::Signed => Self::fetch_price_and_send_signed(),
				TransactionType::UnsignedForAny => {
					Self::fetch_price_and_send_unsigned_for_any_account(block_number)
				},
				TransactionType::UnsignedForAll => {
					Self::fetch_price_and_send_unsigned_for_any_account(block_number)
				},
				TransactionType::Raw => Self::fetch_price_and_send_raw_unsigned(block_number),
				TransactionType::None => Ok(()),
			};
			if let Err(e) = res {
				log::error!("Error: {}", e);
			}
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn something)]
	pub type Prices<T: Config> = StorageValue<_, BoundedVec<u32, T::MaxPrices>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn next_unsigned_at)]
	pub type NextUnsignedAt<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

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

			Ok(())
		}

		/// Here, although this call is unsigned, but still we need to specify the weight.
		/// Otherwise, there won't be any limit to no. of unsigned txs added in a block.

		/// Each unsigned tx has to go through the `validate_unsigned()` defined in the implementation
		/// of `ValidateUnsigned` trait for the pallet.
		#[pallet::call_index(1)]
		#[pallet::weight({10_000})]
		pub fn submit_price_unsigned(
			origin: OriginFor<T>,
			_block_number: BlockNumberFor<T>,
			price: u32,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;

			//
			Self::add_price(None, price);
			NextUnsignedAt::<T>::put(Self::current_block_number() + T::UnsignedInterval::get());

			Ok(().into())
		}

		// Here, although this call is unsigned, but still we need to specify the weight.
		// Otherwise, there won't be any limit to no. of unsigned txs.
		#[pallet::call_index(2)]
		#[pallet::weight({10_000})]
		pub fn submit_price_unsigned_with_signed_payload(
			origin: OriginFor<T>,
			price_payload: PricePayload<T::Public, BlockNumberFor<T>>,
			_signature: T::Signature,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;

			Self::add_price(None, price_payload.price);
			NextUnsignedAt::<T>::put(Self::current_block_number() + T::UnsignedInterval::get());

			Ok(().into())
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		/// Validate unsigned call to this module.
		///
		/// By default unsigned transactions are disallowed, but implementing the validator
		/// here we make sure that some particular calls (the ones produced by offchain worker)
		/// are being whitelisted and marked as valid.
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			// Firstly let's check that we call the right function.
			if let Call::submit_price_unsigned_with_signed_payload {
				price_payload: ref payload,
				ref signature,
			} = call
			{
				let signature_valid =
					SignedPayload::<T>::verify::<T::AuthorityId>(payload, signature.clone());
				if !signature_valid {
					return InvalidTransaction::BadProof.into();
				}
				Self::validate_transaction_parameters(&payload.block_number, &payload.price)
			} else if let Call::submit_price_unsigned { block_number, price: new_price } = call {
				Self::validate_transaction_parameters(block_number, new_price)
			} else {
				InvalidTransaction::Call.into()
			}
		}
	}

	impl<T: Config> BlockNumberProvider for Pallet<T> {
		type BlockNumber = T::BlockNumber;

		fn current_block_number() -> Self::BlockNumber {
			<frame_system::Pallet<T>>::block_number()
		}
	}
}

/// Payload used by this example crate to hold price
/// data required to submit a transaction.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
pub struct PricePayload<Public, BlockNumber> {
	price: u32,
	block_number: BlockNumber,
	public: Public,
}

impl<T: SigningTypes> SignedPayload<T> for PricePayload<T::Public, BlockNumberFor<T>> {
	fn public(&self) -> T::Public {
		self.public.clone()
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
	fn choose_transaction_type(block_number: BlockNumberFor<T>) -> TransactionType {
		const RECENTLY_SENT: () = ();

		let val = StorageValueRef::persistent(b"palletocw::last_send");

		let res = val.mutate(|last_send| match last_send {
			Ok(Some(block)) if block < block_number + T::GracePeriod::get() => Err(RECENTLY_SENT),
			_ => Ok(block_number),
		});

		match res {
			Ok(_) => {
				let transaction_type = block_number % 4u32.into();
				if transaction_type == Zero::zero() {
					TransactionType::Signed
				} else if transaction_type == BlockNumberFor::<T>::from(1u32) {
					TransactionType::UnsignedForAny
				} else if transaction_type == BlockNumberFor::<T>::from(2u32) {
					TransactionType::UnsignedForAll
				} else if transaction_type == BlockNumberFor::<T>::from(3u32) {
					TransactionType::Raw
				} else {
					TransactionType::None
				}
			},

			Err(MutateStorageError::ValueFunctionFailed(RECENTLY_SENT)) => {
				frame_support::log::info!("skipping sending tx, sent recently");
				TransactionType::None
			},
			Err(MutateStorageError::ConcurrentModification(_)) => {
				frame_support::log::error!("error working with storage");
				TransactionType::None
			},
		}
	}

	/// A helper function to fetch the price and send signed transaction.
	fn fetch_price_and_send_signed() -> Result<(), &'static str> {
		let signer = Signer::<T, T::AuthorityId>::all_accounts();
		if !signer.can_sign() {
			return Err(
				"No local accounts available. Consider adding one via `author_insertKey` RPC.",
			);
		}
		// Make an external HTTP request to fetch the current price.
		// Note this call will block until response is received.
		let price = Self::fetch_price().map_err(|_| "Failed to fetch price")?;

		// Using `send_signed_transaction` associated type we create and submit a transaction
		// representing the call, we've just created.
		// Submit signed will return a vector of results for all accounts that were found in the
		// local keystore with expected `KEY_TYPE`.
		// `send_signed_transaction()` return type is `Option<(Account<T>, Result<(), ()>)>`. It is:
		//	 - `None`: no account is available for sending transaction
		//	 - `Some((account, Ok(())))`: transaction is successfully sent
		//	 - `Some((account, Err(())))`: error occurred when sending the transaction
		let results = signer.send_signed_transaction(|_account| {
			// Received price is wrapped into a call to `submit_price` public function of this
			// pallet. This means that the transaction, when executed, will simply call that
			// function passing `price` as an argument.
			Call::submit_price { price }
		});

		for (acc, res) in &results {
			match res {
				Ok(()) => log::info!("[{:?}] Submitted price of {} cents", acc.id, price),
				Err(e) => log::error!("[{:?}] Failed to submit transaction: {:?}", acc.id, e),
			}
		}

		Ok(())
	}

	/// A helper function to fetch the price and send a raw unsigned transaction.
	fn fetch_price_and_send_raw_unsigned(
		block_number: BlockNumberFor<T>,
	) -> Result<(), &'static str> {
		// Make sure we don't fetch the price if unsigned transaction is going to be rejected
		// anyway.
		let next_unsigned_at = <NextUnsignedAt<T>>::get();
		if next_unsigned_at > block_number {
			return Err("Too early to send unsigned transaction");
		}

		// Make an external HTTP request to fetch the current price.
		// Note this call will block until response is received.
		let price = Self::fetch_price().map_err(|_| "Failed to fetch price")?;

		// Received price is wrapped into a call to `submit_price_unsigned` public function of this
		// pallet. This means that the transaction, when executed, will simply call that function
		// passing `price` as an argument.
		let call = Call::submit_price_unsigned { block_number, price };

		// Now let's create a transaction out of this call and submit it to the pool.
		// Here we showcase two ways to send an unsigned transaction / unsigned payload (raw)
		//
		// By default unsigned transactions are disallowed, so we need to whitelist this case
		// by writing `UnsignedValidator`. Note that it's EXTREMELY important to carefuly
		// implement unsigned validation logic, as any mistakes can lead to opening DoS or spam
		// attack vectors. See validation logic docs for more details.
		//
		SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
			.map_err(|()| "Unable to submit unsigned transaction.")?;

		Ok(())
	}

	/// A helper function to fetch the price, sign payload and send an unsigned transaction
	fn fetch_price_and_send_unsigned_for_any_account(
		block_number: BlockNumberFor<T>,
	) -> Result<(), &'static str> {
		// Make sure we don't fetch the price if unsigned transaction is going to be rejected
		// anyway.
		let next_unsigned_at = <NextUnsignedAt<T>>::get();
		if next_unsigned_at > block_number {
			return Err("Too early to send unsigned transaction");
		}

		// Make an external HTTP request to fetch the current price.
		// Note this call will block until response is received.
		let price = Self::fetch_price().map_err(|_| "Failed to fetch price")?;

		// -- Sign using any account
		let (_, result) = Signer::<T, T::AuthorityId>::any_account()
			.send_unsigned_transaction(
				|account| PricePayload { price, block_number, public: account.public.clone() },
				|payload, signature| Call::submit_price_unsigned_with_signed_payload {
					price_payload: payload,
					signature,
				},
			)
			.ok_or("No local accounts accounts available.")?;
		result.map_err(|()| "Unable to submit transaction")?;

		Ok(())
	}

	/// A helper function to fetch the price, sign payload and send an unsigned transaction
	fn fetch_price_and_send_unsigned_for_all_accounts(
		block_number: BlockNumberFor<T>,
	) -> Result<(), &'static str> {
		// Make sure we don't fetch the price if unsigned transaction is going to be rejected
		// anyway.
		let next_unsigned_at = <NextUnsignedAt<T>>::get();
		if next_unsigned_at > block_number {
			return Err("Too early to send unsigned transaction");
		}

		// Make an external HTTP request to fetch the current price.
		// Note this call will block until response is received.
		let price = Self::fetch_price().map_err(|_| "Failed to fetch price")?;

		// -- Sign using all accounts
		let transaction_results = Signer::<T, T::AuthorityId>::all_accounts()
			.send_unsigned_transaction(
				|account| PricePayload { price, block_number, public: account.public.clone() },
				|payload, signature| Call::submit_price_unsigned_with_signed_payload {
					price_payload: payload,
					signature,
				},
			);
		for (_account_id, result) in transaction_results.into_iter() {
			if result.is_err() {
				return Err("Unable to submit transaction");
			}
		}

		Ok(())
	}

	fn fetch_price() -> Result<u32, http::Error> {
		// set a deadline
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));

		// Here we are preparing the http GET request call
		let request =
			http::Request::get("https://min-api.cryptocompare.com/data/price?fsym=BTC&tsyms=USD");

		// Get the pending request
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;

		// Get the response after waiting for the deadline
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;

		// let's check the response before reading the response
		if response.code == 200 {
			log::info!("Unexpected response code: {}", response.code);
			return Err(http::Error::Unknown);
		}

		// Convert the response body into bytes
		let body = response.body().collect::<Vec<u8>>();

		// convert the body (in bytes) to body (in str slice)
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| http::Error::Unknown)?;

		// extract the price value
		let price = match Self::parse_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::info!("Unable to extract price from the response: {body_str}");
				Err(http::Error::Unknown)
			},
		}?;

		log::info!("price: {price}");

		Ok(price)
	}

	// Get the number from string slice price input fetched from HTTP request.
	fn parse_price(price_str: &str) -> Option<u32> {
		let val = parse_json(price_str);
		let price = match val.ok()? {
			JsonValue::Object(obj) => {
				let (_, v) = obj.into_iter().find(|(k, _)| k.iter().copied().eq("USD".chars()))?;
				match v {
					JsonValue::Number(number) => number,
					_ => return None,
				}
			},
			_ => return None,
		};

		let exp = price.fraction_length.saturating_sub(2);
		Some(price.integer as u32 * 100 + (price.fraction / 10_u64.pow(exp)) as u32)
	}

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

	fn validate_transaction_parameters(
		block_number: &BlockNumberFor<T>,
		new_price: &u32,
	) -> TransactionValidity {
		let next_unsigned_at = NextUnsignedAt::<T>::get();
		if &next_unsigned_at > block_number {
			return InvalidTransaction::Stale.into();
		}

		if block_number > &Self::current_block_number() {
			return InvalidTransaction::Future.into();
		}

		// in order to set the priority, we ensure the difference from the current avg price is highest possible.
		let avg_price = Self::average_price()
			.map(|price| if &price > new_price { price - new_price } else { new_price - price })
			.unwrap_or(0);

		ValidTransaction::with_tag_prefix("pallet-ocw")
			// Next we tweak the priority depending on how much
			// it differs from the current average. (the more it differs the more priority it
			// has).
			.priority(T::UnsignedPriority::get().saturating_add(avg_price as _))
			// transaction valid for next 5 blocks, after which it has to be revalidated by the pool
			.longevity(5)
			.propagate(true)
			.build()
	}
}
