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

use parquet2::error::Error as ParquetError;
use parquet2::schema::types::ParquetType;
use parquet2::write::Compressor;
use parquet2::FallibleStreamingIterator;

use super::array_to_columns;
use super::to_parquet_schema;
use super::DynIter;
use super::DynStreamingIterator;
use super::Encoding;
use super::RowGroupIter;
use super::SchemaDescriptor;
use super::WriteOptions;
use crate::arrow::array::Array;
use crate::arrow::chunk::Chunk;
use crate::arrow::datatypes::Schema;
use crate::arrow::error::Error;
use crate::arrow::error::Result;

/// Maps a [`Chunk`] and parquet-specific options to an [`RowGroupIter`] used to
/// write to parquet
/// # Panics
/// Iff
/// * `encodings.len() != fields.len()` or
/// * `encodings.len() != chunk.arrays().len()`
pub fn row_group_iter<A: AsRef<dyn Array> + 'static + Send + Sync>(
    chunk: Chunk<A>,
    encodings: Vec<Vec<Encoding>>,
    fields: Vec<ParquetType>,
    options: WriteOptions,
) -> RowGroupIter<'static, Error> {
    assert_eq!(encodings.len(), fields.len());
    assert_eq!(encodings.len(), chunk.arrays().len());
    DynIter::new(
        chunk
            .into_arrays()
            .into_iter()
            .zip(fields.into_iter())
            .zip(encodings.into_iter())
            .flat_map(move |((array, type_), encoding)| {
                let encoded_columns = array_to_columns(array, type_, options, &encoding).unwrap();
                encoded_columns
                    .into_iter()
                    .map(|encoded_pages| {
                        let pages = encoded_pages;

                        let pages = DynIter::new(
                            pages
                                .into_iter()
                                .map(|x| x.map_err(|e| ParquetError::OutOfSpec(e.to_string()))),
                        );

                        let compressed_pages = Compressor::new(pages, options.compression, vec![])
                            .map_err(Error::from);
                        Ok(DynStreamingIterator::new(compressed_pages))
                    })
                    .collect::<Vec<_>>()
            }),
    )
}

/// An iterator adapter that converts an iterator over [`Chunk`] into an iterator
/// of row groups.
/// Use it to create an iterator consumable by the parquet's API.
pub struct RowGroupIterator<A: AsRef<dyn Array> + 'static, I: Iterator<Item = Result<Chunk<A>>>> {
    iter: I,
    options: WriteOptions,
    parquet_schema: SchemaDescriptor,
    encodings: Vec<Vec<Encoding>>,
}

impl<A: AsRef<dyn Array> + 'static, I: Iterator<Item = Result<Chunk<A>>>> RowGroupIterator<A, I> {
    /// Creates a new [`RowGroupIterator`] from an iterator over [`Chunk`].
    ///
    /// # Errors
    /// Iff
    /// * the Arrow schema can't be converted to a valid Parquet schema.
    /// * the length of the encodings is different from the number of fields in schema
    pub fn try_new(
        iter: I,
        schema: &Schema,
        options: WriteOptions,
        encodings: Vec<Vec<Encoding>>,
    ) -> Result<Self> {
        if encodings.len() != schema.fields.len() {
            return Err(Error::InvalidArgumentError(
                "The number of encodings must equal the number of fields".to_string(),
            ));
        }
        let parquet_schema = to_parquet_schema(schema)?;

        Ok(Self {
            iter,
            options,
            parquet_schema,
            encodings,
        })
    }

    /// Returns the [`SchemaDescriptor`] of the [`RowGroupIterator`].
    pub fn parquet_schema(&self) -> &SchemaDescriptor {
        &self.parquet_schema
    }
}

impl<A: AsRef<dyn Array> + 'static + Send + Sync, I: Iterator<Item = Result<Chunk<A>>>> Iterator
    for RowGroupIterator<A, I>
{
    type Item = Result<RowGroupIter<'static, Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        let options = self.options;

        self.iter.next().map(|maybe_chunk| {
            let chunk = maybe_chunk?;
            if self.encodings.len() != chunk.arrays().len() {
                return Err(Error::InvalidArgumentError(
                    "The number of arrays in the chunk must equal the number of fields in the schema"
                        .to_string(),
                ));
            };
            let encodings = self.encodings.clone();
            Ok(row_group_iter(
                chunk,
                encodings,
                self.parquet_schema.fields().to_vec(),
                options,
            ))
        })
    }
}
