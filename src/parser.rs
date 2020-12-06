use crate::lexer::{tokenize, Token};
use nom::IResult;
use nom::error::{ErrorKind, ParseError};
use std::error::Error;
use core::fmt;

#[derive(Debug)]
pub struct SelectExpression {
    pub columns: Vec<String>,
    pub source: DataSource,
}

#[derive(Debug)]
pub enum DataSource {
    Datastore { name: String },
    SelectExpression(Box<SelectExpression>),
}

pub fn parse(s: &str) -> Result<SelectExpression, String>{
    let (_, tokens) = tokenize(s).map_err(|e| e.to_string())?;
    let (_, parsed) = parse_internal(tokens.as_slice()).map_err(|e| e.to_string())?;
    Ok(parsed)
}

#[derive(Debug, PartialEq)]
// TODO: add <I> template and pass it to NomError if it can be helpful
pub enum SqlParseError {
    CustomError(String),
    Eof,
    NomError(ErrorKind),
}

impl Error for SqlParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl fmt::Display for SqlParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self{
            SqlParseError::CustomError(e) => { write!(f, "Custom error: {}", e) }
            SqlParseError::Eof  => { write!(f, "Unexpected end of file") }
            SqlParseError::NomError(kind) => { write!(f, "Nom could not parse input - err: {:?}", &kind) }
        }
    }
}

impl<I> ParseError<I> for SqlParseError {
    fn from_error_kind(_input: I, kind: ErrorKind) -> Self {
        SqlParseError::NomError(kind)
    }

    fn append(_: I, _: ErrorKind, other: Self) -> Self {
        other
    }
}

fn parse_internal(s: &[Token]) -> IResult<&[Token], SelectExpression, SqlParseError>{
    unimplemented!();
    Ok((s, SelectExpression{ columns:vec![], source: DataSource::Datastore {name: "".to_owned()}}))
}

/// Matches identifier and returns its value.
fn take_identifier(s: &[Token]) -> nom::IResult<&[Token], String, SqlParseError> {
    let elem = match s.first() {
        Some(v) => v,
        None => {
            return Err(nom::Err::Error(SqlParseError::Eof));
        }
    };

    if let Token::Identifier(v) = elem {
        return Ok((&s[1..], v.to_owned()));
    }

    Err(nom::Err::Error(SqlParseError::CustomError("identifier not matched".to_owned())))
}

/// Matches given keyword.
fn take_keyword(name: &str) -> Box<dyn Fn(&[Token]) -> nom::IResult<&[Token], &[Token], SqlParseError>> { // TODO: change `Box` to `impl` like done in nom::Tag
    let name = name.to_owned();
    Box::new(move |i: &[Token]| {
        let elem = match i.first() {
            Some(v) => v,
            None => {
                return Err(nom::Err::Error(SqlParseError::Eof));
            }
        };

        if let Token::Identifier(curr) = elem {
            if curr == &name {
                return Ok((&i[1..], &i[..1]));
            }
        }

        Err(nom::Err::Error(SqlParseError::CustomError(format!("keyword: {} not matched", &name))))
    })
}