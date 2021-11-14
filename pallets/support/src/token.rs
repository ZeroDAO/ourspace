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

pub trait MultiBaseToken<AccountId, Balance> {
    /// Get the current `bonus` amount.
    fn get_bonus_amount() -> Balance;

    /// `who` staking `amount`。
    fn staking(who: &AccountId, amount: &Balance) -> DispatchResult;

    /// Release the currency of `amount` to the account of `who`.
    fn release(who: &AccountId, amount: &Balance) -> DispatchResult;

    /// Returns the current `free_balance` of `who`.
    fn free_balance(who: &AccountId) -> Balance;

    /// Returns the current `social_balance` of `who`.
    fn social_balance(who: &AccountId) -> Balance;

    /// Split `who`s social currency proportionally and return the fee amount.
    fn share(who: &AccountId, target: &[AccountId]) -> Balance;

    /// `who` injects `bonus` in the amount of `amount` into the pool, prioritising 
    /// the deduction of `pending` from `who`.
    fn increase_bonus(who: &AccountId, amount: &Balance) -> DispatchResult;

    /// Direct reduction of `bonus` by the amount of `amount`.
    ///
    /// This is a low level operation and the caller should maintain the balance 
    /// of amounts themselves.
    fn cut_bonus(amount: &Balance) -> DispatchResult;

    /// Returns the `actual_balance` of `who`, including `pending`, `social` and `free`.
    fn actual_balance(who: &AccountId) -> Balance;

    /// Returns the `pending_balance` of `who`.
    fn pending_balance(who: &AccountId) -> Balance;

    /// `from` transfers currency in the amount of `amount` to the social account of `to`.
    fn transfer_social(from: &AccountId, to: &AccountId, amount: Balance) -> DispatchResult;

    /// Preference is given to currencies that use `pending` from `from` to transfer `amount` 
    /// amounts to `SocialPool` accounts.
    fn pay_with_pending(from: &AccountId, amount: Balance) -> DispatchResult;

    /// Take the `pending` out of `who`.
    fn claim(who: &AccountId) -> DispatchResult;
}
