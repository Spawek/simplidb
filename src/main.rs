extern crate nom;

use simplidb::parser::{SelectExpression, DataSource};
use std::path::Path;
use simplidb::database::{Database, Column};
use std::io;
use simplidb::csv::read_csv;

fn execute(expression: SelectExpression, db: Database) -> Vec<Column> {
    let columns = match expression.source {
        DataSource::Datastore { name } => db
            .datastores
            .iter()
            .find(|x| x.name == name)
            .expect(&format!("No table: {} found", &name))
            .columns
            .to_owned(), // NOTE: COPY!
        DataSource::SelectExpression(subselect) => execute(*subselect, db),
    };

    expression
        .columns
        .iter()
        .map(|x| {
            columns
                .iter()
                .find(|y| y.name == *x)
                .expect(&format!("No column: {} found", x))
                .to_owned()
        })
        .collect() // TODO: handle ambiguity
}

#[derive(Debug, PartialEq)]
pub enum SqlParseError {
    Err(String),
}

fn main() -> std::result::Result<(), io::Error> {
    let employee = read_csv(
        Path::new(r"C:\maciek\programowanie\simplidb\database\employee.csv"),
        "employee",
    )?;
    println!("{:#?}", employee);

    let db = Database {
        datastores: vec![employee],
    };

    let select = SelectExpression {
        columns: vec!["name".to_owned()],
        source: DataSource::Datastore {
            name: "employee".to_owned(),
        },
    };

    let result = execute(select, db);
    println!("query result: {:#?}", result);

    Ok(())
}

// TODO: can track if file info is up to date by checking file modification time
// TODO: serialize deserialize tests
