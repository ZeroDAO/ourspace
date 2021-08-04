#![cfg_attr(not(feature = "std"), no_std)]

// use frame_support::{ensure, dispatch::DispatchResultWithPostInfo, pallet, pallet_prelude::*};
use frame_support::{
    codec::{Decode, Encode},
    ensure, pallet,
    traits::Get,
    RuntimeDebug,
};
use frame_system::{self as system};
use orml_traits::{MultiCurrencyExtended, StakingCurrency};
use zd_primitives::{factor, Amount, AppId, Balance};
use zd_traits::{ChallengeBase, Reputation, SeedsBase, TrustBase};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
    traits::{AtLeast32Bit, Zero},
    DispatchError, DispatchResult, SaturatedConversion,
};

pub use pallet::*;

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

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Status {
    EXAMINE,
    REPLY,
    EVIDENCE,
}

impl Default for Status {
    fn default() -> Self {
        Status::EXAMINE
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct Metadata<AccountId, BlockNumber> {
    pub pool: Pool,
    pub beneficiary: AccountId,
    pub progress: Progress<AccountId>,
    pub last_update: BlockNumber,
    pub index: u32,
    pub value: u32,
    pub pathfinder: AccountId,
    pub status: Status,
    pub challenger: AccountId,
}

impl<AccountId, BlockNumber> Metadata<AccountId, BlockNumber>
where
    AccountId: Ord,
    BlockNumber: Copy + AtLeast32Bit,
{
    fn total_amount(&self) -> Option<Balance> {
        self.pool
            .staking
            .checked_add(self.pool.sub_staking)
            .and_then(|a| a.checked_add(self.pool.earnings))
    }

    fn is_allowed_evidence<ChallengePerior>(&self, now: BlockNumber) -> bool
    where
        ChallengePerior: Get<BlockNumber>,
    {
        let challenge_perior = ChallengePerior::get().saturated_into::<BlockNumber>();

        if !self.is_all_done() && self.last_update + challenge_perior >= now {
            return false;
        }
        self.last_update + challenge_perior < now
    }

    fn is_all_done(&self) -> bool {
        self.progress.total == self.progress.done
    }

    fn check_progress(&self) -> bool {
        self.progress.total >= self.progress.done
    }

    fn is_challenger(&self, who: &AccountId) -> bool {
        self.challenger == *who
    }

    fn is_pathfinder(&self, who: &AccountId) -> bool {
        self.pathfinder == *who
    }

    fn new_progress(&mut self, total: u32) -> &mut Self {
        self.progress.total = total;
        self
    }

    fn next(&mut self, count: u32, who: AccountId) -> &mut Self {
        self.progress.done = self.progress.done.saturating_add(count);
        if self.is_all_done() {
            self.beneficiary = who
        }
        self
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
        type TrustBase: TrustBase<Self::AccountId>;
        type SeedsBase: SeedsBase<Self::AccountId>;
        #[pallet::constant]
        type ReceiverProtectionPeriod: Get<Self::BlockNumber>;
        #[pallet::constant]
        type UpdateStakingAmount: Get<Balance>;
        #[pallet::constant]
        type ChallengePerior: Get<Self::BlockNumber>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn get_metadata)]
    pub type Metadatas<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        AppId,
        Twox64Concat,
        T::AccountId,
        Metadata<T::AccountId, T::BlockNumber>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn last_update)]
    pub type LastUpdate<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Launched a challenge. \[challenger, target, analyst, quantity\]
        Challenged(T::AccountId, T::AccountId, T::AccountId, u32),
        /// New path \[challenger, target\]
        NewPath(T::AccountId, T::AccountId),
        /// Launched a secondary challenge. \[challenger, target, count\]
        SubChallenged(T::AccountId, T::AccountId, u32),
        /// Receive benefits. \[who, target, is_proxy\]
        ReceiveIncome(T::AccountId, T::AccountId, bool),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// No permission.
        NoPermission,
        /// Paths and seeds do not match
        NotMatch,
        /// Calculation overflow.
        Overflow,
        /// No challenge allowed
        NoChallengeAllowed,
        /// Error getting user reputation
        ReputationError,
        /// Too soon
        TooSoon,
        /// Wrong progress
        ErrProgress,
        // Non-existent
        NonExistent,
        // Too many uploads
        TooMany,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn receive_income(
            origin: OriginFor<T>,
            app_id: AppId,
            target: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let challenge = Self::get_metadata(&app_id, &target);

            let is_proxy = Self::checked_proxy(&challenge, &who)?;

            Self::remove(&app_id, &target);

            let mut total_amount = challenge.total_amount().ok_or(Error::<T>::Overflow)?;

            let old_ir =
                T::Reputation::get_reputation_new(&target).ok_or(Error::<T>::ReputationError)?;
            let analyst = challenge.progress.owner;

            if old_ir != challenge.value {
                T::Reputation::mutate_reputation(&target, challenge.value);
            }

            if challenge.beneficiary != analyst && old_ir == challenge.value {
                // 结算更新分成
                let analyst_sub_amount =
                    factor::ANALYST_RATIO.mul_floor(challenge.pool.sub_staking);

                let analyst_amount = challenge
                    .pool
                    .earnings
                    .checked_add(challenge.pool.staking)
                    .and_then(|a| a.checked_add(analyst_sub_amount))
                    .map(|a| Self::less_proxy(&a, is_proxy))
                    .ok_or(Error::<T>::Overflow)?;

                let challenger_amount = challenge
                    .pool
                    .sub_staking
                    .checked_sub(analyst_sub_amount)
                    .map(|a| Self::less_proxy(&a, is_proxy))
                    .ok_or(Error::<T>::Overflow)?;

                total_amount = total_amount
                    .checked_sub(analyst_amount)
                    .and_then(|a| a.checked_sub(challenger_amount))
                    .ok_or(Error::<T>::Overflow)?;

                Self::release(&analyst, analyst_amount)?;
                Self::release(&challenge.beneficiary, challenger_amount)?;
            } else {
                let b_amount = Self::less_proxy(&total_amount, is_proxy);
                total_amount = total_amount
                    .checked_sub(b_amount)
                    .ok_or(Error::<T>::Overflow)?;

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

    pub(crate) fn staking(who: &T::AccountId, amount: Balance) -> DispatchResult {
        T::Currency::staking(T::BaceToken::get(), who, amount)
    }

    pub(crate) fn release(who: &T::AccountId, amount: Balance) -> DispatchResult {
        T::Currency::release(T::BaceToken::get(), who, amount)
    }

    pub(crate) fn checked_proxy(
        challenge: &Metadata<T::AccountId, T::BlockNumber>,
        who: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        let is_proxy = challenge.beneficiary != *who && challenge.progress.owner != *who;
        let now_block_number = system::Module::<T>::block_number();
        if is_proxy {
            ensure!(
                challenge.last_update + T::ReceiverProtectionPeriod::get() > now_block_number,
                Error::<T>::TooSoon
            );
        } else {
            ensure!(
                challenge.last_update + T::ChallengePerior::get() > now_block_number,
                Error::<T>::TooSoon
            );
        }
        Ok(is_proxy)
    }

    pub(crate) fn remove(app_id: &AppId, target: &T::AccountId) {
        Metadatas::<T>::remove(&app_id, &target);
    }

    pub(crate) fn get_new_progress(
        progress: &Progress<T::AccountId>,
        count: &u32,
        challenger: &T::AccountId,
    ) -> Result<(u32, bool), DispatchError> {
        ensure!(*count <= MAX_UPDATE_COUNT, Error::<T>::NoPermission);
        let new_done = progress.done + count;
        ensure!(progress.owner == *challenger, Error::<T>::NoPermission);
        ensure!(progress.total >= new_done, Error::<T>::ErrProgress);
        Ok((new_done, progress.total == new_done))
    }

    pub(crate) fn examine(
        app_id: &AppId,
        target: &T::AccountId,
        mut f: impl FnMut(&mut Metadata<T::AccountId, T::BlockNumber>) -> DispatchResult,
    ) -> DispatchResult {
        Metadatas::<T>::try_mutate_exists(app_id, target, |challenge| -> DispatchResult {
            let challenge = challenge.as_mut().ok_or(Error::<T>::NonExistent)?;
            f(challenge)
        })?;
        Ok(())
    }

    pub(crate) fn after_upload() -> DispatchResult {
        T::Reputation::last_challenge_at();
        Ok(())
    }
}

impl<T: Config> ChallengeBase<T::AccountId, AppId, Balance> for Pallet<T> {
    fn is_all_harvest(app_id: &AppId) -> bool {
        <Metadatas<T>>::iter_prefix_values(app_id).next().is_none()
    }

    fn new(
        app_id: &AppId,
        who: &T::AccountId,
        path_finder: &T::AccountId,
        fee: Balance,
        staking: Balance,
        target: &T::AccountId,
        quantity: u32,
        value: u32,
    ) -> DispatchResult {
        let now_block_number = system::Module::<T>::block_number();

        Self::staking(&who, factor::CHALLENGE_STAKING_AMOUNT)?;

        <Metadatas<T>>::try_mutate(app_id, target, |m| -> DispatchResult {
            // TODO 挑战未完成删除数据
            ensure!(
                m.is_allowed_evidence::<T::ChallengePerior>(now_block_number),
                Error::<T>::NoChallengeAllowed
            );

            m.pool.staking = m
                .pool
                .staking
                .checked_add(staking)
                .ok_or(Error::<T>::Overflow)?;
            m.pool.earnings = m
                .pool
                .earnings
                .checked_add(fee)
                .ok_or(Error::<T>::Overflow)?;
            m.progress = Progress {
                owner: who.clone(),
                done: Zero::zero(),
                total: quantity,
            };
            m.beneficiary = path_finder.clone();
            m.last_update = now_block_number;
            m.status = Status::EVIDENCE;
            m.value = value;

            Self::after_upload()?;

            Ok(())
        })?;

        Self::deposit_event(Event::Challenged(
            who.clone(),
            target.clone(),
            path_finder.clone(),
            quantity,
        ));

        Ok(())
    }

    fn next(
        app_id: &AppId,
        who: &T::AccountId,
        target: &T::AccountId,
        count: u32,
        up: impl FnOnce(bool, u32) -> Result<u32, DispatchError>,
    ) -> DispatchResult {
        Metadatas::<T>::try_mutate_exists(app_id, target, |challenge| -> DispatchResult {
            let challenge = challenge.as_mut().ok_or(Error::<T>::NonExistent)?;

            let progress_info = Self::get_new_progress(&challenge.progress, &(count as u32), &who)?;

            challenge.progress.done = progress_info.0;

            if progress_info.1 {
                challenge.beneficiary = who.clone()
            };

            // TODO 判断是否为首次

            let is_first = true;

            let value = up(is_first, challenge.value)?;

            challenge.value = value;

            Ok(())
        })?;

        Self::deposit_event(Event::NewPath(who.clone(), target.clone()));

        Ok(())
    }

    fn question(
        app_id: &AppId,
        who: T::AccountId,
        target: &T::AccountId,
        index: u32,
    ) -> DispatchResult {
        Self::examine(
            app_id,
            target,
            |challenge: &mut Metadata<T::AccountId, T::BlockNumber>| -> DispatchResult {

                ensure!(
                    challenge.status == Status::REPLY && challenge.is_all_done(),
                    Error::<T>::NoChallengeAllowed
                );
                ensure!(challenge.is_challenger(&who), Error::<T>::NoChallengeAllowed);

                challenge.status = Status::EXAMINE;
                challenge.index = index;
                challenge.beneficiary = who.clone();

                Ok(())
            },
        )
    }

    fn reply(
        app_id: &AppId,
        who: T::AccountId,
        target: &T::AccountId,
        total: u32,
        count: u32,
        up: impl Fn(bool,u32) -> DispatchResult,
    ) -> DispatchResult { 
        Self::examine(
            app_id,
            target,
            |challenge: &mut Metadata<T::AccountId, T::BlockNumber>| -> DispatchResult {
                ensure!(challenge.is_pathfinder(&who), Error::<T>::NoPermission);
                
                ensure!(
                    challenge.status == Status::EXAMINE,
                    Error::<T>::NoPermission
                );

                ensure!(
                    challenge
                        .new_progress(total)
                        .next(count, who.clone())
                        .check_progress(),
                    Error::<T>::TooMany
                );

                let is_all_done = challenge.is_all_done();

                if !is_all_done {
                    challenge.status = Status::REPLY;
                }

                up(is_all_done, challenge.index)
            },
        )
    }
}
