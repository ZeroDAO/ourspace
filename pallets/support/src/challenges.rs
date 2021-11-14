// Copyright 2021 ZeroDAO
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use sp_runtime::{DispatchError, DispatchResult};
use zd_primitives::{ChallengeStatus, Metadata};

pub trait ChallengeBase<AccountId, AppId, Balance, BlockNumber> {
    /// Directly modify the data of the challenge game.
    fn set_metadata(
        app_id: &AppId,
        target: &AccountId,
        metadata: &Metadata<AccountId, BlockNumber>,
    );

    /// Whether the challenges under `app_id` are all settled.
    fn is_all_harvest(app_id: &AppId) -> bool;

    /// Whether all the challenges under `app_id` have exceeded the challenge time.
    fn is_all_timeout(app_id: &AppId, now: &BlockNumber) -> bool;

    /// Set the status of the challenge against `target` under `app_id`.
    fn set_status(app_id: &AppId, target: &AccountId, status: &ChallengeStatus);

    /// Launch a challenge against `target` under `app_id`, `metadata` is used to
    /// set the initial challenge status.
    fn launch(
        app_id: &AppId,
        target: &AccountId,
        metadata: &Metadata<AccountId, BlockNumber>,
    ) -> DispatchResult;

    /// Continued uploading `count` data from the challenge against `target` under
    /// `app_id`, `who` is used to verify the original initiator.
    ///
    /// `up` passes three arguments to the caller.
    ///
    /// - `score` - The score currently recorded for this challenge.
    /// - `remark` - The current note information for this challenge makes it easy
    /// for the caller to record information about the challenge.
    /// - `is_all_done` Whether the data has all been uploaded.
    ///
    /// Execute only when `up` returns `Ok()` and updates the returned `score` and
    /// `remark`.
    fn next(
        app_id: &AppId,
        who: &AccountId,
        target: &AccountId,
        count: &u32,
        up: impl FnMut(u64, u32, bool) -> Result<(u64, u32), DispatchError>,
    ) -> DispatchResult;

    /// Challenge the data under `index`
    fn examine(app_id: &AppId, who: &AccountId, target: &AccountId, index: u32) -> DispatchResult;

    /// In response to the `examine` query, you need to upload a total of
    /// `total` data. This upload `count` entries.
    ///
    /// `up` passes three arguments to the caller.
    ///
    /// - `is_all_done` - Whether the data has all been uploaded.
    /// - `score` - The score currently recorded for this challenge.
    /// - `remark` - The current note information for this challenge makes
    /// it easy for the caller to record information about the challenge.
    ///
    /// Update the challenge only when `up` returns `Ok()`, and update `score`.
    fn reply(
        app_id: &AppId,
        who: &AccountId,
        target: &AccountId,
        total: u32,
        count: u32,
        up: impl Fn(bool, u32, u64) -> Result<u64, DispatchError>,
    ) -> DispatchResult;

    /// Submitting evidence
    ///
    /// `up` passes two arguments to the caller.
    ///
    /// - `remark` - The current note information for this challenge makes
    /// it easy for the caller to record information about the challenge.
    /// - `score` - Challenge the current recorded score.
    ///
    /// Update the challenge only when `up` returns `Ok(needs_arbitration)`
    /// and enter the corresponding state according to its value.
    ///
    /// - `true` - A successful challenge will be initialised with `restart`.
    /// - `false` - The evidence is not strong enough and the challenge will
    /// go to arbitration.
    fn evidence(
        app_id: &AppId,
        who: &AccountId,
        target: &AccountId,
        up: impl Fn(u32, u64) -> Result<bool, DispatchError>,
    ) -> Result<Option<u64>, DispatchError>;

    /// Arbitration of submitted data, this is generally used for data that
    /// cannot be computed directly on the chain but can be verified, for
    /// example **shortest path**.
    ///
    /// `up` passes two arguments to the caller.
    ///
    /// - `score` - The score currently recorded for this challenge.
    /// - `remark` - The current note information for this challenge makes
    /// it easy for the caller to record information about the challenge.
    ///
    /// Update the challenge only when `up` returns `Ok(joint_benefits, restart, score)`
    /// and enter the corresponding state according to its value.
    ///
    /// - `joint_benefits` - If `true`, `pathfinder` and `challenger` will
    /// act as co-beneficiaries.
    /// - `restart` - Whether the challenge needs to be initialized, when the
    /// original `challenger` will accept the challenge.
    /// - `score` - Record the score to the challenge system.
    fn arbitral(
        app_id: &AppId,
        who: &AccountId,
        target: &AccountId,
        up: impl Fn(u64, u32) -> Result<(bool, bool, u64), DispatchError>,
    ) -> DispatchResult;

    /// Receive the challenge benefits. Assigned according to `ChallengeStatus`,
    /// `is_all_done`, `joint_benefits`.
    ///
    /// |            |    Free    |    Reply   |   Examine  |  Evidence  |
    /// |------------|------------|------------|------------|------------|
    /// |    Done    | pathfinder | pathfinder | challenger | challenger |
    /// |------------|------------|------------|------------|------------|
    /// | Disruption | pathfinder | challenger | challenger | pathfinder |
    ///
    /// In the `Arbitral` state, settlement is according to `joint_benefits`, if
    /// `true`, then `pathfinder` and `challenger`, otherwise all rewards go to
    /// `pathfinder`.
    fn harvest(
        who: &AccountId,
        app_id: &AppId,
        target: &AccountId,
    ) -> Result<Option<u64>, DispatchError>;

    /// Settle the current challenge. This is a low level operation.
    ///
	/// When `restart` is `true`, the challenge will be set to the `Free` state,
    /// and when `joint_benefits` is
	///  - `true` - The prize pool will be divided equally and sent to `challenger`.
	///  - `false` - Modify challenge data directly.
	///
	/// When `restart` is `false`, modify `joint_benefits` and `score` in the
    /// challenge system.
    fn settle(
        app_id: &AppId,
        target: &AccountId,
        joint_benefits: bool,
        restart: bool,
        score: u64,
    ) -> DispatchResult;
}
