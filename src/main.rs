extern crate nom;

use std::path::{Path, PathBuf};
use std::{fs, io};
use nom::error::ErrorKind;
use nom::error::ParseError;
use nom::Err::Error;
use nom::IResult;
use nom::bytes::complete::{tag, take_while, take_while1};
use nom::branch::alt;
use nom::sequence::tuple;
use nom::multi::{many_till, many0};
use crate::TextToken::Whitespace;

#[derive(Debug)]
struct Database {
    datastores: Vec<Datastore>,
}

#[derive(Debug)]
struct Datastore {
    name: String,
    path: PathBuf,
    columns: Vec<Column>, // TODO: change to read data on-demand
}

#[derive(Debug, Clone)] // TODO: remove Copy/Clone
struct Column {
    name: String,
    data: Vec<String>,
}

fn read_csv(path: &Path, name: &str) -> io::Result<Datastore> {
    let file = fs::read_to_string(path)?;
    let split = file
        .split("\n")
        .filter(|line| !line.is_empty())
        .map(|line| line.replace("\r", ""))
        .map(|line| {
            line.split(",")
                .map(|x| x.to_owned())
                .collect::<Vec<String>>()
        })
        .collect::<Vec<_>>();

    let header = match split.first() {
        None => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("No header in CSV: {}", path.display()),
            ));
        }
        Some(v) => v,
    };

    if header.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Empty header in CSV: {}", path.display()),
        ));
    }

    let data = split.iter().skip(1).collect::<Vec<_>>();

    let mut columns = header
        .into_iter()
        .map(|h| Column {
            name: (*h).to_owned(),
            data: vec![],
        })
        .collect::<Vec<_>>();

    for row in data {
        if row.len() != header.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Inconsistent amount of data in row: {:?} (should have: {} columns, has: {})",
                    row,
                    header.len(),
                    row.len()
                ),
            ));
        }
        for i in 0..row.len() {
            let val = (*row.get(i).unwrap()).to_owned();
            columns.get_mut(i).unwrap().data.push(val);
        }
    }

    Ok(Datastore {
        path: path.to_owned(),
        name: name.to_owned(),
        columns,
    })
}

#[derive(Debug)]
enum DataSource {
    Datastore { name: String },
    SelectExpression(Box<SelectExpression>),
}

#[derive(Debug)]
struct SelectExpression {
    columns: Vec<String>,
    source: DataSource,
}

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

impl<I> ParseError<I> for SqlParseError {
    fn from_error_kind(input: I, kind: ErrorKind) -> Self {
        SqlParseError::Err(format!("From Nom error: {:?}", kind))
    }

    fn append(_: I, _: ErrorKind, other: Self) -> Self {
        other
    }
}

impl SqlParseError {
    fn new(s :&str) -> SqlParseError {
        return SqlParseError::Err(s.to_owned());
    }
}

// https://codeandbitters.com/lets-build-a-parser/

#[derive(Debug, Clone)]
enum TextToken {
    Identifier(String),
    Literal(String),
    Comma,
    Parens(Vec<TextToken>),
    Whitespace,
}

fn parens(s: &str) -> nom::IResult<&str, TextToken> {
    let (s, _) = tag("(")(s)?;
    let (s, subtokens) = text_tokenize(s)?;
    let (s, _) = tag(")")(s)?;
    Ok((s, TextToken::Parens(subtokens)))
}

fn is_literal_character(c: char) -> bool {
    c != '"'
}

fn literal(s: &str) -> nom::IResult<&str, TextToken> {
    let (s, _) = tag("\"")(s)?;
    let (s, text) = take_while(is_literal_character)(s)?;
    let (s, _) = tag("\"")(s)?;
    Ok((s, TextToken::Literal(text.to_owned())))
}

fn const_token(name: &str, token: TextToken) -> Box<dyn Fn(&str) -> nom::IResult<&str, TextToken>> {
    let name = name.to_owned();
    Box::new(move |s| tag(&*name)(s).map(|(x, _)| (x, token.clone())))
}

fn is_whitespace(c: char) -> bool {
    c == ' ' || c == '\n' || c == '\t'
}

fn whitespace(s: &str) -> nom::IResult<&str, TextToken> {
    let (s, _) = take_while1(is_whitespace)(s)?;
    Ok((s, TextToken::Whitespace))
}

// should work for `(")")`
fn text_tokenize(s: &str) -> nom::IResult<&str, Vec<TextToken>> {
    let (s, r) = alt((
        parens,
        literal,
        const_token(",", TextToken::Comma),
        whitespace,
        // identifier()
    )
    )(s)?;
    Ok((s, vec![r]))
}

#[derive(Debug, Clone)]
enum SqlToken {
    // Keywords
    Select,
    From,
    As,

    Identifier(String),
    Literal(String),
    Comma,
}

fn keyword(name: &str, token: SqlToken) -> Box<dyn Fn(&str) -> nom::IResult<&str, SqlToken>> {
    let name = name.to_owned();
    Box::new(move |s| tag(&*name)(s).map(|(x, _)| (x, token.clone())))
}

// TODO: change input to TextTokens
fn sql_tokenize(s: &str) -> nom::IResult<&str, Vec<SqlToken>> {
    // TODO: use lazy static if it's super slow
    // TODO: split to words first so "SELECT_DSADSA" doesn't match with "SELECT"
    let (s, x) = alt(
        (
            keyword("select", SqlToken::Select),
            keyword("from", SqlToken::From),
            keyword(",", SqlToken::Comma),
        )
        // tag("from"),
        //
        // tag(",")
    )(s)?;

    Ok((s, vec![x]))
}

// // // TODO: to_lowercase?
// fn parse(s: &str) -> nom::IResult<&str, SelectExpression, SqlParseError> {
//     let (s, _) = nom::bytes::complete::tag("select")(s)?;
//     let (s, columns) = nom::multi::separated_list0(tag(","), );
//     let (s, _) = nom::bytes::complete::tag("from")(s)?;
//     Ok((s, SelectExpression{ columns: vec![], source: DataSource::Datastore{ name: "".to_owned() }}))
// }

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

    // dbg!(parse("select a,b from employees"));
    // dbg!(text_tokenize(r#"(")")"#));
    // dbg!(text_tokenize(r#"Identifier (")")"#));
    dbg!(text_tokenize(r#"((""))"#));

    Ok(())
}

// TODO: can track if file info is up to date by checking file modification time
// TODO: serialize deserialize tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_csv_with_2_columns() {
        let tempdir = tempfile::tempdir().unwrap();
        let csv_path = tempdir.path().join("data.csv");
        std::fs::write(&csv_path, "col1,col2\n1,2\n2,3").unwrap();

        let datastore = read_csv(&csv_path, "data").unwrap();
        assert_eq!(datastore.columns.len(), 2);

        assert_eq!(datastore.columns.get(0).unwrap().name, "col1");
        assert_eq!(datastore.columns.get(0).unwrap().data.len(), 2);
        assert_eq!(datastore.columns.get(0).unwrap().data.get(0).unwrap(), "1");
        assert_eq!(datastore.columns.get(0).unwrap().data.get(1).unwrap(), "2");

        assert_eq!(datastore.columns.get(1).unwrap().name, "col2");
        assert_eq!(datastore.columns.get(1).unwrap().data.len(), 2);
        assert_eq!(datastore.columns.get(1).unwrap().data.get(0).unwrap(), "2");
        assert_eq!(datastore.columns.get(1).unwrap().data.get(1).unwrap(), "3");
    }
}
