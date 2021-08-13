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
use zd_primitives::{fee::ProxyFee, AppId, Balance, TIRStep};
use zd_traits::{ChallengeBase, Reputation, SeedsBase, TrustBase};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

const APP_ID: AppId = *b"repu    ";

/// 有效路径最大数量
const MAX_PATH_COUNT: u32 = 5;

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

#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct Path<AccountId> {
    pub nodes: Vec<AccountId>,
    pub score: u32,
}

impl<AccountId> Path<AccountId> {
    fn check_nodes_leng(&self) -> bool {
        self.nodes.len() as u32 <= MAX_PATH_COUNT
    }

    fn exclude_zero(&self) -> bool {
        self.nodes.len() as u32 <= MAX_PATH_COUNT && !self.nodes.is_empty() && !self.score.is_zero()
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
        #[pallet::constant]
        type RefRepuTiomeOut: Get<Self::BlockNumber>;
        type Reputation: Reputation<Self::AccountId, Self::BlockNumber, TIRStep>;
        type TrustBase: TrustBase<Self::AccountId>;
        type SeedsBase: SeedsBase<Self::AccountId>;
        type ChallengeBase: ChallengeBase<Self::AccountId, AppId, Balance, Self::BlockNumber>;
    }
    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn started_at)]
    pub type StartedAt<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

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

    #[pallet::storage]
    #[pallet::getter(fn get_path)]
    pub type Paths<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        T::AccountId,
        Twox64Concat,
        T::AccountId,
        Path<T::AccountId>,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Some reputations have been updated. \[pathfinder, count, fee\]
        ReputationRefreshed(T::AccountId, u32, Balance),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Quantity reaches limit.
        QuantityLimitReached,
        /// Not in the update period.
        NoUpdatesAllowed,
        /// Error getting fee.
        ErrorFee,
        /// Challenge timeout.
        ChallengeTimeout,
        /// Calculation overflow.
        Overflow,
        /// Calculation overflow.
        FailedProxy,
        /// The presence of unharvested challenges.
        ChallengeNotClaimed,
        /// Excessive number of seeds
        ExcessiveBumberOfSeeds,
        /// Error getting user reputation
        ReputationError,
        /// The path already exists
        PathAlreadyExist,
        /// Wrong path
        WrongPath,
        /// Error calculating dist
        DistErr,
        /// The dist is too long or score is too low.
        DistTooLong,
        /// Paths and seeds do not match
        NotMatch,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn start(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::check_step()?;

            ensure!(
                T::ChallengeBase::is_all_harvest(&APP_ID),
                Error::<T>::ChallengeNotClaimed
            );

            let last = T::Reputation::get_last_refresh_at();
            let now = system::Module::<T>::block_number();

            ensure!(
                Balance::is_allowed_proxy(last, now),
                Error::<T>::ChallengeTimeout
            );

            let total_fee = Payrolls::<T>::drain()
                .try_fold::<_, _, Result<Balance, DispatchError>>(
                    Zero::zero(),
                    |acc: Balance, (pathfinder, payroll)| {
                        let (proxy_fee, without_fee) = payroll.total_amount::<T>().with_fee();

                        T::Currency::release(T::BaceToken::get(), &pathfinder, without_fee)?;

                        acc.checked_add(proxy_fee)
                            .ok_or(Error::<T>::Overflow.into())
                    },
                )?;
            T::Currency::release(T::BaceToken::get(), &who, total_fee)?;
            <StartedAt<T>>::put(now);
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn refresh(
            origin: OriginFor<T>,
            user_scores: Vec<(T::AccountId, u32)>,
        ) -> DispatchResultWithPostInfo {
            let pathfinder = ensure_signed(origin)?;
            let user_count = user_scores.len();
            ensure!(
                user_count as u32 <= T::MaxUpdateCount::get(),
                Error::<T>::QuantityLimitReached
            );
            Self::check_step()?;
            let now_block_number = system::Module::<T>::block_number();
            Self::check_timeout(&now_block_number)?;

            let amount = T::UpdateStakingAmount::get()
                .checked_mul(user_count as Balance)
                .ok_or(Error::<T>::Overflow)?;
            T::Currency::staking(T::BaceToken::get(), &pathfinder, amount)?;
            let total_fee = user_scores
                .iter()
                .try_fold::<_, _, Result<Balance, DispatchError>>(
                    Zero::zero(),
                    |acc_amount, user_score| {
                        let fee = Self::do_refresh(&pathfinder, &user_score, &now_block_number)?;
                        acc_amount
                            .checked_add(fee)
                            .ok_or_else(|| Error::<T>::Overflow.into())
                    },
                )?;
            Self::mutate_payroll(&pathfinder, &total_fee, &(user_count as u32))?;

            T::Reputation::set_last_refresh_at();

            Self::deposit_event(Event::ReputationRefreshed(
                pathfinder,
                user_count as u32,
                total_fee,
            ));
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn harvest_ref_all(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let pathfinder = ensure_signed(origin)?;
            Self::check_step()?;
            T::Reputation::set_free();
            let payroll = Payrolls::<T>::take(&pathfinder);
            T::Currency::release(
                T::BaceToken::get(),
                &pathfinder,
                payroll.total_amount::<T>(),
            )?;
            <Records<T>>::remove_prefix(&pathfinder);
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn harvest_ref_all_proxy(
            origin: OriginFor<T>,
            pathfinder: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let proxy = ensure_signed(origin)?;
            Self::check_step()?;
            T::Reputation::set_free();
            let last = T::Reputation::get_last_update_at();
            let payroll = Payrolls::<T>::take(&pathfinder);
            let (proxy_fee, without_fee) = payroll
                .total_amount::<T>()
                .checked_with_fee(last, system::Module::<T>::block_number())
                .ok_or(Error::<T>::FailedProxy)?;
            <Records<T>>::remove_prefix(&pathfinder);
            T::Currency::release(T::BaceToken::get(), &proxy, proxy_fee)?;
            T::Currency::release(T::BaceToken::get(), &pathfinder, without_fee)?;
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn harvest_challenge(
            origin: OriginFor<T>,
            target: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::check_step()?;
            T::ChallengeBase::harvest(&who, &APP_ID, &target)?;
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn challenge(
            origin: OriginFor<T>,
            target: T::AccountId,
            pathfinder: T::AccountId,
            quantity: u32,
            score: u32,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
            ensure!(
                quantity < T::SeedsBase::get_seed_count(),
                Error::<T>::ExcessiveBumberOfSeeds
            );
            let reputation =
                T::Reputation::get_reputation_new(&target).ok_or(Error::<T>::ReputationError)?;
            ensure!(score != reputation, Error::<T>::ChallengeTimeout);

            let record = <Records<T>>::take(&target, &pathfinder);

            ensure!(
                record.update_at + T::ConfirmationPeriod::get()
                    > system::Module::<T>::block_number(),
                Error::<T>::ChallengeTimeout
            );

            Payrolls::<T>::mutate(&pathfinder, |f| Payroll {
                total_fee: f.total_fee.saturating_sub(record.fee),
                count: f.count.saturating_sub(1),
            });

            T::ChallengeBase::new(
                &APP_ID,
                &challenger,
                &pathfinder,
                record.fee,
                Zero::zero(),
                &target,
                quantity,
                reputation.into(),
            )?;

            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn arbitral(
            origin: OriginFor<T>,
            target: T::AccountId,
            seeds: Vec<T::AccountId>,
            paths: Vec<Path<T::AccountId>>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            Self::check_step()?;
            let count = seeds.len();
            ensure!(count == paths.len(), Error::<T>::NotMatch);
            T::ChallengeBase::arbitral(
                &APP_ID,
                &who,
                &target,
                |score| -> Result<(bool, bool, u64), _> {
                    let score = score as u32;
                    let new_score =
                        Self::do_update_path_verify(&target, &seeds, &paths, score)?;
                    T::Reputation::mutate_reputation(&target, &new_score);
                    Ok((new_score == score, false, new_score.into()))
                },
            )?;

            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn challenge_update(
            origin: OriginFor<T>,
            target: T::AccountId,
            seeds: Vec<T::AccountId>,
            paths: Vec<Path<T::AccountId>>,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            Self::check_step()?;
            let count = seeds.len();
            ensure!(count == paths.len(), Error::<T>::NotMatch);

            T::ChallengeBase::next(
                &APP_ID,
                &challenger,
                &target,
                &(count as u32),
                |score, remark, is_all_done| -> Result<(u64, u32), DispatchError> {
                    let new_score = Self::do_update_path(&target, &seeds, &paths, score as u32)?;
                    if is_all_done {
                        T::Reputation::mutate_reputation(&target, &new_score);
                    }
                    Ok((new_score as u64, remark))
                },
            )?;
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    fn check_step() -> DispatchResult {
        ensure!(
            T::Reputation::is_step(&TIRStep::REPUTATION),
            Error::<T>::DistTooLong
        );
        Ok(())
    }

    fn next_step() {
        let now = system::Module::<T>::block_number();
        let is_ref_timeout = Self::check_timeout(&now).is_err();
        let is_last_ref_timeout =
            T::Reputation::get_last_refresh_at() + T::ConfirmationPeriod::get() > now;
        let is_cha_all_timeout = T::ChallengeBase::is_all_timeout(&APP_ID, &now);
        if (is_last_ref_timeout || is_ref_timeout) && is_cha_all_timeout {
            T::TrustBase::remove_all_tmp();
            T::Reputation::set_free();
        }
    }

    fn check_timeout(now: &T::BlockNumber) -> DispatchResult {
        ensure!(
            *now < <StartedAt<T>>::get() + T::RefRepuTiomeOut::get(),
            Error::<T>::DistTooLong
        );
        Ok(())
    }

    pub(crate) fn do_refresh(
        pathfinder: &T::AccountId,
        user_score: &(T::AccountId, u32),
        update_at: &T::BlockNumber,
    ) -> Result<Balance, DispatchError> {
        T::Reputation::refresh_reputation(&user_score)?;
        let who = &user_score.0;

        let fee = Self::share(who.clone())?;
        <Records<T>>::mutate(&pathfinder, &who, |_| Record { update_at, fee });
        Ok(fee)
    }

    pub(crate) fn mutate_payroll(
        pathfinder: &T::AccountId,
        amount: &Balance,
        count: &u32,
    ) -> DispatchResult {
        <Payrolls<T>>::try_mutate(&pathfinder, |f| -> DispatchResult {
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

    pub(crate) fn get_dist(paths: &Path<T::AccountId>, seed: &T::AccountId) -> Option<u32> {
        if !paths.nodes.is_empty() && paths.check_nodes_leng() {
            let mut nodes = paths.nodes.clone();
            nodes.insert(0, seed.clone());
            if let Ok((dist, score)) = T::TrustBase::computed_path(&nodes) {
                if score == paths.score {
                    return Some(dist);
                }
            }
        }
        None
    }

    pub(crate) fn do_update_path(
        target: &T::AccountId,
        seeds: &Vec<T::AccountId>,
        paths: &Vec<Path<T::AccountId>>,
        score: u32,
    ) -> Result<u32, DispatchError> {
        let new_score = seeds
            .iter()
            .zip(paths.iter())
            .try_fold(score, |acc, (seed, path)| {
                ensure!(
                    !Paths::<T>::contains_key(seed, target),
                    Error::<T>::PathAlreadyExist
                );
                ensure!(path.exclude_zero(), Error::<T>::WrongPath);
                Paths::<T>::insert(seed, target, path);
                acc.checked_add(path.score).ok_or(Error::<T>::Overflow)
            })?;
        Ok(new_score.clone())
    }

    pub(crate) fn do_update_path_verify(
        target: &T::AccountId,
        seeds: &Vec<T::AccountId>,
        paths: &Vec<Path<T::AccountId>>,
        score: u32,
    ) -> Result<u32, DispatchError> {
        let new_score = seeds
            .iter()
            .zip(paths.iter())
            .try_fold(score, |acc, (seed, path)| {
                Paths::<T>::try_mutate_exists(&seed, &target, |p| -> Result<u32, DispatchError> {
                    let dist_new = Self::get_dist(&path, seed).ok_or(Error::<T>::DistErr)?;
                    let old_path = p.take().unwrap_or_default();
                    if let Some(old_dist) = Self::get_dist(&old_path, &seed) {
                        ensure!(old_dist >= dist_new, Error::<T>::DistTooLong);
                        ensure!(
                            old_dist == dist_new && old_path.score > path.score,
                            Error::<T>::DistTooLong
                        );
                    }
                    let acc = acc
                        .checked_sub(old_path.score)
                        .and_then(|s| s.checked_add(path.score))
                        .ok_or(Error::<T>::Overflow)?;
                    *p = if path.score == 0 {
                        None
                    } else {
                        Some(path.clone())
                    };
                    Ok(acc)
                })
            })?;
        Ok(new_score)
    }
}
