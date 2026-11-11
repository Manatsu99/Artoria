use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SqlType {
    Int,
    BigInt,
    Bit,
    DateTime2,
    UniqueIdentifier,
    Decimal(u8, u8),       // (精度, 小数桁)
    NVarChar(Option<u32>), // None = MAX
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub ty: SqlType,
    pub nullable: bool,
    pub identity: bool,
    pub default: Option<String>, // null = DEFAULTなし
    pub description: String,     // 列の説明
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub schema: String,
    pub name: String,
    pub columns: Vec<Column>,
    pub primary_key: Vec<String>,      // None = 主キーなし
    pub foreign_keys: Vec<ForeignKey>, // None = 外部キーなし
    pub indexes: Vec<Index>,           // None = インデックスなし
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForeignKey {
    pub name: String,
    pub columns: Vec<String>,
    pub ref_schema: String,
    pub ref_table: String,
    pub ref_columns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Index {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Schema {
    pub tables: Vec<Table>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_schema() -> Schema {
        let users = Table {
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
                Column {
                    name: "Email".to_string(),
                    ty: SqlType::NVarChar(Some(255)),
                    nullable: true,
                    identity: false,
                    default: None,
                    description: "".to_string(),
                },
                Column {
                    name: "CreatedAt".to_string(),
                    ty: SqlType::DateTime2,
                    nullable: false,
                    identity: false,
                    default: Some("GETDATE()".to_string()),
                    description: "".to_string(),
                },
            ],
            primary_key: vec!["Id".to_string()],
            foreign_keys: vec![],
            indexes: vec![Index {
                name: "IX_Users_Email".to_string(),
                columns: vec!["Email".to_string()],
                unique: true,
            }],
            description: "".to_string(),
        };

        let orders = Table {
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
            indexes: vec![],
            description: "".to_string(),
        };

        Schema {
            tables: vec![users, orders],
        }
    }

    #[test]
    fn schema_can_be_cloned_and_compared() {
        let schema = sample_schema();
        assert_eq!(schema, schema.clone());

        // 中身を変えたら「等しくない」になることも確認する
        let mut changed = schema.clone();
        changed.tables[0].name = "Other".to_string();
        assert_ne!(schema, changed);
    }
}
