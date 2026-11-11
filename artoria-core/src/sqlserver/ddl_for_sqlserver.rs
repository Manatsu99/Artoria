use crate::sqlserver::model_for_sqlserver::{SqlType, Table, ForeignKey, Index};

fn quote(name: &str) -> String {
    format!("[{}]", name)
}

fn type_sql(ty: &SqlType) -> String {
    match ty {
        SqlType::Int => "INT".to_string(),
        SqlType::BigInt => "BIGINT".to_string(),
        SqlType::Bit => "BIT".to_string(),
        SqlType::DateTime2 => "DATETIME2".to_string(),
        SqlType::UniqueIdentifier => "UNIQUEIDENTIFIER".to_string(),
        SqlType::Decimal(precision, scale) => format!("DECIMAL({}, {})", precision, scale),
        SqlType::NVarChar(size) => match size {
            Some(size) => format!("NVARCHAR({})", size),
            None => "NVARCHAR(MAX)".to_string(),
        },
    }
}   

pub fn create_table_sql(table: &Table) -> String {
    let columns_sql: Vec<String> = table.columns.iter().map(|col| {
        let col_name = quote(&col.name);
        let col_type = type_sql(&col.ty);
        let col_identity = if col.identity { " IDENTITY(1,1)" } else { "" };
        let col_nullable = if !col.nullable { " NOT NULL" } else { " NULL" };
        let col_default = if let Some(default) = &col.default { format!(" DEFAULT {}", default) } else { "".to_string() };
        format!("{} {}{}{}{}", col_name, col_type, col_identity, col_nullable, col_default)
    }).collect();

    let constraints_sql: Vec<String> = if table.primary_key.is_empty() {
        vec![]
    } else {
        let pk_name = quote(&format!("PK_{}", table.name));
        let pk_columns = table
            .primary_key
            .iter()
            .map(|col| quote(col))
            .collect::<Vec<String>>()
            .join(", ");
        vec![format!("CONSTRAINT {} PRIMARY KEY ({})", pk_name, pk_columns)]
    };

    // printf("CREATE TABLE %s (\n    %s\n);", quote_name, columns_sql);
    format!("CREATE TABLE {}.{} (\n    {}\n);", quote(&table.schema), quote(&table.name), 
        if constraints_sql.is_empty() {
            columns_sql.join(",\n    ")
        } else {
            format!("{},\n    {}", columns_sql.join(",\n    "), constraints_sql.join(",\n    "))
        }
    )
}

pub fn add_foreign_key_sql(table: &Table, fk: &ForeignKey) -> String {
    let fk_name = quote(&fk.name);
    let columns = fk.columns.iter().map(|col| quote(col)).collect::<Vec<String>>().join(", ");
    let ref_schema = quote(&fk.ref_schema);
    let ref_table: String = format!("{}.{}", ref_schema, quote(&fk.ref_table)); 
    let ref_columns: String = fk.ref_columns.iter().map(|col| quote(col)).collect::<Vec<String>>().join(", ");
    format!(
        "ALTER TABLE {}.{} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({});",
        quote(&table.schema),
        quote(&table.name),
        fk_name,
        columns,
        ref_table,
        ref_columns
    )
}

pub fn create_index_sql(table: &Table, index: &Index) -> String {
    let index_name = quote(&index.name);
    let columns = index.columns.iter().map(|col| quote(col)).collect::<Vec<String>>().join(", ");
    let unique = if index.unique { " UNIQUE" } else { "" };
    format!(
        "CREATE{} INDEX {} ON {}.{} ({});",
        unique,
        index_name,
        quote(&table.schema),
        quote(&table.name),
        columns
    )
}

pub fn generate_comments(table: &Table) -> Vec<String> {
    let mut comments = Vec::new();
    if !table.description.is_empty() {
        comments.push(format!(
            "EXEC sp_addextendedproperty @name = N'MS_Description', @value = N'{}', @level0type = N'SCHEMA', @level0name = N'{}', @level1type = N'TABLE', @level1name = N'{}';",
            table.description,
            table.schema,
            table.name
        ));
    }
    for column in &table.columns {
        if !column.description.is_empty() {
            comments.push(format!(
                "EXEC sp_addextendedproperty @name = N'MS_Description', @value = N'{}', @level0type = N'SCHEMA', @level0name = N'{}', @level1type = N'TABLE', @level1name = N'{}', @level2type = N'COLUMN', @level2name = N'{}';",
                column.description,
                table.schema,
                table.name,
                column.name
            ));
        }
    }
    comments
}

pub fn generate_ddl_table(table: &Table) -> Vec<String> {
    let mut ddls = vec![create_table_sql(table)];
    for fk in &table.foreign_keys {
        ddls.push(add_foreign_key_sql(table, fk));
    }
    for index in &table.indexes {
        ddls.push(create_index_sql(table, index));
    }
    ddls.extend(generate_comments(table));
    ddls
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlserver::model_for_sqlserver::{Column, SqlType, Table, ForeignKey};

    #[test]
    fn test_create_table_sql_with_primary_key() {
        let table = Table {
            schema: "dbo".to_string(),
            name: "Users".to_string(),
            columns: vec![
                Column { name: "Id".to_string(), ty: SqlType::Int, nullable: false , identity: true , default: None, description: "".to_string() },
                Column { name: "Name".to_string(), ty: SqlType::NVarChar(Some(100)), nullable: false , identity: false , default: None, description: "".to_string() },
                Column { name: "Email".to_string(), ty: SqlType::NVarChar(Some(255)), nullable: true , identity: false , default: None, description: "".to_string() },
                Column { name: "CreatedAt".to_string(), ty: SqlType::DateTime2, nullable: false , identity: false , default: Some("GETDATE()".to_string()), description: "".to_string() },
            ],
            primary_key: vec!["Id".to_string()],
            foreign_keys: vec![],
            indexes: vec![],
            description: "".to_string()
        };
        let sql = create_table_sql(&table);
        let expected_sql = "CREATE TABLE [dbo].[Users] (\n    [Id] INT IDENTITY(1,1) NOT NULL,\n    [Name] NVARCHAR(100) NOT NULL,\n    [Email] NVARCHAR(255) NULL,\n    [CreatedAt] DATETIME2 NOT NULL DEFAULT GETDATE(),\n    CONSTRAINT [PK_Users] PRIMARY KEY ([Id])\n);";
        println!("{}", sql);
        assert_eq!(sql, expected_sql);
    }

    #[test]
    fn test_create_table_sql_without_primary_key() {
        let table = Table {
            schema: "dbo".to_string(),
            name: "Users".to_string(),
            columns: vec![
                Column { name: "Id".to_string(), ty: SqlType::Int, nullable: false , identity: true , default: None, description: "".to_string() },
                Column { name: "Name".to_string(), ty: SqlType::NVarChar(Some(100)), nullable: false , identity: false , default: None, description: "".to_string() },
                Column { name: "Email".to_string(), ty: SqlType::NVarChar(Some(255)), nullable: true , identity: false , default: None, description: "".to_string() },
                Column { name: "CreatedAt".to_string(), ty: SqlType::DateTime2, nullable: false , identity: false , default: Some("GETDATE()".to_string()), description: "".to_string() },
            ],
            primary_key: vec![],
            foreign_keys: vec![],
            indexes: vec![],
            description: "".to_string()
        };
        let sql = create_table_sql(&table);
        let expected_sql = "CREATE TABLE [dbo].[Users] (\n    [Id] INT IDENTITY(1,1) NOT NULL,\n    [Name] NVARCHAR(100) NOT NULL,\n    [Email] NVARCHAR(255) NULL,\n    [CreatedAt] DATETIME2 NOT NULL DEFAULT GETDATE()\n);";
        println!("{}", sql);
        assert_eq!(sql, expected_sql);
    }

    #[test]
    fn test_create_table_sql_with_multi_primary_key() {
        let table = Table {
            schema: "dbo".to_string(),
            name: "Users".to_string(),
            columns: vec![
                Column { name: "Id".to_string(), ty: SqlType::Int, nullable: false , identity: true , default: None, description: "".to_string() },
                Column { name: "Name".to_string(), ty: SqlType::NVarChar(Some(100)), nullable: false , identity: false , default: None, description: "".to_string() },
                Column { name: "Email".to_string(), ty: SqlType::NVarChar(Some(255)), nullable: true , identity: false , default: None, description: "".to_string() },
                Column { name: "CreatedAt".to_string(), ty: SqlType::DateTime2, nullable: false , identity: false , default: Some("GETDATE()".to_string()), description: "".to_string() },
            ],
            primary_key: vec!["Id".to_string(), "Name".to_string()],
            foreign_keys: vec![],
            indexes: vec![],
            description: "".to_string()
        };
        let sql = create_table_sql(&table);
        let expected_sql = "CREATE TABLE [dbo].[Users] (\n    [Id] INT IDENTITY(1,1) NOT NULL,\n    [Name] NVARCHAR(100) NOT NULL,\n    [Email] NVARCHAR(255) NULL,\n    [CreatedAt] DATETIME2 NOT NULL DEFAULT GETDATE(),\n    CONSTRAINT [PK_Users] PRIMARY KEY ([Id], [Name])\n);";
        println!("{}", sql);
        assert_eq!(sql, expected_sql);
    }

    // #[test]
    // fn test_create_table_sql_with_primary_key_keyname_contains_kakko() {
    //     let table = Table {
    //         schema: "dbo".to_string(),
    //         name: "Users".to_string(),
    //         columns: vec![
    //             Column { name: "Id[".to_string(), ty: SqlType::Int, nullable: false , identity: true , default: None },
    //             Column { name: "Name".to_string(), ty: SqlType::NVarChar(Some(100)), nullable: false , identity: false , default: None },
    //         ],
    //         primary_key: vec!["Id".to_string()],
    //         foreign_keys: vec![],
    //         indexes: vec![],
    //     };
    //     let sql = create_table_sql(&table);
    //     let expected_sql = "CREATE TABLE [dbo].[Users] (\n    [Id[] INT IDENTITY(1,1) NOT NULL,\n    [Name] NVARCHAR(100) NOT NULL,\n    CONSTRAINT [PK_Users(Id[])] PRIMARY KEY ([Id[]])\n);";
    //     println!("{}", sql);
    //     assert_eq!(sql, expected_sql);
    // }

    #[test]
    fn test_alter_table_add_foreign_key() {
        let table = Table {
            schema: "dbo".to_string(),
            name: "Orders".to_string(),
            columns: vec![
                Column { name: "Id".to_string(), ty: SqlType::Int, nullable: false , identity: true , default: None, description: "".to_string() },
                Column { name: "UserId".to_string(), ty: SqlType::Int, nullable: false , identity: false , default: None, description: "".to_string() },
            ],
            primary_key: vec!["Id".to_string()],
            foreign_keys: vec![
                ForeignKey {
                    name: "FK_Orders_Users".to_string(),
                    columns: vec!["UserId".to_string()],
                    ref_schema: "dbo".to_string(),
                    ref_table: "Users".to_string(),
                    ref_columns: vec!["Id".to_string()],
                }
            ],
            indexes: vec![],
            description: "".to_string()
        };
        let sql = add_foreign_key_sql(&table, &table.foreign_keys[0]);
        let expected_sql = "ALTER TABLE [dbo].[Orders] ADD CONSTRAINT [FK_Orders_Users] FOREIGN KEY ([UserId]) REFERENCES [dbo].[Users] ([Id]);";
        println!("{}", sql);
        assert_eq!(sql, expected_sql);  
    }

    #[test]
    fn test_alter_table_add_multiple_foreign_key() {
        let table = Table {
            schema: "dbo".to_string(),
            name: "Orders".to_string(),
            columns: vec![
                Column { name: "Id".to_string(), ty: SqlType::Int, nullable: false , identity: true , default: None, description: "".to_string() },
                Column { name: "UserId".to_string(), ty: SqlType::Int, nullable: false , identity: false , default: None, description: "".to_string() },
                Column { name: "ProductId".to_string(), ty: SqlType::Int, nullable: false , identity: false , default: None, description: "".to_string() },
            ],
            primary_key: vec!["Id".to_string()],
            foreign_keys: vec![
                ForeignKey {
                    name: "FK_Orders_Users".to_string(),
                    columns: vec!["UserId".to_string()],
                    ref_schema: "dbo".to_string(),
                    ref_table: "Users".to_string(),
                    ref_columns: vec!["Id".to_string()],
                },
                ForeignKey {
                    name: "FK_Orders_Products".to_string(),
                    columns: vec!["ProductId".to_string()],
                    ref_schema: "dbo".to_string(),
                    ref_table: "Products".to_string(),
                    ref_columns: vec!["Id".to_string()],
                }
            ],
            indexes: vec![],
            description: "".to_string()
        };
        let sql1 = add_foreign_key_sql(&table, &table.foreign_keys[0]);
        let sql2 = add_foreign_key_sql(&table, &table.foreign_keys[1]);
        let expected_sql1 = "ALTER TABLE [dbo].[Orders] ADD CONSTRAINT [FK_Orders_Users] FOREIGN KEY ([UserId]) REFERENCES [dbo].[Users] ([Id]);";
        let expected_sql2 = "ALTER TABLE [dbo].[Orders] ADD CONSTRAINT [FK_Orders_Products] FOREIGN KEY ([ProductId]) REFERENCES [dbo].[Products] ([Id]);";
        println!("{}", sql1);
        println!("{}", sql2);
        assert_eq!(sql1, expected_sql1);
        assert_eq!(sql2, expected_sql2);
    }


    #[test]
    fn test_alter_table_add_foreign_key_with_multiple_columns() {
        let table = Table {
            schema: "dbo".to_string(),
            name: "Orders".to_string(),
            columns: vec![
                Column { name: "Id".to_string(), ty: SqlType::Int, nullable: false , identity: true , default: None, description: "".to_string() },
                Column { name: "UserId".to_string(), ty: SqlType::Int, nullable: false , identity: false , default: None, description: "".to_string() },
                Column { name: "ProductId".to_string(), ty: SqlType::Int, nullable: false , identity: false , default: None, description: "".to_string() },
            ],
            primary_key: vec!["Id".to_string()],
            foreign_keys: vec![
                ForeignKey {
                    name: "FK_Orders_User_Product".to_string(),
                    columns: vec!["UserId".to_string(), "ProductId".to_string()],
                    ref_schema: "dbo".to_string(),
                    ref_table: "UserProducts".to_string(),
                    ref_columns: vec!["UserId".to_string(), "ProductId".to_string()],
                }
            ],
            indexes: vec![],
            description: "".to_string()
        };
        let sql = add_foreign_key_sql(&table, &table.foreign_keys[0]);
        let expected_sql = "ALTER TABLE [dbo].[Orders] ADD CONSTRAINT [FK_Orders_User_Product] FOREIGN KEY ([UserId], [ProductId]) REFERENCES [dbo].[UserProducts] ([UserId], [ProductId]);";
        println!("{}", sql);
        assert_eq!(sql, expected_sql);
    }

    #[test]
    fn test_create_index_sql_non_unique() {
        let table = Table {
            schema: "dbo".to_string(),
            name: "Orders".to_string(),
            columns: vec![
                Column { name: "Id".to_string(), ty: SqlType::Int, nullable: false , identity: true , default: None, description: "".to_string() },
                Column { name: "UserId".to_string(), ty: SqlType::Int, nullable: false , identity: false , default: None, description: "".to_string() },
                Column { name: "ProductId".to_string(), ty: SqlType::Int, nullable: false , identity: false , default: None, description: "".to_string() },
            ],
            primary_key: vec!["Id".to_string()],
            foreign_keys: vec![],
            indexes: vec![
                Index {
                    name: "IX_Orders_UserId".to_string(),
                    columns: vec!["UserId".to_string()],
                    unique: false,
                }
            ],
            description: "".to_string()
        };
        let sql = create_index_sql(&table, &table.indexes[0]);
        let expected_sql = "CREATE INDEX [IX_Orders_UserId] ON [dbo].[Orders] ([UserId]);";
        println!("{}", sql);
        assert_eq!(sql, expected_sql);
    }

    #[test]
    fn test_create_index_sql_unique() {
        let table = Table {
            schema: "dbo".to_string(),
            name: "Orders".to_string(),
            columns: vec![
                Column { name: "Id".to_string(), ty: SqlType::Int, nullable: false , identity: true , default: None, description: "".to_string() },
                Column { name: "UserId".to_string(), ty: SqlType::Int, nullable: false , identity: false , default: None, description: "".to_string() },
                Column { name: "ProductId".to_string(), ty: SqlType::Int, nullable: false , identity: false , default: None, description: "".to_string() },
            ],
            primary_key: vec!["Id".to_string()],
            foreign_keys: vec![],
            indexes: vec![
                Index {
                    name: "IX_Orders_UserId".to_string(),
                    columns: vec!["UserId".to_string()],
                    unique: true,
                }
            ],
            description: "".to_string()
        };
        let sql = create_index_sql(&table, &table.indexes[0]);
        let expected_sql = "CREATE UNIQUE INDEX [IX_Orders_UserId] ON [dbo].[Orders] ([UserId]);";
        println!("{}", sql);
        assert_eq!(sql, expected_sql);
    }

    #[test]
    fn test_create_index_sql_multiple_columns() {
        let table = Table {
            schema: "dbo".to_string(),
            name: "Orders".to_string(),
            columns: vec![
                Column { name: "Id".to_string(), ty: SqlType::Int, nullable: false , identity: true , default: None, description: "".to_string() },
                Column { name: "UserId".to_string(), ty: SqlType::Int, nullable: false , identity: false , default: None, description: "".to_string() },
                Column { name: "ProductId".to_string(), ty: SqlType::Int, nullable: false , identity: false , default: None, description: "".to_string() },
            ],
            primary_key: vec!["Id".to_string()],
            foreign_keys: vec![],
            indexes: vec![
                Index {
                    name: "IX_Orders_UserId_ProductId".to_string(),
                    columns: vec!["UserId".to_string(), "ProductId".to_string()],
                    unique: false,
                }
            ],
            description: "".to_string()
        };
        let sql = create_index_sql(&table, &table.indexes[0]);
        let expected_sql = "CREATE INDEX [IX_Orders_UserId_ProductId] ON [dbo].[Orders] ([UserId], [ProductId]);";
        println!("{}", sql);
        assert_eq!(sql, expected_sql);
    }

    #[test]
    fn test_generate_ddl_table() {
        let table = Table {
            schema: "dbo".to_string(),
            name: "Orders".to_string(),
            columns: vec![
                Column { name: "Id".to_string(), ty: SqlType::Int, nullable: false , identity: true , default: None, description: "".to_string() },
                Column { name: "UserId".to_string(), ty: SqlType::Int, nullable: false , identity: false , default: None, description: "".to_string() },
                Column { name: "ProductId".to_string(), ty: SqlType::Int, nullable: false , identity: false , default: None, description: "".to_string() },
            ],
            primary_key: vec!["Id".to_string()],
            foreign_keys: vec![
                ForeignKey {
                    name: "FK_Orders_UserId".to_string(),
                    columns: vec!["UserId".to_string()],
                    ref_schema: "dbo".to_string(),
                    ref_table: "Users".to_string(),
                    ref_columns: vec!["Id".to_string()],                  
                }
            ],
            indexes: vec![
                Index {
                    name: "IX_Orders_UserId".to_string(),
                    columns: vec!["UserId".to_string()],
                    unique: true,
                }
            ],
            description: "".to_string()
        };
        let ddls = generate_ddl_table(&table);
        for ddl in &ddls {
            println!("{}", ddl);
        }
        assert_eq!(ddls[0], "CREATE TABLE [dbo].[Orders] (\n    [Id] INT IDENTITY(1,1) NOT NULL,\n    [UserId] INT NOT NULL,\n    [ProductId] INT NOT NULL,\n    CONSTRAINT [PK_Orders] PRIMARY KEY ([Id])\n);");
        assert_eq!(ddls[1], "ALTER TABLE [dbo].[Orders] ADD CONSTRAINT [FK_Orders_UserId] FOREIGN KEY ([UserId]) REFERENCES [dbo].[Users] ([Id]);");
        assert_eq!(ddls[2], "CREATE UNIQUE INDEX [IX_Orders_UserId] ON [dbo].[Orders] ([UserId]);");
    }

    #[test]
    fn test_generate_comments() {
        let table = Table {
            schema: "dbo".to_string(),
            name: "Orders".to_string(),
            columns: vec![
                Column { name: "Id".to_string(), ty: SqlType::Int, nullable: false , identity: true , default: None, description: "Primary key".to_string() },
                Column { name: "UserId".to_string(), ty: SqlType::Int, nullable: false , identity: false , default: None, description: "User ID".to_string() },
                Column { name: "ProductId".to_string(), ty: SqlType::Int, nullable: false , identity: false , default: None, description: "Product ID".to_string() },
            ],
            primary_key: vec!["Id".to_string()],
            foreign_keys: vec![],
            indexes: vec![],
            description: "Orders table".to_string()
        };
        let comments = generate_comments(&table);
        for comment in &comments {
            println!("{}", comment);
        }
        assert_eq!(comments[0], "EXEC sp_addextendedproperty @name = N'MS_Description', @value = N'Orders table', @level0type = N'SCHEMA', @level0name = N'dbo', @level1type = N'TABLE', @level1name = N'Orders';");
        assert_eq!(comments[1], "EXEC sp_addextendedproperty @name = N'MS_Description', @value = N'Primary key', @level0type = N'SCHEMA', @level0name = N'dbo', @level1type = N'TABLE', @level1name = N'Orders', @level2type = N'COLUMN', @level2name = N'Id';");
        assert_eq!(comments[2], "EXEC sp_addextendedproperty @name = N'MS_Description', @value = N'User ID', @level0type = N'SCHEMA', @level0name = N'dbo', @level1type = N'TABLE', @level1name = N'Orders', @level2type = N'COLUMN', @level2name = N'UserId';");
        assert_eq!(comments[3], "EXEC sp_addextendedproperty @name = N'MS_Description', @value = N'Product ID', @level0type = N'SCHEMA', @level0name = N'dbo', @level1type = N'TABLE', @level1name = N'Orders', @level2type = N'COLUMN', @level2name = N'ProductId';");
    }

    #[test]
    fn test_generate_ddl_and_comments() {
        let table = Table {
            schema: "dbo".to_string(),
            name: "Orders".to_string(),
            columns: vec![
                Column { name: "Id".to_string(), ty: SqlType::Int, nullable: false , identity: true , default: None, description: "Primary key".to_string() },
                Column { name: "UserId".to_string(), ty: SqlType::Int, nullable: false , identity: false , default: None, description: "User ID".to_string() },
                Column { name: "ProductId".to_string(), ty: SqlType::Int, nullable: false , identity: false , default: None, description: "Product ID".to_string() },
            ],
            primary_key: vec!["Id".to_string()],
            foreign_keys: vec![],
            indexes: vec![],
            description: "Orders table".to_string()
        };
        let ddls = generate_ddl_table(&table);
        let comments = generate_comments(&table);
        for ddl in &ddls {
            println!("{}", ddl);
        }
        for comment in &comments {
            println!("{}", comment);
        }
    
    }
}