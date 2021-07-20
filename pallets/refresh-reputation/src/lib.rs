#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{
    codec::{Decode, Encode},
    ensure, pallet,
    traits::Get,
    RuntimeDebug,
};
use frame_system::{self as system, ensure_signed};
use orml_traits::{MultiCurrency, SocialCurrency, StakingCurrency};
use sp_runtime::{traits::Zero, DispatchError, DispatchResult, Perbill};
use sp_std::vec::Vec;
use zd_primitives::{Balance,fee::ProxyFee};
use zd_traits::{Reputation, StartChallenge, TrustBase};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct Record<BlockNumber, Balance> {
    pub update_at: BlockNumber,
    pub fee: Balance,
}

#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct Payroll<Balance> {
    pub count: u32,
    pub total_fee: Balance,
}

impl Payroll<Balance> {
    fn total_amount<T: Config>(&self) -> Balance {
        T::UpdateStakingAmount::get()
            .saturating_mul(self.count.into())
            .saturating_add(self.total_fee)
    }
}

#[pallet]
pub mod pallet {
    use super::*;

    use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
    use frame_system::pallet_prelude::*;
    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord;
        type BaceToken: Get<Self::CurrencyId>;
        type Currency: MultiCurrency<Self::AccountId, CurrencyId = Self::CurrencyId, Balance = Balance>
            + StakingCurrency<Self::AccountId>
            + SocialCurrency<Self::AccountId>;
        #[pallet::constant]
        type ShareRatio: Get<Perbill>;
        #[pallet::constant]
        type FeeRation: Get<Perbill>;
        #[pallet::constant]
        type SelfRation: Get<Perbill>;
        #[pallet::constant]
        type MaxUpdateCount: Get<u32>;
        #[pallet::constant]
        type UpdateStakingAmount: Get<Balance>;
        #[pallet::constant]
        type ConfirmationPeriod: Get<Self::BlockNumber>;
        type Reputation: Reputation<Self::AccountId, Self::BlockNumber>;
        type TrustBase: TrustBase<Self::AccountId>;
    }
    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn get_payroll)]
    pub type Payrolls<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Payroll<Balance>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn update_record)]
    pub type Records<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        T::AccountId,
        Twox64Concat,
        T::AccountId,
        Record<T::BlockNumber, Balance>,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Some reputations have been updated. \[analyst, count, fee\]
        ReputationRefreshed(T::AccountId, u32, Balance),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Quantity reaches limit
        QuantityLimitReached,
        /// Not in the update period
        NoUpdatesAllowed,
        /// Error getting fee
        ErrorFee,
        /// Challenge timeout
        ChallengeTimeout,
        /// Calculation overflow.
        Overflow,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn new_round(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            // 确保所有更新都超过代领期限，这种情况一般不会发生
            // 但不正常的参数设置有可能带来此种问题
           // ensure!(
           //     Reputation::get_last_refresh_at() ,
           //     Error::<T>::ChallengeTimeout
            //);
            // Fees::<T>::drain().try_fold(Zero::zero(), |acc,fee| {
                // 计算fee - 统一到某个pallet ，直接用工具就可以
                // 将每个用户扣除代理金额后，打入
            // });
            // TODO： 检查reputation pallet 收益是否完毕
            // 迭代 Fees
            // TODO: 检查challenges pallet 是否领取完毕，没有领完则无法开始
            T::Reputation::new_round()?;
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn refresh_reputation(
            origin: OriginFor<T>,
            user_scores: Vec<(T::AccountId, u32)>,
        ) -> DispatchResultWithPostInfo {
            let analyst = ensure_signed(origin)?;
            let user_count = user_scores.len();
            ensure!(
                user_count as u32 <= T::MaxUpdateCount::get(),
                Error::<T>::QuantityLimitReached
            );

            let _ = T::Reputation::check_update_status(true).ok_or(Error::<T>::NoUpdatesAllowed)?;

            let amount = T::UpdateStakingAmount::get()
                .checked_mul(user_count as Balance)
                .ok_or(Error::<T>::Overflow)?;

            T::Currency::staking(T::BaceToken::get(), &analyst, amount)?;

            let now_block_number = system::Module::<T>::block_number();

            let total_fee = user_scores
                .iter()
                .try_fold::<_, _, Result<Balance, DispatchError>>(
                    Zero::zero(),
                    |acc_amount, user_score| {
                        let fee = Self::do_refresh(&analyst, &user_score, &now_block_number)?;
                        acc_amount
                            .checked_add(fee)
                            .ok_or_else(|| Error::<T>::Overflow.into())
                    },
                )?;

            Self::mutate_payroll(&analyst, &total_fee, &(user_count as u32))?;

            T::Reputation::set_last_refresh_at();

            Self::deposit_event(Event::ReputationRefreshed(
                analyst,
                user_count as u32,
                total_fee,
            ));

            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn receiver_all(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            T::Reputation::end_refresh()?;

            let analyst = ensure_signed(origin)?;

            let payroll = Payrolls::<T>::take(&analyst);

            // TODO: 增加代理

            T::Currency::release(T::BaceToken::get(), &analyst, payroll.total_amount::<T>())?;

            <Records<T>>::remove_prefix(&analyst);
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    pub(crate) fn do_refresh(
        analyst: &T::AccountId,
        user_score: &(T::AccountId, u32),
        update_at: &T::BlockNumber,
    ) -> Result<Balance, DispatchError> {
        T::Reputation::refresh_reputation(&user_score)?;
        let who = &user_score.0;

        let fee = Self::share(who.clone())?;
        <Records<T>>::mutate(&analyst, &who, |_| Record { update_at, fee });
        Ok(fee)
    }

    pub(crate) fn mutate_payroll(
        analyst: &T::AccountId,
        amount: &Balance,
        count: &u32,
    ) -> DispatchResult {
        <Payrolls<T>>::try_mutate(&analyst, |f| -> DispatchResult {
            let total_fee = f
                .total_fee
                .checked_add(*amount)
                .ok_or(Error::<T>::Overflow)?;

            let count = f.count.checked_add(*count).ok_or(Error::<T>::Overflow)?;
            *f = Payroll { count, total_fee };
            Ok(())
        })
    }

    pub(crate) fn share(user: T::AccountId) -> Result<Balance, DispatchError> {
        let targets = T::TrustBase::get_trust_old(&user);
        let total_share = T::Currency::social_balance(T::BaceToken::get(), &user);

        T::Currency::bat_share(
            T::BaceToken::get(),
            &user,
            &targets,
            T::ShareRatio::get().mul_floor(total_share),
        )?;
        T::Currency::thaw(
            T::BaceToken::get(),
            &user,
            T::SelfRation::get().mul_floor(total_share),
        )?;
        let actor_amount = T::FeeRation::get().mul_floor(total_share);
        T::Currency::social_staking(T::BaceToken::get(), &user, actor_amount.clone())?;

        Ok(actor_amount)
    }
}

impl<T: Config> StartChallenge<T::AccountId, Balance> for Pallet<T> {
    fn start(target: &T::AccountId, analyst: &T::AccountId) -> Result<Balance, DispatchError> {
        let _ = T::Reputation::check_update_status(true).ok_or(Error::<T>::NoUpdatesAllowed)?;

        let record = <Records<T>>::take(&target, &analyst);

        ensure!(
            record.update_at + T::ConfirmationPeriod::get() > system::Module::<T>::block_number(),
            Error::<T>::ChallengeTimeout
        );

        Payrolls::<T>::mutate(&analyst, |f| Payroll {
            total_fee: f.total_fee.saturating_sub(record.fee),
            count: f.count.saturating_sub(1),
        });

        Ok(record.fee)
    }
}
