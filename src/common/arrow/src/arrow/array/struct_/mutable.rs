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

use std::sync::Arc;

use super::StructArray;
use crate::arrow::array::Array;
use crate::arrow::array::MutableArray;
use crate::arrow::bitmap::MutableBitmap;
use crate::arrow::datatypes::DataType;
use crate::arrow::error::Error;

/// Converting a [`MutableStructArray`] into a [`StructArray`] is `O(1)`.
#[derive(Debug)]
pub struct MutableStructArray {
    data_type: DataType,
    values: Vec<Box<dyn MutableArray>>,
    validity: Option<MutableBitmap>,
}

fn check(
    data_type: &DataType,
    values: &[Box<dyn MutableArray>],
    validity: Option<usize>,
) -> Result<(), Error> {
    let fields = StructArray::try_get_fields(data_type)?;
    if fields.is_empty() {
        return Err(Error::oos("A StructArray must contain at least one field"));
    }
    if fields.len() != values.len() {
        return Err(Error::oos(
            "A StructArray must have a number of fields in its DataType equal to the number of child values",
        ));
    }

    fields
            .iter().map(|a| &a.data_type)
            .zip(values.iter().map(|a| a.data_type()))
            .enumerate()
            .try_for_each(|(index, (data_type, child))| {
                if data_type != child {
                    Err(Error::oos(format!(
                        "The children DataTypes of a StructArray must equal the children data types. 
                         However, the field {index} has data type {data_type:?} but the value has data type {child:?}"
                    )))
                } else {
                    Ok(())
                }
            })?;

    let len = values[0].len();
    values
            .iter()
            .map(|a| a.len())
            .enumerate()
            .try_for_each(|(index, a_len)| {
                if a_len != len {
                    Err(Error::oos(format!(
                        "The children must have an equal number of values.
                         However, the values at index {index} have a length of {a_len}, which is different from values at index 0, {len}."
                    )))
                } else {
                    Ok(())
                }
            })?;

    if validity.map_or(false, |validity| validity != len) {
        return Err(Error::oos(
            "The validity length of a StructArray must match its number of elements",
        ));
    }
    Ok(())
}

impl From<MutableStructArray> for StructArray {
    fn from(other: MutableStructArray) -> Self {
        let validity = if other.validity.as_ref().map(|x| x.unset_bits()).unwrap_or(0) > 0 {
            other.validity.map(|x| x.into())
        } else {
            None
        };

        StructArray::new(
            other.data_type,
            other.values.into_iter().map(|mut v| v.as_box()).collect(),
            validity,
        )
    }
}

impl MutableStructArray {
    /// Creates a new [`MutableStructArray`].
    pub fn new(data_type: DataType, values: Vec<Box<dyn MutableArray>>) -> Self {
        Self::try_new(data_type, values, None).unwrap()
    }

    /// Create a [`MutableStructArray`] out of low-end APIs.
    /// # Errors
    /// This function errors iff:
    /// * `data_type` is not [`DataType::Struct`]
    /// * The inner types of `data_type` are not equal to those of `values`
    /// * `validity` is not `None` and its length is different from the `values`'s length
    pub fn try_new(
        data_type: DataType,
        values: Vec<Box<dyn MutableArray>>,
        validity: Option<MutableBitmap>,
    ) -> Result<Self, Error> {
        check(&data_type, &values, validity.as_ref().map(|x| x.len()))?;
        Ok(Self {
            data_type,
            values,
            validity,
        })
    }

    /// Extract the low-end APIs from the [`MutableStructArray`].
    pub fn into_inner(self) -> (DataType, Vec<Box<dyn MutableArray>>, Option<MutableBitmap>) {
        (self.data_type, self.values, self.validity)
    }

    /// The mutable values
    pub fn mut_values(&mut self) -> &mut Vec<Box<dyn MutableArray>> {
        &mut self.values
    }

    /// The values
    pub fn values(&self) -> &Vec<Box<dyn MutableArray>> {
        &self.values
    }

    /// Return the `i`th child array.
    pub fn value<A: MutableArray + 'static>(&mut self, i: usize) -> Option<&mut A> {
        self.values[i].as_mut_any().downcast_mut::<A>()
    }
}

impl MutableStructArray {
    /// Reserves `additional` entries.
    pub fn reserve(&mut self, additional: usize) {
        for v in &mut self.values {
            v.reserve(additional);
        }
        if let Some(x) = self.validity.as_mut() {
            x.reserve(additional)
        }
    }

    /// Call this once for each "row" of children you push.
    pub fn push(&mut self, valid: bool) {
        match &mut self.validity {
            Some(validity) => validity.push(valid),
            None => match valid {
                true => (),
                false => self.init_validity(),
            },
        };
    }

    fn push_null(&mut self) {
        for v in &mut self.values {
            v.push_null();
        }
        self.push(false);
    }

    fn init_validity(&mut self) {
        let mut validity = MutableBitmap::with_capacity(self.values.capacity());
        let len = self.len();
        if len > 0 {
            validity.extend_constant(len, true);
            validity.set(len - 1, false);
        }
        self.validity = Some(validity)
    }

    /// Converts itself into an [`Array`].
    pub fn into_arc(self) -> Arc<dyn Array> {
        let a: StructArray = self.into();
        Arc::new(a)
    }

    /// Shrinks the capacity of the [`MutableStructArray`] to fit its current length.
    pub fn shrink_to_fit(&mut self) {
        for v in &mut self.values {
            v.shrink_to_fit();
        }
        if let Some(validity) = self.validity.as_mut() {
            validity.shrink_to_fit()
        }
    }
}

impl MutableArray for MutableStructArray {
    fn len(&self) -> usize {
        self.values.first().map(|v| v.len()).unwrap_or(0)
    }

    fn validity(&self) -> Option<&MutableBitmap> {
        self.validity.as_ref()
    }

    fn as_box(&mut self) -> Box<dyn Array> {
        StructArray::new(
            self.data_type.clone(),
            std::mem::take(&mut self.values)
                .into_iter()
                .map(|mut v| v.as_box())
                .collect(),
            std::mem::take(&mut self.validity).map(|x| x.into()),
        )
        .boxed()
    }

    fn as_arc(&mut self) -> Arc<dyn Array> {
        StructArray::new(
            self.data_type.clone(),
            std::mem::take(&mut self.values)
                .into_iter()
                .map(|mut v| v.as_box())
                .collect(),
            std::mem::take(&mut self.validity).map(|x| x.into()),
        )
        .arced()
    }

    fn data_type(&self) -> &DataType {
        &self.data_type
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn push_null(&mut self) {
        self.push_null()
    }

    fn shrink_to_fit(&mut self) {
        self.shrink_to_fit()
    }

    fn reserve(&mut self, additional: usize) {
        self.reserve(additional)
    }
}
