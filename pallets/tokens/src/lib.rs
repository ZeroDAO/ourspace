#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::pallet_prelude::*;
use frame_system::{ensure_signed, pallet_prelude::*};
use orml_traits::{
    arithmetic::{self, Signed},
    MultiCurrency, SocialCurrency,
};
use sp_runtime::traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize, Member, StaticLookup};

mod default_weight;
mod mock;
mod tests;

use sp_std::convert::{TryFrom, TryInto};

pub use module::*;

#[frame_support::pallet]
pub mod module {
    use super::*;

    pub trait WeightInfo {
        fn transfer_social() -> Weight;
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency ID type
        type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord;

        type Balance: Parameter
            + Member
            + AtLeast32BitUnsigned
            + Default
            + Copy
            + MaybeSerializeDeserialize;

        type Amount: Signed
            + TryInto<Self::Balance>
            + TryFrom<Self::Balance>
            + Parameter
            + Member
            + arithmetic::SimpleArithmetic
            + Default
            + Copy
            + MaybeSerializeDeserialize;

        type Currency: MultiCurrency<
                Self::AccountId,
                CurrencyId = Self::CurrencyId,
                Balance = Self::Balance,
            > + SocialCurrency<Self::AccountId, Balance = Self::Balance>;

        /// Weight information for extrinsics in this module.
        type WeightInfo: WeightInfo;
    }

    #[pallet::error]
    pub enum Error<T> {}

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Token transfer_social success. \[currency_id, from, to, amount\]
        TransferSocial(T::CurrencyId, T::AccountId, T::AccountId, T::Balance),
    }

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Transfer some balance to another social-currency account
        ///
        /// The dispatch origin for this call must be `Signed` by the
        /// transactor.
        #[pallet::weight(T::WeightInfo::transfer_social())]
        pub fn transfer_social(
            origin: OriginFor<T>,
            dest: <T::Lookup as StaticLookup>::Source,
            currency_id: T::CurrencyId,
            #[pallet::compact] amount: T::Balance,
        ) -> DispatchResultWithPostInfo {
            let from = ensure_signed(origin)?;
            let to = T::Lookup::lookup(dest)?;
            T::Currency::transfer_social(currency_id, &from, &to, amount)?;
            Self::deposit_event(Event::TransferSocial(currency_id, from, to, amount));
            Ok(().into())
        }
    }
}
