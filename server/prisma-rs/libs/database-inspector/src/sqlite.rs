use prisma_query::Connectional;
use std::sync::Arc;
use crate::*;
use crate::database_inspector_impl::{convert_introspected_columns, IntrospectedForeignKey };
use prisma_query::ast::ParameterizedValue;

pub struct Sqlite {
    pub connectional: Arc<Connectional>
}

impl DatabaseInspector for Sqlite {
    fn introspect(&self, schema: &String) -> DatabaseSchema {
        DatabaseSchema {
            tables: self.get_table_names(schema)
                .into_iter()
                .map(|t| self.get_table(schema, &t))
                .collect(),
        }
    }
}

impl Sqlite {
    pub fn new(connectional: Arc<Connectional>) -> Sqlite {
        Sqlite { 
            connectional: connectional
        }
    }

    fn get_table_names(&self, schema: &String) -> Vec<String> {
        let sql = format!(
            "
            SELECT
                name
            FROM
                {}.sqlite_master
            WHERE
                type='table'
        ",
            schema
        );

        let result_set = self.connectional.query_on_raw_connection(&schema, &sql, &[]).unwrap();
        let names = result_set
            .into_iter()
            .map(|row| row.get_as_string("name").unwrap())
            .filter(|n| n != "sqlite_sequence")
            .collect();
        names
    }

    fn get_table(&self, schema: &String, table: &String) -> Table {
        let introspected_columns = self.get_columns(&schema, &table);
        let introspected_foreign_keys = self.get_foreign_constraints(&schema, &table);

        let mut columns_copy = introspected_columns.clone();
        columns_copy.sort_by_key(|c| c.pk);
        let pk_columns = columns_copy
            .into_iter()
            .filter(|c| c.pk > 0) // only the columns with a value greater 0 are part of the primary key
            .map(|c| c.name)
            .collect();

        Table {
            name: table.to_string(),
            columns: convert_introspected_columns(introspected_columns, introspected_foreign_keys),
            indexes: Vec::new(),
            primary_key_columns: pk_columns,
        }
    }

    fn get_columns(&self, schema: &String, table: &String) -> Vec<IntrospectedColumn> {
        let sql = format!(r#"Pragma "{}".table_info ("{}")"#, schema, table);

        let result_set = self.connectional.query_on_raw_connection(&schema, &sql, &[]).unwrap();
        let columns = result_set
                    .into_iter()
                    .map(|row| {
                        let default_value = match row.get("dflt_value") {
                            Ok(ParameterizedValue::Text(v)) => Some(v.clone()),
                            Ok(ParameterizedValue::Null) => None,
                            Ok(p) => panic!(format!("expectd a string value but got {:?}", p)),
                            Err(err) => panic!(format!("{}", err)),
                        };
                        IntrospectedColumn {
                            name: row.get_as_string("name").unwrap(),
                            table: table.to_string(),
                            tpe: row.get_as_string("type").unwrap(),
                            is_required: row.get_as_bool("notnull").unwrap(),
                            default: default_value,
                            pk: row.get_as_integer("pk").unwrap() as u32,
                        }
                    })
                    .collect();

        columns
    }

    fn get_foreign_constraints(&self, schema: &String, table: &String) -> Vec<IntrospectedForeignKey> {
        let sql = format!(r#"Pragma "{}".foreign_key_list("{}");"#, schema, table);

        let result_set = self.connectional.query_on_raw_connection(&schema, &sql, &[]).unwrap();

        let foreign_keys = result_set
            .into_iter()
            .map(|row| IntrospectedForeignKey {
                name: "".to_string(),
                table: table.to_string(),
                column: row.get_as_string("from").unwrap(),
                referenced_table: row.get_as_string("table").unwrap(),
                referenced_column: row.get_as_string("to").unwrap(),
            })
            .collect();
        
        foreign_keys
    }

    #[allow(unused)]
    fn get_sequence(&self, _schema: &String, _table: &String) -> Sequence {
        unimplemented!()
    }

    #[allow(unused)]
    fn get_index(&self, _schema: &String, _table: &String) -> Index {
        unimplemented!()
    }
}