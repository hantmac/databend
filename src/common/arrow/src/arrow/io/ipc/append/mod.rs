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

//! A struct adapter of Read+Seek+Write to append to IPC files
// read header and convert to writer information
// seek to first byte of header - 1
// write new batch
// write new footer
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

use super::endianness::is_native_little_endian;
use super::read::FileMetadata;
use super::read::{self};
use super::write::common::DictionaryTracker;
use super::write::writer::*;
use super::write::*;
use crate::arrow::error::Error;
use crate::arrow::error::Result;

impl<R: Read + Seek + Write> FileWriter<R> {
    /// Creates a new [`FileWriter`] from an existing file, seeking to the last message
    /// and appending new messages afterwards. Users call `finish` to write the footer (with both)
    /// the existing and appended messages on it.
    /// # Error
    /// This function errors iff:
    /// * the file's endianness is not the native endianness (not yet supported)
    /// * the file is not a valid Arrow IPC file
    pub fn try_from_file(
        mut writer: R,
        metadata: FileMetadata,
        options: WriteOptions,
    ) -> Result<FileWriter<R>> {
        if metadata.ipc_schema.is_little_endian != is_native_little_endian() {
            return Err(Error::nyi(
                "Appending to a file of a non-native endianness is still not supported",
            ));
        }

        let dictionaries =
            read::read_file_dictionaries(&mut writer, &metadata, &mut Default::default())?;

        let last_block = metadata.blocks.last().ok_or_else(|| {
            Error::oos("An Arrow IPC file must have at least 1 message (the schema message)")
        })?;
        let offset: u64 = last_block
            .offset
            .try_into()
            .map_err(|_| Error::oos("The block's offset must be a positive number"))?;
        let meta_data_length: u64 = last_block
            .meta_data_length
            .try_into()
            .map_err(|_| Error::oos("The block's meta length must be a positive number"))?;
        let body_length: u64 = last_block
            .body_length
            .try_into()
            .map_err(|_| Error::oos("The block's body length must be a positive number"))?;
        let offset: u64 = offset + meta_data_length + body_length;

        writer.seek(SeekFrom::Start(offset))?;

        Ok(FileWriter {
            writer,
            options,
            schema: metadata.schema,
            ipc_fields: metadata.ipc_schema.fields,
            block_offsets: offset as usize,
            dictionary_blocks: metadata.dictionaries.unwrap_or_default(),
            record_blocks: metadata.blocks,
            state: State::Started, // file already exists, so we are ready
            dictionary_tracker: DictionaryTracker {
                dictionaries,
                cannot_replace: true,
            },
            encoded_message: Default::default(),
        })
    }
}
