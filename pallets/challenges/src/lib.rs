#![cfg_attr(not(feature = "std"), no_std)]

// use frame_support::{ensure, dispatch::DispatchResultWithPostInfo, pallet, pallet_prelude::*};
use frame_support::{
    codec::{Decode, Encode},
    ensure, pallet,
    traits::Get,
    RuntimeDebug,
};
use frame_system::{self as system};
use sp_std::convert::{TryFrom, TryInto};

use orml_traits::{MultiCurrency, StakingCurrency, arithmetic::{self, Signed}};
use zd_primitives::{fee::ProxyFee, AppId, Balance, TIRStep};
use zd_traits::{ChallengeBase, Reputation, ChallengeStatus};

use sp_runtime::{
    traits::{AtLeast32Bit, Zero},
    DispatchError, DispatchResult, SaturatedConversion,
};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

const MAX_UPDATE_COUNT: u32 = 257;

#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct Pool {
    pub staking: Balance,
    pub earnings: Balance,
}

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Default, RuntimeDebug)]
pub struct Progress {
    pub total: u32,
    pub done: u32,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct Metadata<AccountId, BlockNumber> {
    pub pool: Pool,
    pub joint_benefits: bool,
    pub progress: Progress,
    pub last_update: BlockNumber,
    pub remark: u32,
    pub score: u64,
    pub pathfinder: AccountId,
    pub status: ChallengeStatus,
    pub challenger: AccountId,
}

impl<AccountId, BlockNumber> Metadata<AccountId, BlockNumber>
where
    AccountId: Ord + Clone,
    BlockNumber: Copy + AtLeast32Bit,
{
    fn total_amount(&self) -> Option<Balance> {
        self.pool.staking.checked_add(self.pool.earnings)
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
        self.progress.done = Zero::zero();
        self
    }

    fn next(&mut self, count: u32) -> &mut Self {
        self.progress.done = self.progress.done.saturating_add(count);
        self
    }

    fn set_status(&mut self, status: &ChallengeStatus) {
        self.status = *status;
    }

    fn restart(&mut self, full_probative: bool) {
        self.status = ChallengeStatus::FREE;
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
        type Currency: MultiCurrency<
                Self::AccountId,
                CurrencyId = Self::CurrencyId,
                Balance = Balance,
            > + StakingCurrency<Self::AccountId>;
        type Amount: Signed
            + TryInto<Balance>
            + TryFrom<Balance>
            + Parameter
            + Member
            + arithmetic::SimpleArithmetic
            + Default
            + Copy
            + MaybeSerializeDeserialize;
        type Reputation: Reputation<Self::AccountId, Self::BlockNumber, TIRStep>;
        #[pallet::constant]
        type ChallengeTimeout: Get<Self::BlockNumber>;
        #[pallet::constant]
        type BaceToken: Get<Self::CurrencyId>;
        #[pallet::constant]
        type ChallengeStakingAmount: Get<Balance>;
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
        /// Non-existent
        NonExistent,
        /// Too many uploads
        TooMany,
        /// An error in progress has occurred
        ProgressErr,
        /// Status does not match
        StatusErr,
        /// Not available for collection
        NotAllowedSweeper,
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

    fn get_metadata_exist(
        app_id: &AppId,
        target: &T::AccountId,
    ) -> Result<Metadata<T::AccountId, T::BlockNumber>, Error<T>> {
        <Metadatas<T>>::try_get(&app_id, &target).map_err(|_err| Error::<T>::NonExistent)
    }

    fn get_challenge_timeout() -> T::BlockNumber {
        T::ChallengeTimeout::get().saturated_into::<T::BlockNumber>()
    }

    pub(crate) fn challenge_staking_amount() -> Balance {
        T::ChallengeStakingAmount::get()
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
                .ok_or(Error::<T>::NotAllowedSweeper)?;
            Ok((sweeper_fee, awards))
        } else {
            ensure!(
                Self::is_challenge_timeout(&challenge.last_update),
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
            f(challenge)?;
            challenge.last_update = Self::now();
            Ok(())
        })?;
        Ok(())
    }

    pub(crate) fn after_upload(app_id: &AppId) {
        <LastAt<T>>::mutate(*app_id, |l| *l = Self::now());
    }

    pub(crate) fn is_challenge_timeout(last_update: &T::BlockNumber) -> bool {
        let now_block_number = system::Module::<T>::block_number();
        now_block_number > (Self::get_challenge_timeout() + *last_update)
    }
}

impl<T: Config> ChallengeBase<T::AccountId, AppId, Balance, T::BlockNumber> for Pallet<T> {

    fn is_all_harvest(app_id: &AppId) -> bool {
        <Metadatas<T>>::iter_prefix_values(app_id).next().is_none()
    }

    fn is_all_timeout(app_id: &AppId, now: &T::BlockNumber) -> bool {
        let last = LastAt::<T>::get(app_id);
        *now > last + Self::get_challenge_timeout()
    }

    fn set_status(app_id: &AppId, target: &T::AccountId,status: &ChallengeStatus) {
        <Metadatas<T>>::mutate(app_id,target,|c|c.set_status(status));
    }

    fn harvest(
        who: &T::AccountId,
        app_id: &AppId,
        target: &T::AccountId,
    ) -> Result<Option<u64>, DispatchError> {
        let challenge = Self::get_metadata_exist(&app_id, &target)?;
        let total_amount: Balance = challenge.total_amount().ok_or(Error::<T>::Overflow)?;
        let (sweeper_fee, awards) = Self::checked_sweeper_fee(&challenge, &who, &total_amount)?;
        let mut pathfinder_amount: Balance = Zero::zero();
        let mut challenger_amount: Balance = Zero::zero();
        let mut maybe_score: Option<u64> = None;
        let is_all_done = challenge.is_all_done();
        match challenge.status {
            ChallengeStatus::FREE => {
                pathfinder_amount = awards;
            }
            ChallengeStatus::REPLY => match is_all_done {
                true => {
                    pathfinder_amount = awards;
                }
                false => {
                    challenger_amount = awards;
                }
            },
            ChallengeStatus::EXAMINE => {
                challenger_amount = awards;
                maybe_score = Some(challenge.score);
            }
            ChallengeStatus::EVIDENCE => {
                maybe_score = Some(challenge.score);
                match is_all_done {
                    true => {
                        challenger_amount = awards;
                    }
                    false => {
                        pathfinder_amount = awards;
                    }
                }
            }
            ChallengeStatus::ARBITRATION => match challenge.joint_benefits {
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
            Self::release(&challenge.pathfinder, pathfinder_amount)?;
        }
        if challenger_amount > 0 {
            Self::release(&challenge.challenger, challenger_amount)?;
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
        remark: u32,
    ) -> DispatchResult {
        let now_block_number = system::Module::<T>::block_number();

        let mut challenge = match <Metadatas<T>>::try_get(app_id, target) {
            Ok(challenge_storage) => {
                ensure!(
                    challenge_storage.status == ChallengeStatus::FREE,
                    Error::<T>::NoChallengeAllowed
                );
                challenge_storage
            },
            Err(_) => Metadata::default(),
        };

        challenge.pool.staking = challenge
            .pool
            .staking
            .checked_add(staking)
            .and_then(|v| v.checked_add(Self::challenge_staking_amount()))
            .ok_or(Error::<T>::Overflow)?;
        challenge.pool.earnings = challenge
            .pool
            .earnings
            .checked_add(fee)
            .ok_or(Error::<T>::Overflow)?;
        challenge.progress = Progress {
            done: Zero::zero(),
            total: quantity,
        };
        challenge.last_update = now_block_number;
        challenge.status = ChallengeStatus::EXAMINE;
        challenge.score = score;
        challenge.remark = remark;
        challenge.pathfinder = path_finder.clone();
        challenge.challenger = who.clone();

        <Metadatas<T>>::try_mutate(app_id, target, |m| -> DispatchResult {
            *m = challenge;
            Self::staking(&who, Self::challenge_staking_amount())?;
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
                ensure!(*count <= MAX_UPDATE_COUNT, Error::<T>::TooMany);

                match challenge.status {
                    ChallengeStatus::REPLY => {
                        ensure!(
                            challenge.is_pathfinder(&who),
                            Error::<T>::NoPermission
                        );
                    },
                    _ => {
                        ensure!(
                            challenge.is_challenger(&who),
                            Error::<T>::NoPermission
                        );
                    },
                }

                ensure!(challenge.next(*count).check_progress(), Error::<T>::ProgressErr);
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
                    challenge.status == ChallengeStatus::REPLY && challenge.is_all_done(),
                    Error::<T>::NoChallengeAllowed
                );
                ensure!(
                    challenge.is_challenger(&who),
                    Error::<T>::NoPermission
                );

                challenge.status = ChallengeStatus::EXAMINE;
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
        up: impl Fn(bool, u32, u64) -> Result<u64, DispatchError>,
    ) -> DispatchResult {
        Self::mutate_metadata(
            app_id,
            target,
            |challenge: &mut Metadata<T::AccountId, T::BlockNumber>| -> DispatchResult {
                ensure!(challenge.is_pathfinder(&who), Error::<T>::NoPermission);
                ensure!(
                    challenge.status == ChallengeStatus::EXAMINE,
                    Error::<T>::StatusErr
                );
                ensure!(
                    challenge.new_progress(total).next(count).check_progress(),
                    Error::<T>::ProgressErr
                );
                challenge.status = ChallengeStatus::REPLY;
                challenge.score = up(challenge.is_all_done(), challenge.remark, challenge.score)?;
                Ok(())
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
            <Metadatas<T>>::try_get(app_id, target).map_err(|_| Error::<T>::NonExistent)?;
        ensure!(challenge.is_challenger(&who), Error::<T>::NoPermission);
        ensure!(challenge.is_all_done(), Error::<T>::ProgressErr);
        ensure!(challenge.status != ChallengeStatus::EXAMINE, Error::<T>::StatusErr);
        let needs_arbitration = up(challenge.remark, challenge.score)?;
        let score = challenge.score;
        match needs_arbitration {
            true => challenge.set_status(&ChallengeStatus::ARBITRATION),
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
        up: impl Fn(u64,u32) -> Result<(bool, bool, u64), DispatchError>,
    ) -> DispatchResult {
        Self::mutate_metadata(
            app_id,
            target,
            |challenge: &mut Metadata<T::AccountId, T::BlockNumber>| -> DispatchResult {
                ensure!(challenge.status != ChallengeStatus::EXAMINE, Error::<T>::StatusErr);
                ensure!(challenge.is_all_done(), Error::<T>::ProgressErr);
                if !challenge.is_challenger(&who) {
                    ensure!(
                       Self::is_challenge_timeout(&challenge.last_update),
                       Error::<T>::NoPermission
                    );
                    Self::staking(&who, Self::challenge_staking_amount())?;
                    challenge.challenger = who.clone();
                }
                let (joint_benefits, restart, score) = up(challenge.score,challenge.remark)?;
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
                ensure!(challenge.is_all_done(), Error::<T>::ProgressErr);
                Self::do_settle(challenge, &restart, &joint_benefits, &score)
            },
        )
    }
}
