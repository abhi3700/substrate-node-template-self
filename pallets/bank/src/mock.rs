use crate as pallet_bank;
use frame_support::{
	parameter_types,
	traits::{ConstU128, ConstU16, ConstU32, ConstU64},
};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// define test accounts
pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const CHARLIE: u64 = 3;
pub const DAVE: u64 = 4;
pub const TREASURY: u64 = 100;

pub const ONE_YEAR: u32 = 5_184_000;

/// Balance of an account.
pub type Balance = u128;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		// used as dependency (for handling accounts and balances) for pallet_bank
		Balances: pallet_balances,
		Bank: pallet_bank,
	}
);

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU128<100>;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type HoldIdentifier = ();
	type MaxHolds = ();
}

parameter_types! {
	pub const MinFDAmount: <Test as pallet_balances::Config>::Balance = 50 * 1e10 as Balance;
	pub const MaxFDAmount: <Test as pallet_balances::Config>::Balance = 200_000 * 1e10 as Balance;
	pub const MinLockValue: <Test as pallet_balances::Config>::Balance = 20 * 1e10 as Balance;
	pub const MaxLockValue: <Test as pallet_balances::Config>::Balance = 100_000 * 1e10 as Balance;
	pub const MaxFDMaturityPeriod: u32 = 5 * ONE_YEAR;	// 5 years
}

impl pallet_bank::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MyCurrency = Balances;
	type MinFDAmount = MinFDAmount;
	type MaxFDAmount = MaxFDAmount;
	type MinLockValue = MinLockValue;
	type MaxLockValue = MaxLockValue;
	type MaxFDMaturityPeriod = MaxFDMaturityPeriod;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(ALICE, 10_000 * 1e10 as Balance),
			(BOB, 20_000 * 1e10 as Balance),
			(CHARLIE, 30_000 * 1e10 as Balance),
			(DAVE, 40_000 * 1e10 as Balance),
			(TREASURY, 1_000_000 * 1e10 as Balance),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
