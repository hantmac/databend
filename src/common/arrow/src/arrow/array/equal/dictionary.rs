// Copyright 2021 Datafuse Labs
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

use crate::arrow::array::DictionaryArray;
use crate::arrow::array::DictionaryKey;

pub(super) fn equal<K: DictionaryKey>(lhs: &DictionaryArray<K>, rhs: &DictionaryArray<K>) -> bool {
    if !(lhs.data_type() == rhs.data_type() && lhs.len() == rhs.len()) {
        return false;
    };

    // if x is not valid and y is but its child is not, the slots are equal.
    lhs.iter().zip(rhs.iter()).all(|(x, y)| match (&x, &y) {
        (None, Some(y)) => !y.is_valid(),
        (Some(x), None) => !x.is_valid(),
        _ => x == y,
    })
}
