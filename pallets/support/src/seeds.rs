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

pub trait SeedsBase<AccountId> {

	/// Returns whether `seed` is the seed user.
	fn is_seed(seed: &AccountId) -> bool;

	/// Returns the number of seed users.
	fn get_seed_count() -> u32;

	/// Empty all seeds.
	fn remove_all();

	/// Add a seed.
	fn add_seed(new_seed: &AccountId);
}
