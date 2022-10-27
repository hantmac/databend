// Copyright 2022 Datafuse Labs.
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

use std::fmt::Display;
use std::fmt::Formatter;

use common_datavalues::format_data_type_sql;
use common_datavalues::format_datavalue_sql;
use common_datavalues::DataField;
use common_datavalues::DataType;
use common_datavalues::DataTypeImpl;
use common_datavalues::DataValue;

type ColumnID = String;
type IndexType = usize;

/// Serializable and desugared representation of `Scalar`.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PhysicalScalar {
    IndexedVariable {
        index: usize,
        data_type: DataTypeImpl,

        display_name: String,
    },
    Constant {
        value: DataValue,
        data_type: DataTypeImpl,
    },
    Function {
        name: String,
        args: Vec<PhysicalScalar>,
        return_type: DataTypeImpl,
    },

    Cast {
        input: Box<PhysicalScalar>,
        target: DataTypeImpl,
    },
}

impl PhysicalScalar {
    pub fn column_name(&self) -> String {
        match self {
            PhysicalScalar::IndexedVariable { display_name, .. } => display_name.clone(),
            PhysicalScalar::Constant { value, .. } => format_datavalue_sql(value),
            PhysicalScalar::Function { name, args, .. } => {
                let args_column_name = args
                    .iter()
                    .map(PhysicalScalar::column_name)
                    .collect::<Vec<_>>();
                format!("{}({})", name, args_column_name.join(", "))
            }
            PhysicalScalar::Cast { input, target } => {
                format!("{}::{}", input.column_name(), target.sql_name())
            }
        }
    }

    pub fn data_type(&self) -> DataTypeImpl {
        match self {
            PhysicalScalar::Constant { data_type, .. } => data_type.clone(),
            PhysicalScalar::Function { return_type, .. } => return_type.clone(),
            PhysicalScalar::Cast { target, .. } => target.clone(),
            PhysicalScalar::IndexedVariable { data_type, .. } => data_type.clone(),
        }
    }

    pub fn to_data_field(&self) -> DataField {
        let name = self.column_name();
        let data_type = self.data_type();
        DataField::new(&name, data_type)
    }

    /// Display with readable variable name.
    pub fn pretty_display(&self) -> String {
        match self {
            PhysicalScalar::Constant { value, .. } => value.to_string(),
            PhysicalScalar::Function { name, args, .. } => {
                let args = args
                    .iter()
                    .map(|arg| arg.pretty_display())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", name, args)
            }
            PhysicalScalar::Cast { input, target } => format!(
                "CAST({} AS {})",
                input.pretty_display(),
                format_data_type_sql(target)
            ),
            PhysicalScalar::IndexedVariable { display_name, .. } => display_name.clone(),
        }
    }
}

impl Display for PhysicalScalar {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            PhysicalScalar::Constant { value, .. } => write!(f, "{}", value),
            PhysicalScalar::Function { name, args, .. } => write!(
                f,
                "{}({})",
                name,
                args.iter()
                    .map(|arg| format!("{}", arg))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            PhysicalScalar::Cast { input, target } => {
                write!(f, "CAST({} AS {})", input, format_data_type_sql(target))
            }
            PhysicalScalar::IndexedVariable { index, .. } => write!(f, "${index}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AggregateFunctionDesc {
    pub sig: AggregateFunctionSignature,
    pub column_id: ColumnID,
    pub args: Vec<usize>,

    /// Only used for debugging
    pub arg_indices: Vec<IndexType>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AggregateFunctionSignature {
    pub name: String,
    pub args: Vec<DataTypeImpl>,
    pub params: Vec<DataValue>,
    pub return_type: DataTypeImpl,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SortDesc {
    pub asc: bool,
    pub nulls_first: bool,
    pub order_by: ColumnID,
}
