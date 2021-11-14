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

use sp_runtime::DispatchResult;

pub trait Reputation<AccountId, BlockNumber, TIRStep> {
    /// The first nonce in `target` has a reputation value that is modified.
    fn mutate_reputation(target: &AccountId, ir: &u32);

    /// Set `TIRStep` to `step`.
    fn set_step(step: &TIRStep);

    /// Whether the current `TIRStep` is `step`.
    fn is_step(step: &TIRStep) -> bool;

    /// Gets the reputation value under the latest `nonce` of the `target`
    /// system, or if the system is being updated, the may return an unverified
    /// reputation value.
    fn get_reputation_new(target: &AccountId) -> Option<u32>;

    /// Returns the latest verified reputation value of `target`.
    fn get_reputation(target: &AccountId) -> Option<u32>;

    /// Accepts a tuple of `AccountId`, `u32` and performs a refresh only if the
    /// user is not refreshed.
    fn refresh_reputation(user_score: &(AccountId, u32)) -> DispatchResult;

    /// Return to the system level for the last update block.
    fn get_last_refresh_at() -> BlockNumber;

    /// Modify the latest refresh time to the current block.
    fn set_last_refresh_at();

    /// Set the system status to `TIRStep::Free`.
    fn set_free();

    /// Start a new round.
    fn new_round() -> DispatchResult;
}
