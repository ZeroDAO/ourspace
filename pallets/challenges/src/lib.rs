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
use zd_primitives::{factor, fee::ProxyFee, Amount, AppId, Balance, TIRStep};
use zd_traits::{ChallengeBase, Reputation};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
    traits::{AtLeast32Bit, Zero},
    DispatchError, DispatchResult, SaturatedConversion,
};

pub use pallet::*;

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

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Status {
    FREE,
    EXAMINE,
    REPLY,
    EVIDENCE,
    ARBITRATION,
}

impl Default for Status {
    fn default() -> Self {
        Status::EXAMINE
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct Metadata<AccountId, BlockNumber> {
    pub pool: Pool,
    pub joint_benefits: bool,
    pub progress: Progress<AccountId>,
    pub last_update: BlockNumber,
    pub remark: u32,
    pub score: u64,
    pub pathfinder: AccountId,
    pub status: Status,
    pub challenger: AccountId,
}

impl<AccountId, BlockNumber> Metadata<AccountId, BlockNumber>
where
    AccountId: Ord + Clone,
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
        self
    }

    fn set_status(&mut self, status: &Status) {
        self.status = *status;
    }

    fn restart(&mut self, full_probative: bool) {
        self.status = Status::FREE;
        self.joint_benefits = false;
        if full_probative {
            self.pathfinder = self.challenger.clone();
        }
    }
}

#[pallet]
pub mod pallet {
    use super::*;

    use frame_support::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord;
        type Currency: MultiCurrencyExtended<
                Self::AccountId,
                CurrencyId = Self::CurrencyId,
                Balance = Balance,
                Amount = Amount,
            > + StakingCurrency<Self::AccountId>;
        type Reputation: Reputation<Self::AccountId, Self::BlockNumber, TIRStep>;
        #[pallet::constant]
        type ReceiverProtectionPeriod: Get<Self::BlockNumber>;
        #[pallet::constant]
        type UpdateStakingAmount: Get<Balance>;
        #[pallet::constant]
        type ChallengePerior: Get<Self::BlockNumber>;
        #[pallet::constant]
        type BaceToken: Get<Self::CurrencyId>;
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
    #[pallet::getter(fn last_at)]
    pub type LastAt<T: Config> = StorageMap<_, Twox64Concat, AppId, T::BlockNumber, ValueQuery>;

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Launched a challenge. \[challenger, target, analyst, quantity\]
        Challenged(T::AccountId, T::AccountId, T::AccountId, u32),
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
    impl<T: Config> Pallet<T> {}
}

impl<T: Config> Pallet<T> {
    fn now() -> T::BlockNumber {
        system::Module::<T>::block_number()
    }

    pub(crate) fn staking(who: &T::AccountId, amount: Balance) -> DispatchResult {
        T::Currency::staking(T::BaceToken::get(), who, amount)
    }

    pub(crate) fn release(who: &T::AccountId, amount: Balance) -> DispatchResult {
        T::Currency::release(T::BaceToken::get(), who, amount)
    }

    pub(crate) fn checked_sweeper_fee(
        challenge: &Metadata<T::AccountId, T::BlockNumber>,
        who: &T::AccountId,
        total_amount: &Balance,
    ) -> Result<(Balance, Balance), DispatchError> {
        let is_sweeper = challenge.challenger != *who && challenge.pathfinder != *who;
        let now_block_number = system::Module::<T>::block_number();
        if is_sweeper {
            let (sweeper_fee, awards) = total_amount
                .checked_with_fee(challenge.last_update, now_block_number)
                .ok_or(Error::<T>::TooSoon)?;
            Ok((sweeper_fee, awards))
        } else {
            ensure!(
                challenge.last_update + T::ChallengePerior::get() > now_block_number,
                Error::<T>::TooSoon
            );
            Ok((Zero::zero(), *total_amount))
        }
    }

    pub(crate) fn remove(app_id: &AppId, target: &T::AccountId) {
        Metadatas::<T>::remove(&app_id, &target);
    }

    pub(crate) fn do_settle(
        challenge: &mut Metadata<T::AccountId, T::BlockNumber>,
        restart: &bool,
        joint_benefits: &bool,
        score: &u64,
    ) -> DispatchResult {
        match restart {
            true => {
                if *joint_benefits {
                    let arbitral_fee = challenge
                        .pool
                        .staking
                        .checked_div(2)
                        .ok_or(Error::<T>::Overflow)?;
                    challenge.pool.staking -= arbitral_fee;
                    Self::release(&challenge.challenger, arbitral_fee)?;
                }
                challenge.restart(!joint_benefits);
                Ok(())
            }
            false => {
                challenge.joint_benefits = *joint_benefits;
                challenge.score = *score;
                Ok(())
            }
        }
    }

    pub(crate) fn mutate_metadata(
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

    pub(crate) fn after_upload(app_id: &AppId) {
        <LastAt<T>>::mutate(*app_id, |l| Self::now());
    }

    fn is_challenge_timeout(challenge: &Metadata<T::AccountId, T::BlockNumber>) -> bool {
        let now_block_number = system::Module::<T>::block_number();
        now_block_number > challenge.last_update + T::ChallengePerior::get()
    }
}

impl<T: Config> ChallengeBase<T::AccountId, AppId, Balance, T::BlockNumber> for Pallet<T> {
    fn is_all_harvest(app_id: &AppId) -> bool {
        <Metadatas<T>>::iter_prefix_values(app_id).next().is_none()
    }

    fn is_all_timeout(app_id: &AppId, now: &T::BlockNumber) -> bool {
        let last = <LastAt<T>>::get(app_id);
        *now > last + T::ChallengePerior::get()
    }

    fn harvest(
        who: &T::AccountId,
        app_id: &AppId,
        target: &T::AccountId,
    ) -> Result<Option<u64>, DispatchError> {
        let challenge = Self::get_metadata(&app_id, &target);
        let total_amount: Balance = challenge.total_amount().ok_or(Error::<T>::Overflow)?;
        let (sweeper_fee, awards) = Self::checked_sweeper_fee(&challenge, &who, &total_amount)?;
        let mut pathfinder_amount: Balance = Zero::zero();
        let mut challenger_amount: Balance = Zero::zero();
        let mut maybe_score: Option<u64> = None;
        match challenge.status {
            Status::FREE | Status::REPLY => {
                pathfinder_amount = awards;
            }
            Status::EXAMINE | Status::EVIDENCE => {
                challenger_amount = awards;
                maybe_score = Some(challenge.score);
            }
            Status::ARBITRATION => match challenge.joint_benefits {
                true => {
                    pathfinder_amount = awards / 2;
                    challenger_amount = awards.saturating_sub(pathfinder_amount);
                }
                false => {
                    pathfinder_amount = awards;
                    maybe_score = Some(challenge.score);
                }
            },
        }
        if sweeper_fee > 0 {
            Self::release(&who, sweeper_fee)?;
        }
        if pathfinder_amount > 0 {
            Self::release(&challenge.pathfinder, sweeper_fee)?;
        }
        if challenger_amount > 0 {
            Self::release(&challenge.challenger, sweeper_fee)?;
        };
        Self::remove(&app_id, &target);
        Ok(maybe_score)
    }

    fn new(
        app_id: &AppId,
        who: &T::AccountId,
        path_finder: &T::AccountId,
        fee: Balance,
        staking: Balance,
        target: &T::AccountId,
        quantity: u32,
        score: u64,
    ) -> DispatchResult {
        let now_block_number = system::Module::<T>::block_number();

        let mut challenge = <Metadatas<T>>::try_get(app_id, target)
            .map_err(|_err| Error::<T>::NoChallengeAllowed)?;

        ensure!(
            challenge.is_allowed_evidence::<T::ChallengePerior>(now_block_number),
            Error::<T>::NoChallengeAllowed
        );

        challenge.pool.staking = challenge
            .pool
            .staking
            .checked_add(staking)
            .ok_or(Error::<T>::Overflow)?;
        challenge.pool.earnings = challenge
            .pool
            .earnings
            .checked_add(fee)
            .ok_or(Error::<T>::Overflow)?;
        challenge.progress = Progress {
            owner: who.clone(),
            done: Zero::zero(),
            total: quantity,
        };
        challenge.last_update = now_block_number;
        challenge.status = Status::EXAMINE;
        challenge.score = score;

        <Metadatas<T>>::try_mutate(app_id, target, |m| -> DispatchResult {
            *m = challenge;
            Self::staking(&who, factor::CHALLENGE_STAKING_AMOUNT)?;
            Ok(())
        })?;

        Self::after_upload(&app_id);

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
        count: &u32,
        mut up: impl FnMut(u64, u32, bool) -> Result<(u64, u32), DispatchError>,
    ) -> DispatchResult {
        Self::mutate_metadata(
            app_id,
            target,
            |challenge: &mut Metadata<T::AccountId, T::BlockNumber>| -> DispatchResult {
                ensure!(*count <= MAX_UPDATE_COUNT, Error::<T>::NoPermission);

                ensure!(
                    challenge.is_challenger(&who),
                    Error::<T>::NoChallengeAllowed
                );
                ensure!(
                    challenge.next(*count, who.clone()).check_progress(),
                    Error::<T>::TooMany
                );
                let is_all_done = challenge.is_all_done();
                let (score, remark) = up(challenge.score, challenge.remark, is_all_done)?;
                challenge.remark = remark;
                challenge.score = score;
                Self::after_upload(&app_id);
                Ok(())
            },
        )
    }

    fn examine(
        app_id: &AppId,
        who: T::AccountId,
        target: &T::AccountId,
        index: u32,
    ) -> DispatchResult {
        Self::mutate_metadata(
            app_id,
            target,
            |challenge: &mut Metadata<T::AccountId, T::BlockNumber>| -> DispatchResult {
                ensure!(
                    challenge.status == Status::REPLY && challenge.is_all_done(),
                    Error::<T>::NoChallengeAllowed
                );
                ensure!(
                    challenge.is_challenger(&who),
                    Error::<T>::NoChallengeAllowed
                );

                challenge.status = Status::EXAMINE;
                challenge.remark = index;

                Self::after_upload(&app_id);
                Ok(())
            },
        )
    }

    fn reply(
        app_id: &AppId,
        who: &T::AccountId,
        target: &T::AccountId,
        total: u32,
        count: u32,
        up: impl Fn(bool, u32) -> DispatchResult,
    ) -> DispatchResult {
        Self::mutate_metadata(
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

                up(is_all_done, challenge.remark)
            },
        )
    }

    fn evidence(
        app_id: &AppId,
        who: &T::AccountId,
        target: &T::AccountId,
        up: impl Fn(u32, u64) -> Result<bool, DispatchError>,
    ) -> Result<Option<u64>, DispatchError> {
        let mut challenge =
            <Metadatas<T>>::try_get(app_id, target).map_err(|_| Error::<T>::NoPermission)?;
        ensure!(challenge.is_challenger(&who), Error::<T>::NoPermission);
        ensure!(challenge.is_all_done(), Error::<T>::NoPermission);
        // TODO 权限检查，非质询状态下
        let needs_arbitration = up(challenge.remark, challenge.score)?;
        let score = challenge.score;
        match needs_arbitration {
            true => challenge.set_status(&Status::ARBITRATION),
            false => {
                challenge.restart(true);
            }
        };
        <Metadatas<T>>::mutate(app_id, target, |m| *m = challenge);
        Self::after_upload(&app_id);
        Ok(match needs_arbitration {
            false => Some(score),
            true => None,
        })
    }

    fn arbitral(
        app_id: &AppId,
        who: &T::AccountId,
        target: &T::AccountId,
        up: impl Fn(u64) -> Result<(bool, bool, u64), DispatchError>,
    ) -> DispatchResult {
        Self::mutate_metadata(
            app_id,
            target,
            |challenge: &mut Metadata<T::AccountId, T::BlockNumber>| -> DispatchResult {
                ensure!(challenge.is_all_done(), Error::<T>::NoPermission);
                if !challenge.is_challenger(&who) {
                    ensure!(
                        Self::is_challenge_timeout(&challenge),
                        Error::<T>::NoPermission
                    );
                    Self::staking(&who, factor::CHALLENGE_STAKING_AMOUNT)?;
                    challenge.challenger = who.clone();
                }
                let (joint_benefits, restart, score) = up(challenge.score)?;
                Self::do_settle(challenge, &restart, &joint_benefits, &score)?;
                Self::after_upload(&app_id);
                Ok(())
            },
        )
    }

    fn settle(
        app_id: &AppId,
        target: &T::AccountId,
        joint_benefits: bool,
        restart: bool,
        score: u64,
    ) -> DispatchResult {
        Self::mutate_metadata(
            app_id,
            target,
            |challenge: &mut Metadata<T::AccountId, T::BlockNumber>| -> DispatchResult {
                ensure!(challenge.is_all_done(), Error::<T>::NoPermission);
                Self::do_settle(challenge, &restart, &joint_benefits, &score)
            },
        )
    }
}
