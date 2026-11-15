use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::sqlserver::model_for_sqlserver::{Column, Schema, SqlType, Table};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub schema: String,
    pub table: String,
    pub column: Option<String>, // テーブル全体のエラーなら None
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    #[error("識別子 '{name}' が空です")]
    EmptyIdentifier { location: Location, name: String },

    #[error("識別子 '{name}' が128文字を超えています({length}文字)")]
    IdentifierTooLong {
        location: Location,
        name: String,
        length: usize,
    },

    #[error("テーブル '{}.{}' が重複しています", .location.schema, .location.table)]
    DuplicateTableName { location: Location, name: String },

    #[error("カラム '{name}' がテーブル '{}.{}' で重複しています", .location.schema, .location.table)]
    DuplicateColumnName { location: Location, name: String },

    #[error("テーブル '{}.{}' にカラムが1つもありません", .location.schema, .location.table)]
    NoColumns { location: Location },

    #[error("NVARCHAR の長さが不正です: {:?}", .size)]
    InvalidNVarCharLength {
        location: Location,
        size: Option<u32>,
    },

    #[error("DECIMAL の精度/小数桁が不正です: precision={}, scale={}", .precision, .scale)]
    InvalidDecimalPrecision {
        location: Location,
        precision: u8,
        scale: u8,
    },

    #[error("テーブル '{}.{}' に IDENTITY カラムが複数あります", .location.schema, .location.table)]
    MultipleIdentities { location: Location },

    #[error("IDENTITY カラム '{}.{}.{}' の型は INT または BIGINT である必要があります", .location.schema, .location.table, .column)]
    IdentityColumnTypeMustBeInteger { location: Location, column: String },

    #[error("主キー列 '{}.{}.{}' が存在しません", .location.schema, .location.table, .column)]
    PrimaryKeyColumnMissing { location: Location, column: String },

    #[error("主キー列 '{}.{}.{}' は NULL を許可できません", .location.schema, .location.table, .column)]
    PrimaryKeyColumnNullable { location: Location, column: String },

    #[error("インデックス '{}.{}' の列 '{}' が見つかりません", .location.schema, .location.table, .column)]
    IndexColumnMissing {
        location: Location,
        index_name: String,
        column: String,
    },

    #[error("インデックス '{}.{}' の列が空です", .location.schema, .location.table)]
    EmptyIndexColumn {
        location: Location,
        index_name: String,
    },

    #[error("インデックス '{}.{}' の名前が重複しています", .location.schema, .location.table)]
    DuplicateIndexName {
        location: Location,
        index_name: String,
    },

    #[error("外部キー '{}.{}' の列 '{}' が見つかりません ({side})", .location.schema, .location.table, .column)]
    ForeignKeyColumnMissing {
        location: Location,
        fk_name: String,
        column: String,
        side: &'static str,
    },

    #[error("外部キー '{}.{}' の列数が一致しません: source={}, ref={}", .location.schema, .location.table, .source_count, .ref_count)]
    ForeignKeyColumnCountMismatch {
        location: Location,
        fk_name: String,
        source_count: usize,
        ref_count: usize,
    },

    #[error("外部キー '{}.{}' の列 '{}' の型が一致しません: {} != {}", .location.schema, .location.table, .column, .source_type, .target_type)]
    ForeignKeyColumnTypeMismatch {
        location: Location,
        fk_name: String,
        column: String,
        source_type: String,
        target_type: String,
    },

    #[error("外部キー '{}.{}' は参照先 '{}.{}' の PK または UNIQUE インデックスに一致しません", .location.schema, .location.table, .ref_schema, .ref_table)]
    ForeignKeyTargetNotUniqueOrPrimaryKey {
        location: Location,
        fk_name: String,
        ref_schema: String,
        ref_table: String,
    },

    #[error("外部キー名 '{}.{}' が重複しています", .location.schema, .location.table)]
    DuplicateForeignKeyName { location: Location, fk_name: String },
}

fn is_blank(value: &str) -> bool {
    value.trim().is_empty()
}

fn identifier_key(name: &str) -> String {
    name.to_lowercase()
}

fn table_key(schema: &str, table: &str) -> String {
    format!("{}:{}", schema.to_lowercase(), table.to_lowercase())
}

fn type_name(ty: &SqlType) -> String {
    match ty {
        SqlType::Int => "INT".to_string(),
        SqlType::BigInt => "BIGINT".to_string(),
        SqlType::Bit => "BIT".to_string(),
        SqlType::DateTime2 => "DATETIME2".to_string(),
        SqlType::UniqueIdentifier => "UNIQUEIDENTIFIER".to_string(),
        SqlType::Decimal(precision, scale) => format!("DECIMAL({}, {})", precision, scale),
        SqlType::NVarChar(size) => match size {
            Some(length) => format!("NVARCHAR({})", length),
            None => "NVARCHAR(MAX)".to_string(),
        },
    }
}

fn validate_identifier_name(location: &Location, name: &str) -> Option<ValidationError> {
    if is_blank(name) {
        return Some(ValidationError::EmptyIdentifier {
            location: location.clone(),
            name: name.to_string(),
        });
    }

    let length = name.chars().count();
    if length > 128 {
        return Some(ValidationError::IdentifierTooLong {
            location: location.clone(),
            name: name.to_string(),
            length,
        });
    }

    None
}

fn validate_type(location: &Location, ty: &SqlType) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    match ty {
        SqlType::NVarChar(size) => {
            if let Some(length) = size
                && (*length == 0 || *length > 4000)
            {
                errors.push(ValidationError::InvalidNVarCharLength {
                    location: location.clone(),
                    size: Some(*length),
                });
            }
        }
        SqlType::Decimal(precision, scale)
            if *precision == 0 || *precision > 38 || *scale > *precision =>
        {
            errors.push(ValidationError::InvalidDecimalPrecision {
                location: location.clone(),
                precision: *precision,
                scale: *scale,
            });
        }
        _ => {}
    }

    errors
}

fn find_column<'a>(table: &'a Table, name: &str) -> Option<&'a Column> {
    table
        .columns
        .iter()
        .find(|column| identifier_key(&column.name) == identifier_key(name))
}

fn validate_table(table: &Table) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let table_location = Location {
        schema: table.schema.clone(),
        table: table.name.clone(),
        column: None,
    };

    if let Some(error) = validate_identifier_name(&table_location, &table.schema) {
        errors.push(error);
    }
    if let Some(error) = validate_identifier_name(&table_location, &table.name) {
        errors.push(error);
    }

    if table.columns.is_empty() {
        errors.push(ValidationError::NoColumns {
            location: table_location.clone(),
        });
    }

    let mut seen_columns = HashSet::new();
    for column in &table.columns {
        let column_location = Location {
            schema: table.schema.clone(),
            table: table.name.clone(),
            column: Some(column.name.clone()),
        };

        if let Some(error) = validate_identifier_name(&column_location, &column.name) {
            errors.push(error);
        }

        let normalized = identifier_key(&column.name);
        if !seen_columns.insert(normalized.clone()) {
            errors.push(ValidationError::DuplicateColumnName {
                location: column_location.clone(),
                name: column.name.clone(),
            });
        }

        errors.extend(validate_type(&column_location, &column.ty));
    }

    let identity_columns: Vec<&Column> = table.columns.iter().filter(|col| col.identity).collect();
    if identity_columns.len() > 1 {
        errors.push(ValidationError::MultipleIdentities {
            location: table_location.clone(),
        });
    }
    for column in identity_columns {
        if !matches!(column.ty, SqlType::Int | SqlType::BigInt) {
            errors.push(ValidationError::IdentityColumnTypeMustBeInteger {
                location: Location {
                    schema: table.schema.clone(),
                    table: table.name.clone(),
                    column: Some(column.name.clone()),
                },
                column: column.name.clone(),
            });
        }
    }

    if !table.primary_key.is_empty() {
        for pk_name in &table.primary_key {
            let pk_location = Location {
                schema: table.schema.clone(),
                table: table.name.clone(),
                column: Some(pk_name.clone()),
            };

            if let Some(column) = find_column(table, pk_name) {
                if column.nullable {
                    errors.push(ValidationError::PrimaryKeyColumnNullable {
                        location: pk_location,
                        column: pk_name.clone(),
                    });
                }
            } else {
                errors.push(ValidationError::PrimaryKeyColumnMissing {
                    location: pk_location,
                    column: pk_name.clone(),
                });
            }
        }
    }

    let mut seen_index_names = HashSet::new();
    for index in &table.indexes {
        let index_location = Location {
            schema: table.schema.clone(),
            table: table.name.clone(),
            column: None,
        };

        if let Some(error) = validate_identifier_name(
            &Location {
                schema: table.schema.clone(),
                table: table.name.clone(),
                column: Some(index.name.clone()),
            },
            &index.name,
        ) {
            errors.push(error);
        }

        let name_key = identifier_key(&index.name);
        if !seen_index_names.insert(name_key.clone()) {
            errors.push(ValidationError::DuplicateIndexName {
                location: index_location.clone(),
                index_name: index.name.clone(),
            });
        }

        if index.columns.is_empty() {
            errors.push(ValidationError::EmptyIndexColumn {
                location: index_location,
                index_name: index.name.clone(),
            });
            continue;
        }

        for column_name in &index.columns {
            if column_name.trim().is_empty() {
                errors.push(ValidationError::EmptyIdentifier {
                    location: Location {
                        schema: table.schema.clone(),
                        table: table.name.clone(),
                        column: Some(column_name.clone()),
                    },
                    name: column_name.clone(),
                });
                continue;
            }

            if !table
                .columns
                .iter()
                .any(|col| identifier_key(&col.name) == identifier_key(column_name))
            {
                errors.push(ValidationError::IndexColumnMissing {
                    location: Location {
                        schema: table.schema.clone(),
                        table: table.name.clone(),
                        column: Some(column_name.clone()),
                    },
                    index_name: index.name.clone(),
                    column: column_name.clone(),
                });
            }
        }
    }

    errors
}

fn is_unique_index_for_columns(table: &Table, columns: &[String]) -> bool {
    let normalized: Vec<String> = columns.iter().map(|name| identifier_key(name)).collect();
    table.indexes.iter().any(|index| {
        index.unique
            && index.columns.len() == normalized.len()
            && index
                .columns
                .iter()
                .map(|name| identifier_key(name))
                .collect::<Vec<String>>()
                == normalized
    })
}

pub fn validate(schema: &Schema) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let mut seen_tables = HashSet::new();
    let mut tables_by_key: HashMap<String, &Table> = HashMap::new();

    for table in &schema.tables {
        let table_location = Location {
            schema: table.schema.clone(),
            table: table.name.clone(),
            column: None,
        };

        let key = table_key(&table.schema, &table.name);
        if !seen_tables.insert(key.clone()) {
            errors.push(ValidationError::DuplicateTableName {
                location: table_location.clone(),
                name: format!("{}.{}", table.schema, table.name),
            });
        }
        tables_by_key.entry(key).or_insert(table);

        errors.extend(validate_table(table));
    }

    let mut seen_fk_names = HashSet::new();
    for table in &schema.tables {
        for fk in &table.foreign_keys {
            let fk_location = Location {
                schema: table.schema.clone(),
                table: table.name.clone(),
                column: None,
            };

            let fk_name_location = Location {
                schema: table.schema.clone(),
                table: table.name.clone(),
                column: Some(fk.name.clone()),
            };
            if let Some(error) = validate_identifier_name(&fk_name_location, &fk.name) {
                errors.push(error);
            }

            let fk_name_key = identifier_key(&fk.name);
            if !seen_fk_names.insert(fk_name_key.clone()) {
                errors.push(ValidationError::DuplicateForeignKeyName {
                    location: fk_location.clone(),
                    fk_name: fk.name.clone(),
                });
            }

            if fk.columns.len() != fk.ref_columns.len() {
                errors.push(ValidationError::ForeignKeyColumnCountMismatch {
                    location: fk_location.clone(),
                    fk_name: fk.name.clone(),
                    source_count: fk.columns.len(),
                    ref_count: fk.ref_columns.len(),
                });
            }

            let ref_table_key = table_key(&fk.ref_schema, &fk.ref_table);
            let ref_table = tables_by_key.get(&ref_table_key);

            for (index, source_column_name) in fk.columns.iter().enumerate() {
                let source_column = find_column(table, source_column_name);
                if source_column.is_none() {
                    errors.push(ValidationError::ForeignKeyColumnMissing {
                        location: Location {
                            schema: table.schema.clone(),
                            table: table.name.clone(),
                            column: Some(source_column_name.clone()),
                        },
                        fk_name: fk.name.clone(),
                        column: source_column_name.clone(),
                        side: "source",
                    });
                    continue;
                }

                if let Some(ref_table_data) = ref_table
                    && index < fk.ref_columns.len()
                {
                    let ref_column_name = &fk.ref_columns[index];
                    if let Some(ref_column) = find_column(ref_table_data, ref_column_name) {
                        if source_column.unwrap().ty != ref_column.ty {
                            errors.push(ValidationError::ForeignKeyColumnTypeMismatch {
                                location: fk_location.clone(),
                                fk_name: fk.name.clone(),
                                column: source_column_name.clone(),
                                source_type: type_name(&source_column.unwrap().ty),
                                target_type: type_name(&ref_column.ty),
                            });
                        }
                    } else {
                        errors.push(ValidationError::ForeignKeyColumnMissing {
                            location: Location {
                                schema: fk.ref_schema.clone(),
                                table: fk.ref_table.clone(),
                                column: Some(ref_column_name.clone()),
                            },
                            fk_name: fk.name.clone(),
                            column: ref_column_name.clone(),
                            side: "reference",
                        });
                    }
                }
            }

            if let Some(ref_table_data) = ref_table {
                let ref_columns_normalized: Vec<String> = fk
                    .ref_columns
                    .iter()
                    .map(|name| identifier_key(name))
                    .collect();
                let pk_match = ref_table_data
                    .primary_key
                    .iter()
                    .map(|name| identifier_key(name))
                    .collect::<Vec<String>>()
                    == ref_columns_normalized;
                let unique_match = is_unique_index_for_columns(ref_table_data, &fk.ref_columns);
                if !pk_match && !unique_match {
                    errors.push(ValidationError::ForeignKeyTargetNotUniqueOrPrimaryKey {
                        location: fk_location.clone(),
                        fk_name: fk.name.clone(),
                        ref_schema: fk.ref_schema.clone(),
                        ref_table: fk.ref_table.clone(),
                    });
                }
            }
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlserver::model_for_sqlserver::{Column, ForeignKey, Index, SqlType, Table};

    fn valid_schema() -> Schema {
        Schema {
            tables: vec![
                Table {
                    schema: "dbo".to_string(),
                    name: "Users".to_string(),
                    columns: vec![
                        Column {
                            name: "Id".to_string(),
                            ty: SqlType::Int,
                            nullable: false,
                            identity: true,
                            default: None,
                            description: "".to_string(),
                        },
                        Column {
                            name: "Name".to_string(),
                            ty: SqlType::NVarChar(Some(100)),
                            nullable: false,
                            identity: false,
                            default: None,
                            description: "".to_string(),
                        },
                    ],
                    primary_key: vec!["Id".to_string()],
                    foreign_keys: vec![],
                    indexes: vec![],
                    description: "".to_string(),
                },
                Table {
                    schema: "dbo".to_string(),
                    name: "Orders".to_string(),
                    columns: vec![
                        Column {
                            name: "Id".to_string(),
                            ty: SqlType::BigInt,
                            nullable: false,
                            identity: true,
                            default: None,
                            description: "".to_string(),
                        },
                        Column {
                            name: "UserId".to_string(),
                            ty: SqlType::Int,
                            nullable: false,
                            identity: false,
                            default: None,
                            description: "".to_string(),
                        },
                    ],
                    primary_key: vec!["Id".to_string()],
                    foreign_keys: vec![ForeignKey {
                        name: "FK_Orders_Users".to_string(),
                        columns: vec!["UserId".to_string()],
                        ref_schema: "dbo".to_string(),
                        ref_table: "Users".to_string(),
                        ref_columns: vec!["Id".to_string()],
                    }],
                    indexes: vec![Index {
                        name: "IX_Orders_UserId".to_string(),
                        columns: vec!["UserId".to_string()],
                        unique: false,
                    }],
                    description: "".to_string(),
                },
            ],
        }
    }

    #[test]
    fn validate_accepts_valid_schema() {
        let schema = valid_schema();
        assert!(validate(&schema).is_empty());
    }

    #[test]
    fn validate_collects_basic_errors() {
        let schema = Schema {
            tables: vec![
                Table {
                    schema: "dbo".to_string(),
                    name: "Users".to_string(),
                    columns: vec![
                        Column {
                            name: "".to_string(),
                            ty: SqlType::Int,
                            nullable: false,
                            identity: false,
                            default: None,
                            description: "".to_string(),
                        },
                        Column {
                            name: "Id".to_string(),
                            ty: SqlType::Int,
                            nullable: false,
                            identity: false,
                            default: None,
                            description: "".to_string(),
                        },
                        Column {
                            name: "Id".to_string(),
                            ty: SqlType::Int,
                            nullable: false,
                            identity: false,
                            default: None,
                            description: "".to_string(),
                        },
                    ],
                    primary_key: vec!["Id".to_string()],
                    foreign_keys: vec![],
                    indexes: vec![],
                    description: "".to_string(),
                },
                Table {
                    schema: "dbo".to_string(),
                    name: "Users".to_string(),
                    columns: vec![],
                    primary_key: vec![],
                    foreign_keys: vec![],
                    indexes: vec![],
                    description: "".to_string(),
                },
            ],
        };

        let errors = validate(&schema);
        assert!(
            errors
                .iter()
                .any(|error| matches!(error, ValidationError::EmptyIdentifier { .. }))
        );
        assert!(
            errors
                .iter()
                .any(|error| matches!(error, ValidationError::DuplicateTableName { .. }))
        );
        assert!(
            errors
                .iter()
                .any(|error| matches!(error, ValidationError::DuplicateColumnName { .. }))
        );
        assert!(
            errors
                .iter()
                .any(|error| matches!(error, ValidationError::NoColumns { .. }))
        );
    }

    #[test]
    fn validate_reports_type_and_relationship_errors() {
        let schema = Schema {
            tables: vec![
                Table {
                    schema: "dbo".to_string(),
                    name: "Users".to_string(),
                    columns: vec![
                        Column {
                            name: "Id".to_string(),
                            ty: SqlType::Int,
                            nullable: false,
                            identity: true,
                            default: None,
                            description: "".to_string(),
                        },
                        Column {
                            name: "Code".to_string(),
                            ty: SqlType::Decimal(40, 2),
                            nullable: false,
                            identity: false,
                            default: None,
                            description: "".to_string(),
                        },
                        Column {
                            name: "Name".to_string(),
                            ty: SqlType::NVarChar(Some(0)),
                            nullable: false,
                            identity: false,
                            default: None,
                            description: "".to_string(),
                        },
                    ],
                    primary_key: vec!["Id".to_string(), "Missing".to_string()],
                    foreign_keys: vec![],
                    indexes: vec![Index {
                        name: "IX_Users_Code".to_string(),
                        columns: vec!["Missing".to_string()],
                        unique: false,
                    }],
                    description: "".to_string(),
                },
                Table {
                    schema: "dbo".to_string(),
                    name: "Orders".to_string(),
                    columns: vec![
                        Column {
                            name: "Id".to_string(),
                            ty: SqlType::Int,
                            nullable: false,
                            identity: false,
                            default: None,
                            description: "".to_string(),
                        },
                        Column {
                            name: "UserId".to_string(),
                            ty: SqlType::BigInt,
                            nullable: false,
                            identity: false,
                            default: None,
                            description: "".to_string(),
                        },
                    ],
                    primary_key: vec![],
                    foreign_keys: vec![ForeignKey {
                        name: "FK_Orders_Users".to_string(),
                        columns: vec!["UserId".to_string()],
                        ref_schema: "dbo".to_string(),
                        ref_table: "Users".to_string(),
                        ref_columns: vec!["Code".to_string()],
                    }],
                    indexes: vec![
                        Index {
                            name: "IX_Orders_UserId".to_string(),
                            columns: vec!["UserId".to_string()],
                            unique: false,
                        },
                        Index {
                            name: "IX_Orders_UserId".to_string(),
                            columns: vec!["Id".to_string()],
                            unique: true,
                        },
                    ],
                    description: "".to_string(),
                },
            ],
        };

        let errors = validate(&schema);
        assert!(
            errors
                .iter()
                .any(|error| matches!(error, ValidationError::InvalidNVarCharLength { .. }))
        );
        assert!(
            errors
                .iter()
                .any(|error| matches!(error, ValidationError::InvalidDecimalPrecision { .. }))
        );
        assert!(
            errors
                .iter()
                .any(|error| matches!(error, ValidationError::PrimaryKeyColumnMissing { .. }))
        );
        assert!(
            errors
                .iter()
                .any(|error| matches!(error, ValidationError::IndexColumnMissing { .. }))
        );
        assert!(
            errors
                .iter()
                .any(|error| matches!(error, ValidationError::DuplicateIndexName { .. }))
        );
        assert!(
            errors
                .iter()
                .any(|error| matches!(error, ValidationError::ForeignKeyColumnTypeMismatch { .. }))
        );
        assert!(errors.iter().any(|error| matches!(
            error,
            ValidationError::ForeignKeyTargetNotUniqueOrPrimaryKey { .. }
        )));
    }
}
