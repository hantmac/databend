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

use std::sync::atomic::AtomicU64;

#[derive(Default)]
pub struct Profile {
    /// The id of processor
    pub pid: usize,
    /// The name of processor
    pub p_name: String,

    /// The time spent to process in nanoseconds
    pub cpu_time: AtomicU64,
    /// The time spent to wait in nanoseconds, usually used to
    /// measure the time spent on waiting for I/O
    pub wait_time: AtomicU64,
}

impl Profile {
    pub fn create(pid: usize, p_name: String) -> Profile {
        Profile {
            pid,
            p_name,
            cpu_time: AtomicU64::new(0),
            wait_time: AtomicU64::new(0),
        }
    }
}
