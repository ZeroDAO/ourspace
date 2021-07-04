#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

// use frame_support::{ensure, dispatch::DispatchResultWithPostInfo, pallet, pallet_prelude::*};
use frame_support::{
    codec::{Decode, Encode},
    ensure, pallet,
    traits::Get,
    RuntimeDebug,
};
use frame_system::{self as system};
use orml_traits::{MultiCurrencyExtended, StakingCurrency};
use zd_primitives::{factor, Amount, Balance};
use zd_traits::{Reputation, SeedsBase, StartChallenge, TrustBase};

use sp_runtime::DispatchResult;
use sp_runtime::{traits::Zero, DispatchError};
use sp_std::vec::Vec;

pub use pallet::*;

/// 有效路径最大数量
const MAX_PATH_COUNT: u32 = 5;

/// 单次最多长传路径
const MAX_UPDATE_COUNT: u32 = 10;

#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct Pool {
    pub staking: Balance,
    pub sub_staking: Balance,
    pub earnings: Balance,
}

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Default, RuntimeDebug)]
pub struct Progress<AccountId> {
    pub owner: AccountId,
    pub total: u32,
    pub done: u32,
}

impl<AccountId> Progress<AccountId> {
    fn is_all_done(&self) -> bool {
        self.total == self.done
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct Challenge<AccountId, BlockNumber> {
    pub pool: Pool,
    pub beneficiary: AccountId,
    pub progress: Progress<AccountId>,
    pub last_update: BlockNumber,
    pub score: u32,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct Path<AccountId> {
    pub nodes: Vec<AccountId>,
    pub score: u32,
}

impl<AccountId, BlockNumber> Challenge<AccountId, BlockNumber> {
    fn total_amount(&self) -> Option<Balance> {
        self.pool
            .staking
            .checked_add(self.pool.sub_staking)
            .and_then(|a| a.checked_add(self.pool.earnings))
    }
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

    use frame_system::{ensure_signed, pallet_prelude::*};

    use frame_support::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord;
        type BaceToken: Get<Self::CurrencyId>;
        type Currency: MultiCurrencyExtended<
                Self::AccountId,
                CurrencyId = Self::CurrencyId,
                Balance = Balance,
                Amount = Amount,
            > + StakingCurrency<Self::AccountId>;
        type Reputation: Reputation<Self::AccountId, Self::BlockNumber>;
        type StartChallenge: StartChallenge<Self::AccountId, Balance>;
        type TrustBase: TrustBase<Self::AccountId>;
        type SeedsBase: SeedsBase<Self::AccountId>;
        #[pallet::constant]
        type ReceiverProtectionPeriod: Get<Self::BlockNumber>;
        type ChallengePerior: Get<Self::BlockNumber>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn get_challenge)]
    pub type Challenges<T: Config> = StorageMap<
        _,
        Twox64Concat,
        T::AccountId,
        Challenge<T::AccountId, T::BlockNumber>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn last_update)]
    pub type LastUpdate<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_sub_challenge)]
    pub type SubChallenges<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Progress<T::AccountId>, ValueQuery>;

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
        /// 发起了一个挑战 \[challenger, target, analyst, quantity\]
        Challenged(T::AccountId, T::AccountId, T::AccountId, u32),
        /// new path \[challenger, target\]
        NewPath(T::AccountId, T::AccountId),
        /// 发起了一个二次挑战 \[challenger, target, count\]
        SubChallenged(T::AccountId, T::AccountId, u32),
        /// 领取收益 \[who, target, is_proxy\]
        ReceiveIncome(T::AccountId, T::AccountId, bool),
    }

    #[pallet::error]
    pub enum Error<T> {
        NoneValue,
        StorageOverflow,
        NoPermission,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn start_challenge(
            origin: OriginFor<T>,
            target: T::AccountId,
            analyst: T::AccountId,
            quantity: u32,
        ) -> DispatchResultWithPostInfo {
            let challenger = ensure_signed(origin)?;
            // TODO: 是否应该限制连续重复挑战？
            ensure!(
                quantity < T::SeedsBase::get_seed_count(),
                Error::<T>::NoPermission
            );
            let now_block_number = system::Module::<T>::block_number();
            Self::staking(&challenger, factor::CHALLENGE_STAKING_AMOUNT)?;
            let fee = T::StartChallenge::start(&target, &analyst)?;
            <Challenges<T>>::mutate(&target, |_| Challenge {
                pool: Pool {
                    staking: factor::CHALLENGE_STAKING_AMOUNT,
                    sub_staking: Zero::zero(),
                    earnings: fee,
                },
                progress: Progress {
                    owner: &analyst,
                    done: Zero::zero(),
                    total: quantity,
                },
                beneficiary: &challenger,
                last_update: now_block_number,
                score: Zero::zero(),
            });
            Self::after_upload(now_block_number)?;
            Self::deposit_event(Event::Challenged(challenger, target, analyst, quantity));
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn upload_path(
            origin: OriginFor<T>,
            target: T::AccountId,
            seeds: Vec<T::AccountId>,
            paths: Vec<Path<T::AccountId>>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let count = seeds.len();
            ensure!(count == paths.len(), Error::<T>::NoPermission);

            Challenges::<T>::try_mutate_exists(&target, |challenge| -> DispatchResult {
                let challenge = challenge.as_mut().ok_or(Error::<T>::NoPermission)?;
                let new_score = challenge.score;

                let is_end = if <SubChallenges<T>>::contains_key(&target) {
                    SubChallenges::<T>::try_mutate_exists(
                        &target,
                        |sub_challenge| -> Result<bool, DispatchError> {
                            let sub_challenge =
                                sub_challenge.as_mut().ok_or(Error::<T>::NoPermission)?;
                            let progress_info =
                                Self::get_new_progress(sub_challenge, &(count as u32), &who)?;
                            challenge.score =
                                Self::do_update_path_verify(&target, seeds, paths, new_score)?;
                            sub_challenge.done = progress_info.0;
                            Ok(progress_info.1)
                        },
                    )?
                } else {
                    let progress_info =
                        Self::get_new_progress(&challenge.progress, &(count as u32), &who)?;
                    let score = Self::do_update_path(&target, &seeds, &paths, new_score)?;
                    challenge.score = score;
                    challenge.progress.done = progress_info.0;
                    progress_info.1
                };
                if is_end {
                    challenge.beneficiary = who.clone()
                };
                Ok(())
            })?;
            Self::deposit_event(Event::NewPath(who, target));
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn start_sub_challenge(
            origin: OriginFor<T>,
            target: T::AccountId,
            quantity: u32,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let challenge = Self::get_challenge(&target);
            let now_block_number = system::Module::<T>::block_number();
            SubChallenges::<T>::try_mutate_exists(&target, |sub_challenge| -> DispatchResult {
                if !sub_challenge.is_some() {
                    ensure!(
                        Self::allow_sub_challenge(
                            &challenge.progress.is_all_done(),
                            &challenge.last_update,
                            now_block_number
                        ),
                        Error::<T>::NoPermission
                    );
                } else {
                    ensure!(
                        Self::allow_sub_challenge(
                            &sub_challenge.as_ref().unwrap().is_all_done(),
                            &challenge.last_update,
                            now_block_number
                        ),
                        Error::<T>::NoPermission
                    );
                }
                *sub_challenge = Some(Progress {
                    owner: who.clone(),
                    total: quantity,
                    done: Zero::zero(),
                });
                Ok(())
            })?;
            Challenges::<T>::mutate(&target, |c| c.last_update = now_block_number);
            Self::after_upload(now_block_number)?;
            Self::deposit_event(Event::SubChallenged(who, target, quantity));
            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn receive_income(
            origin: OriginFor<T>,
            target: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let challenge = Self::get_challenge(&target);

            let is_proxy = Self::checked_proxy(&challenge, &who)?;

            Self::remove(&target);

            let mut total_amount = challenge.total_amount().ok_or(Error::<T>::NoPermission)?;

            let old_ir = T::Reputation::get_reputation_new(&target).ok_or(Error::<T>::NoPermission)?;
            let analyst = challenge.progress.owner;

            if old_ir != challenge.score {
                T::Reputation::mutate_reputation(&target, challenge.score);
            }

            if challenge.beneficiary != analyst && old_ir == challenge.score {
                // 结算更新分成
                let analyst_sub_amount =
                    factor::ANALYST_RATIO.mul_floor(challenge.pool.sub_staking);
                let analyst_amount = challenge
                    .pool
                    .earnings
                    .checked_add(challenge.pool.staking)
                    .and_then(|a| a.checked_add(analyst_sub_amount))
                    .map(|a| Self::less_proxy(&a, is_proxy))
                    .ok_or(Error::<T>::NoPermission)?;

                let challenger_amount = challenge
                    .pool
                    .sub_staking
                    .checked_sub(analyst_sub_amount)
                    .map(|a| Self::less_proxy(&a, is_proxy))
                    .ok_or(Error::<T>::NoPermission)?;

                total_amount = total_amount
                    .checked_sub(analyst_amount)
                    .and_then(|a| a.checked_sub(challenger_amount))
                    .ok_or(Error::<T>::NoPermission)?;

                Self::release(&analyst, analyst_amount)?;
                Self::release(&challenge.beneficiary, challenger_amount)?;
            } else {
                let b_amount = Self::less_proxy(&total_amount, is_proxy);
                total_amount = total_amount
                    .checked_sub(b_amount)
                    .ok_or(Error::<T>::NoPermission)?;

                Self::release(
                    &challenge.beneficiary,
                    Self::less_proxy(&b_amount, is_proxy),
                )?;
            }

            if is_proxy {
                Self::release(&who, total_amount)?;
            }

            Self::deposit_event(Event::ReceiveIncome(who, target, is_proxy));
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    pub(crate) fn less_proxy(amount: &Balance, is_proxy: bool) -> Balance {
        if is_proxy {
            factor::PROXY_PICKUP_RATIO.mul_floor(amount.clone())
        } else {
            amount.clone()
        }
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

    pub(crate) fn staking(who: &T::AccountId, amount: Balance) -> DispatchResult {
        T::Currency::staking(T::BaceToken::get(), who, amount)
    }

    pub(crate) fn release(who: &T::AccountId, amount: Balance) -> DispatchResult {
        T::Currency::release(T::BaceToken::get(), who, amount)
    }

    pub(crate) fn checked_proxy(
        challenge: &Challenge<T::AccountId, T::BlockNumber>,
        who: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        let is_proxy = challenge.beneficiary != *who && challenge.progress.owner != *who;
        let now_block_number = system::Module::<T>::block_number();
        if is_proxy {
            ensure!(
                challenge.last_update + T::ReceiverProtectionPeriod::get() > now_block_number,
                Error::<T>::NoPermission
            );
        } else {
            ensure!(
                challenge.last_update + T::ChallengePerior::get() > now_block_number,
                Error::<T>::NoPermission
            );
        }
        Ok(is_proxy)
    }

    pub(crate) fn remove(target: &T::AccountId) {
        Challenges::<T>::remove(&target);
        SubChallenges::<T>::remove(&target);
        Paths::<T>::remove_prefix(&target);
    }

    pub(crate) fn get_new_progress(
        progress: &Progress<T::AccountId>,
        count: &u32,
        challenger: &T::AccountId,
    ) -> Result<(u32, bool), DispatchError> {
        ensure!(*count <= MAX_UPDATE_COUNT, Error::<T>::NoPermission);
        let new_done = progress.done + count;
        ensure!(progress.owner == *challenger, Error::<T>::NoPermission);
        ensure!(progress.total >= new_done, Error::<T>::NoPermission);
        Ok((new_done, progress.total == new_done))
    }

    pub(crate) fn allow_sub_challenge(
        update_end: &bool,
        last_update: &T::BlockNumber,
        now_block_number: T::BlockNumber,
    ) -> bool {
        if !update_end && *last_update + T::ChallengePerior::get() >= now_block_number {
            return false;
        }
        *last_update + T::ChallengePerior::get() < now_block_number
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
                    Error::<T>::NoPermission
                );
                ensure!(path.exclude_zero(), Error::<T>::NoPermission);
                Paths::<T>::insert(seed, target, path);
                acc.checked_add(path.score).ok_or(Error::<T>::NoPermission)
            })?;
        Ok(new_score.clone())
    }

    pub(crate) fn do_update_path_verify(
        target: &T::AccountId,
        seeds: Vec<T::AccountId>,
        paths: Vec<Path<T::AccountId>>,
        score: u32,
    ) -> Result<u32, DispatchError> {
        let new_score = seeds
            .iter()
            .zip(paths.iter())
            .try_fold(score, |acc, (seed, path)| {
                Paths::<T>::try_mutate_exists(&seed, &target, |p| -> Result<u32, DispatchError> {
                    let dist_new = Self::get_dist(&path, seed).ok_or(Error::<T>::NoPermission)?;
                    let old_path = p.take().unwrap_or_default();
                    if let Some(old_dist) = Self::get_dist(&old_path, &seed) {
                        ensure!(old_dist >= dist_new, Error::<T>::NoPermission);
                        ensure!(
                            old_dist == dist_new && old_path.score > path.score,
                            Error::<T>::NoPermission
                        );
                    }
                    let acc = acc
                        .checked_sub(old_path.score)
                        .and_then(|s| s.checked_add(path.score))
                        .ok_or(Error::<T>::NoPermission)?;
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

    pub(crate) fn after_upload(now: T::BlockNumber) -> DispatchResult {
        T::Reputation::last_challenge_at(&now);
        Ok(())
    }
}
