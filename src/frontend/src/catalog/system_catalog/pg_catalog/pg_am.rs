// Copyright 2024 RisingWave Labs
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

use std::sync::LazyLock;

use risingwave_common::catalog::PG_CATALOG_SCHEMA_NAME;
use risingwave_common::types::DataType;

use crate::catalog::system_catalog::{infer_dummy_view_sql, BuiltinView, SystemCatalogColumnsDef};

pub const PG_AM_COLUMNS: &[SystemCatalogColumnsDef<'_>] = &[
    (DataType::Int32, "oid"),
    (DataType::Varchar, "amname"),
    (DataType::Int32, "amhandler"),
    (DataType::Varchar, "amtype"),
];

/// Stores information about relation access methods.
/// Reference: [`https://www.postgresql.org/docs/current/catalog-pg-am.html`]
pub static PG_AM: LazyLock<BuiltinView> = LazyLock::new(|| BuiltinView {
    name: "pg_am",
    schema: PG_CATALOG_SCHEMA_NAME,
    columns: PG_AM_COLUMNS,
    sql: infer_dummy_view_sql(PG_AM_COLUMNS),
});
