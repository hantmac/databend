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

// automatically generated by rust-bindgen 0.59.2

/// ABI-compatible struct for [`ArrowSchema`](https://arrow.apache.org/docs/format/CDataInterface.html#structure-definitions)
#[repr(C)]
#[derive(Debug)]
pub struct ArrowSchema {
    pub(super) format: *const ::std::os::raw::c_char,
    pub(super) name: *const ::std::os::raw::c_char,
    pub(super) metadata: *const ::std::os::raw::c_char,
    pub(super) flags: i64,
    pub(super) n_children: i64,
    pub(super) children: *mut *mut ArrowSchema,
    pub(super) dictionary: *mut ArrowSchema,
    pub(super) release: ::std::option::Option<unsafe extern "C" fn(arg1: *mut ArrowSchema)>,
    pub(super) private_data: *mut ::std::os::raw::c_void,
}

/// ABI-compatible struct for [`ArrowArray`](https://arrow.apache.org/docs/format/CDataInterface.html#structure-definitions)
#[repr(C)]
#[derive(Debug)]
pub struct ArrowArray {
    pub(super) length: i64,
    pub(super) null_count: i64,
    pub(super) offset: i64,
    pub(super) n_buffers: i64,
    pub(super) n_children: i64,
    pub(super) buffers: *mut *const ::std::os::raw::c_void,
    pub(super) children: *mut *mut ArrowArray,
    pub(super) dictionary: *mut ArrowArray,
    pub(super) release: ::std::option::Option<unsafe extern "C" fn(arg1: *mut ArrowArray)>,
    pub(super) private_data: *mut ::std::os::raw::c_void,
}

/// ABI-compatible struct for [`ArrowArrayStream`](https://arrow.apache.org/docs/format/CStreamInterface.html).
#[repr(C)]
#[derive(Debug)]
pub struct ArrowArrayStream {
    pub(super) get_schema: ::std::option::Option<
        unsafe extern "C" fn(
            arg1: *mut ArrowArrayStream,
            out: *mut ArrowSchema,
        ) -> ::std::os::raw::c_int,
    >,
    pub(super) get_next: ::std::option::Option<
        unsafe extern "C" fn(
            arg1: *mut ArrowArrayStream,
            out: *mut ArrowArray,
        ) -> ::std::os::raw::c_int,
    >,
    pub(super) get_last_error: ::std::option::Option<
        unsafe extern "C" fn(arg1: *mut ArrowArrayStream) -> *const ::std::os::raw::c_char,
    >,
    pub(super) release: ::std::option::Option<unsafe extern "C" fn(arg1: *mut ArrowArrayStream)>,
    pub(super) private_data: *mut ::std::os::raw::c_void,
}
