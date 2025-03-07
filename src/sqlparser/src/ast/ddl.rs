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

//! AST types specific to CREATE/ALTER variants of [`crate::ast::Statement`]
//! (commonly referred to as Data Definition Language, or DDL)

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::ToString, vec::Vec};
use core::fmt;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::ConnectorSchema;
use crate::ast::{
    display_comma_separated, display_separated, DataType, Expr, Ident, ObjectName, SetVariableValue,
};
use crate::tokenizer::Token;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "visitor", derive(Visit, VisitMut))]
pub enum AlterDatabaseOperation {
    ChangeOwner { new_owner_name: Ident },
    RenameDatabase { database_name: ObjectName },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "visitor", derive(Visit, VisitMut))]
pub enum AlterSchemaOperation {
    ChangeOwner { new_owner_name: Ident },
    RenameSchema { schema_name: ObjectName },
}

/// An `ALTER TABLE` (`Statement::AlterTable`) operation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AlterTableOperation {
    /// `ADD <table_constraint>`
    AddConstraint(TableConstraint),
    /// `ADD [ COLUMN ] <column_def>`
    AddColumn { column_def: ColumnDef },
    /// TODO: implement `DROP CONSTRAINT <name>`
    DropConstraint { name: Ident },
    /// `DROP [ COLUMN ] [ IF EXISTS ] <column_name> [ CASCADE ]`
    DropColumn {
        column_name: Ident,
        if_exists: bool,
        cascade: bool,
    },
    /// `RENAME [ COLUMN ] <old_column_name> TO <new_column_name>`
    RenameColumn {
        old_column_name: Ident,
        new_column_name: Ident,
    },
    /// `RENAME TO <table_name>`
    RenameTable { table_name: ObjectName },
    // CHANGE [ COLUMN ] <old_name> <new_name> <data_type> [ <options> ]
    ChangeColumn {
        old_name: Ident,
        new_name: Ident,
        data_type: DataType,
        options: Vec<ColumnOption>,
    },
    /// `RENAME CONSTRAINT <old_constraint_name> TO <new_constraint_name>`
    ///
    /// Note: this is a PostgreSQL-specific operation.
    RenameConstraint { old_name: Ident, new_name: Ident },
    /// `ALTER [ COLUMN ]`
    AlterColumn {
        column_name: Ident,
        op: AlterColumnOperation,
    },
    /// `OWNER TO <owner_name>`
    ChangeOwner { new_owner_name: Ident },
    /// `SET SCHEMA <schema_name>`
    SetSchema { new_schema_name: ObjectName },
    /// `SET PARALLELISM TO <parallelism>`
    SetParallelism { parallelism: SetVariableValue },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "visitor", derive(Visit, VisitMut))]
pub enum AlterIndexOperation {
    RenameIndex {
        index_name: ObjectName,
    },
    /// `SET PARALLELISM TO <parallelism>`
    SetParallelism {
        parallelism: SetVariableValue,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "visitor", derive(Visit, VisitMut))]
pub enum AlterViewOperation {
    RenameView {
        view_name: ObjectName,
    },
    ChangeOwner {
        new_owner_name: Ident,
    },
    SetSchema {
        new_schema_name: ObjectName,
    },
    /// `SET PARALLELISM TO <parallelism>`
    SetParallelism {
        parallelism: SetVariableValue,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "visitor", derive(Visit, VisitMut))]
pub enum AlterSinkOperation {
    RenameSink {
        sink_name: ObjectName,
    },
    ChangeOwner {
        new_owner_name: Ident,
    },
    SetSchema {
        new_schema_name: ObjectName,
    },
    /// `SET PARALLELISM TO <parallelism>`
    SetParallelism {
        parallelism: SetVariableValue,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "visitor", derive(Visit, VisitMut))]
pub enum AlterSourceOperation {
    RenameSource { source_name: ObjectName },
    AddColumn { column_def: ColumnDef },
    ChangeOwner { new_owner_name: Ident },
    SetSchema { new_schema_name: ObjectName },
    FormatEncode { connector_schema: ConnectorSchema },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "visitor", derive(Visit, VisitMut))]
pub enum AlterFunctionOperation {
    SetSchema { new_schema_name: ObjectName },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "visitor", derive(Visit, VisitMut))]
pub enum AlterConnectionOperation {
    SetSchema { new_schema_name: ObjectName },
}

impl fmt::Display for AlterDatabaseOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlterDatabaseOperation::ChangeOwner { new_owner_name } => {
                write!(f, "OWNER TO {}", new_owner_name)
            }
            AlterDatabaseOperation::RenameDatabase { database_name } => {
                write!(f, "RENAME TO {}", database_name)
            }
        }
    }
}

impl fmt::Display for AlterSchemaOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlterSchemaOperation::ChangeOwner { new_owner_name } => {
                write!(f, "OWNER TO {}", new_owner_name)
            }
            AlterSchemaOperation::RenameSchema { schema_name } => {
                write!(f, "RENAME TO {}", schema_name)
            }
        }
    }
}

impl fmt::Display for AlterTableOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlterTableOperation::AddConstraint(c) => write!(f, "ADD {}", c),
            AlterTableOperation::AddColumn { column_def } => {
                write!(f, "ADD COLUMN {}", column_def)
            }
            AlterTableOperation::AlterColumn { column_name, op } => {
                write!(f, "ALTER COLUMN {} {}", column_name, op)
            }
            AlterTableOperation::DropConstraint { name } => write!(f, "DROP CONSTRAINT {}", name),
            AlterTableOperation::DropColumn {
                column_name,
                if_exists,
                cascade,
            } => write!(
                f,
                "DROP COLUMN {}{}{}",
                if *if_exists { "IF EXISTS " } else { "" },
                column_name,
                if *cascade { " CASCADE" } else { "" }
            ),
            AlterTableOperation::RenameColumn {
                old_column_name,
                new_column_name,
            } => write!(
                f,
                "RENAME COLUMN {} TO {}",
                old_column_name, new_column_name
            ),
            AlterTableOperation::RenameTable { table_name } => {
                write!(f, "RENAME TO {}", table_name)
            }
            AlterTableOperation::ChangeColumn {
                old_name,
                new_name,
                data_type,
                options,
            } => {
                write!(f, "CHANGE COLUMN {} {} {}", old_name, new_name, data_type)?;
                if options.is_empty() {
                    Ok(())
                } else {
                    write!(f, " {}", display_separated(options, " "))
                }
            }
            AlterTableOperation::RenameConstraint { old_name, new_name } => {
                write!(f, "RENAME CONSTRAINT {} TO {}", old_name, new_name)
            }
            AlterTableOperation::ChangeOwner { new_owner_name } => {
                write!(f, "OWNER TO {}", new_owner_name)
            }
            AlterTableOperation::SetSchema { new_schema_name } => {
                write!(f, "SET SCHEMA {}", new_schema_name)
            }
            AlterTableOperation::SetParallelism { parallelism } => {
                write!(f, "SET PARALLELISM TO {}", parallelism)
            }
        }
    }
}

impl fmt::Display for AlterIndexOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlterIndexOperation::RenameIndex { index_name } => {
                write!(f, "RENAME TO {index_name}")
            }
            AlterIndexOperation::SetParallelism { parallelism } => {
                write!(f, "SET PARALLELISM TO {}", parallelism)
            }
        }
    }
}

impl fmt::Display for AlterViewOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlterViewOperation::RenameView { view_name } => {
                write!(f, "RENAME TO {view_name}")
            }
            AlterViewOperation::ChangeOwner { new_owner_name } => {
                write!(f, "OWNER TO {}", new_owner_name)
            }
            AlterViewOperation::SetSchema { new_schema_name } => {
                write!(f, "SET SCHEMA {}", new_schema_name)
            }
            AlterViewOperation::SetParallelism { parallelism } => {
                write!(f, "SET PARALLELISM TO {}", parallelism)
            }
        }
    }
}

impl fmt::Display for AlterSinkOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlterSinkOperation::RenameSink { sink_name } => {
                write!(f, "RENAME TO {sink_name}")
            }
            AlterSinkOperation::ChangeOwner { new_owner_name } => {
                write!(f, "OWNER TO {}", new_owner_name)
            }
            AlterSinkOperation::SetSchema { new_schema_name } => {
                write!(f, "SET SCHEMA {}", new_schema_name)
            }
            AlterSinkOperation::SetParallelism { parallelism } => {
                write!(f, "SET PARALLELISM TO {}", parallelism)
            }
        }
    }
}

impl fmt::Display for AlterSourceOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlterSourceOperation::RenameSource { source_name } => {
                write!(f, "RENAME TO {source_name}")
            }
            AlterSourceOperation::AddColumn { column_def } => {
                write!(f, "ADD COLUMN {column_def}")
            }
            AlterSourceOperation::ChangeOwner { new_owner_name } => {
                write!(f, "OWNER TO {}", new_owner_name)
            }
            AlterSourceOperation::SetSchema { new_schema_name } => {
                write!(f, "SET SCHEMA {}", new_schema_name)
            }
            AlterSourceOperation::FormatEncode { connector_schema } => {
                write!(f, "{connector_schema}")
            }
        }
    }
}

impl fmt::Display for AlterFunctionOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlterFunctionOperation::SetSchema { new_schema_name } => {
                write!(f, "SET SCHEMA {new_schema_name}")
            }
        }
    }
}

impl fmt::Display for AlterConnectionOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlterConnectionOperation::SetSchema { new_schema_name } => {
                write!(f, "SET SCHEMA {new_schema_name}")
            }
        }
    }
}

/// An `ALTER COLUMN` (`Statement::AlterTable`) operation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AlterColumnOperation {
    /// `SET NOT NULL`
    SetNotNull,
    /// `DROP NOT NULL`
    DropNotNull,
    /// `SET DEFAULT <expr>`
    SetDefault { value: Expr },
    /// `DROP DEFAULT`
    DropDefault,
    /// `[SET DATA] TYPE <data_type> [USING <expr>]`
    SetDataType {
        data_type: DataType,
        /// PostgreSQL specific
        using: Option<Expr>,
    },
}

impl fmt::Display for AlterColumnOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlterColumnOperation::SetNotNull => write!(f, "SET NOT NULL",),
            AlterColumnOperation::DropNotNull => write!(f, "DROP NOT NULL",),
            AlterColumnOperation::SetDefault { value } => {
                write!(f, "SET DEFAULT {}", value)
            }
            AlterColumnOperation::DropDefault {} => {
                write!(f, "DROP DEFAULT")
            }
            AlterColumnOperation::SetDataType { data_type, using } => {
                if let Some(expr) = using {
                    write!(f, "SET DATA TYPE {} USING {}", data_type, expr)
                } else {
                    write!(f, "SET DATA TYPE {}", data_type)
                }
            }
        }
    }
}

/// The watermark on source.
/// `WATERMARK FOR <column> AS (<expr>)`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SourceWatermark {
    pub column: Ident,
    pub expr: Expr,
}

impl fmt::Display for SourceWatermark {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WATERMARK FOR {} AS {}", self.column, self.expr,)
    }
}

/// A table-level constraint, specified in a `CREATE TABLE` or an
/// `ALTER TABLE ADD <constraint>` statement.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TableConstraint {
    /// `[ CONSTRAINT <name> ] { PRIMARY KEY | UNIQUE } (<columns>)`
    Unique {
        name: Option<Ident>,
        columns: Vec<Ident>,
        /// Whether this is a `PRIMARY KEY` or just a `UNIQUE` constraint
        is_primary: bool,
    },
    /// A referential integrity constraint (`[ CONSTRAINT <name> ] FOREIGN KEY (<columns>)
    /// REFERENCES <foreign_table> (<referred_columns>)
    /// { [ON DELETE <referential_action>] [ON UPDATE <referential_action>] |
    ///   [ON UPDATE <referential_action>] [ON DELETE <referential_action>]
    /// }`).
    ForeignKey {
        name: Option<Ident>,
        columns: Vec<Ident>,
        foreign_table: ObjectName,
        referred_columns: Vec<Ident>,
        on_delete: Option<ReferentialAction>,
        on_update: Option<ReferentialAction>,
    },
    /// `[ CONSTRAINT <name> ] CHECK (<expr>)`
    Check {
        name: Option<Ident>,
        expr: Box<Expr>,
    },
}

impl fmt::Display for TableConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TableConstraint::Unique {
                name,
                columns,
                is_primary,
            } => write!(
                f,
                "{}{} ({})",
                display_constraint_name(name),
                if *is_primary { "PRIMARY KEY" } else { "UNIQUE" },
                display_comma_separated(columns)
            ),
            TableConstraint::ForeignKey {
                name,
                columns,
                foreign_table,
                referred_columns,
                on_delete,
                on_update,
            } => {
                write!(
                    f,
                    "{}FOREIGN KEY ({}) REFERENCES {}({})",
                    display_constraint_name(name),
                    display_comma_separated(columns),
                    foreign_table,
                    display_comma_separated(referred_columns),
                )?;
                if let Some(action) = on_delete {
                    write!(f, " ON DELETE {}", action)?;
                }
                if let Some(action) = on_update {
                    write!(f, " ON UPDATE {}", action)?;
                }
                Ok(())
            }
            TableConstraint::Check { name, expr } => {
                write!(f, "{}CHECK ({})", display_constraint_name(name), expr)
            }
        }
    }
}

/// SQL column definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ColumnDef {
    pub name: Ident,
    pub data_type: Option<DataType>,
    pub collation: Option<ObjectName>,
    pub options: Vec<ColumnOptionDef>,
}

impl ColumnDef {
    pub fn new(
        name: Ident,
        data_type: DataType,
        collation: Option<ObjectName>,
        options: Vec<ColumnOptionDef>,
    ) -> Self {
        ColumnDef {
            name,
            data_type: Some(data_type),
            collation,
            options,
        }
    }

    pub fn is_generated(&self) -> bool {
        self.options
            .iter()
            .any(|option| matches!(option.option, ColumnOption::GeneratedColumns(_)))
    }
}

impl fmt::Display for ColumnDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}",
            self.name,
            if let Some(data_type) = &self.data_type {
                data_type.to_string()
            } else {
                "None".to_string()
            }
        )?;
        for option in &self.options {
            write!(f, " {}", option)?;
        }
        Ok(())
    }
}

/// An optionally-named `ColumnOption`: `[ CONSTRAINT <name> ] <column-option>`.
///
/// Note that implementations are substantially more permissive than the ANSI
/// specification on what order column options can be presented in, and whether
/// they are allowed to be named. The specification distinguishes between
/// constraints (NOT NULL, UNIQUE, PRIMARY KEY, and CHECK), which can be named
/// and can appear in any order, and other options (DEFAULT, GENERATED), which
/// cannot be named and must appear in a fixed order. PostgreSQL, however,
/// allows preceding any option with `CONSTRAINT <name>`, even those that are
/// not really constraints, like NULL and DEFAULT. MSSQL is less permissive,
/// allowing DEFAULT, UNIQUE, PRIMARY KEY and CHECK to be named, but not NULL or
/// NOT NULL constraints (the last of which is in violation of the spec).
///
/// For maximum flexibility, we don't distinguish between constraint and
/// non-constraint options, lumping them all together under the umbrella of
/// "column options," and we allow any column option to be named.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ColumnOptionDef {
    pub name: Option<Ident>,
    pub option: ColumnOption,
}

impl fmt::Display for ColumnOptionDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", display_constraint_name(&self.name), self.option)
    }
}

/// `ColumnOption`s are modifiers that follow a column definition in a `CREATE
/// TABLE` statement.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ColumnOption {
    /// `NULL`
    Null,
    /// `NOT NULL`
    NotNull,
    /// `DEFAULT <restricted-expr>`
    DefaultColumns(Expr),
    /// `{ PRIMARY KEY | UNIQUE }`
    Unique { is_primary: bool },
    /// A referential integrity constraint (`[FOREIGN KEY REFERENCES
    /// <foreign_table> (<referred_columns>)
    /// { [ON DELETE <referential_action>] [ON UPDATE <referential_action>] |
    ///   [ON UPDATE <referential_action>] [ON DELETE <referential_action>]
    /// }`).
    ForeignKey {
        foreign_table: ObjectName,
        referred_columns: Vec<Ident>,
        on_delete: Option<ReferentialAction>,
        on_update: Option<ReferentialAction>,
    },
    /// `CHECK (<expr>)`
    Check(Expr),
    /// Dialect-specific options, such as:
    /// - MySQL's `AUTO_INCREMENT` or SQLite's `AUTOINCREMENT`
    /// - ...
    DialectSpecific(Vec<Token>),
    /// AS ( <generation_expr> )`
    GeneratedColumns(Expr),
}

impl fmt::Display for ColumnOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ColumnOption::*;
        match self {
            Null => write!(f, "NULL"),
            NotNull => write!(f, "NOT NULL"),
            DefaultColumns(expr) => write!(f, "DEFAULT {}", expr),
            Unique { is_primary } => {
                write!(f, "{}", if *is_primary { "PRIMARY KEY" } else { "UNIQUE" })
            }
            ForeignKey {
                foreign_table,
                referred_columns,
                on_delete,
                on_update,
            } => {
                write!(f, "REFERENCES {}", foreign_table)?;
                if !referred_columns.is_empty() {
                    write!(f, " ({})", display_comma_separated(referred_columns))?;
                }
                if let Some(action) = on_delete {
                    write!(f, " ON DELETE {}", action)?;
                }
                if let Some(action) = on_update {
                    write!(f, " ON UPDATE {}", action)?;
                }
                Ok(())
            }
            Check(expr) => write!(f, "CHECK ({})", expr),
            DialectSpecific(val) => write!(f, "{}", display_separated(val, " ")),
            GeneratedColumns(expr) => write!(f, "AS {}", expr),
        }
    }
}

fn display_constraint_name(name: &'_ Option<Ident>) -> impl fmt::Display + '_ {
    struct ConstraintName<'a>(&'a Option<Ident>);
    impl<'a> fmt::Display for ConstraintName<'a> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if let Some(name) = self.0 {
                write!(f, "CONSTRAINT {} ", name)?;
            }
            Ok(())
        }
    }
    ConstraintName(name)
}

/// `<referential_action> =
/// { RESTRICT | CASCADE | SET NULL | NO ACTION | SET DEFAULT }`
///
/// Used in foreign key constraints in `ON UPDATE` and `ON DELETE` options.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ReferentialAction {
    Restrict,
    Cascade,
    SetNull,
    NoAction,
    SetDefault,
}

impl fmt::Display for ReferentialAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ReferentialAction::Restrict => "RESTRICT",
            ReferentialAction::Cascade => "CASCADE",
            ReferentialAction::SetNull => "SET NULL",
            ReferentialAction::NoAction => "NO ACTION",
            ReferentialAction::SetDefault => "SET DEFAULT",
        })
    }
}
