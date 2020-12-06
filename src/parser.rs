use crate::lexer::{tokenize, Token};
use core::fmt;
use nom::error::{ErrorKind, ParseError};
use nom::IResult;
use std::error::Error;
use nom::bytes::complete::take;

#[derive(Debug, PartialEq)]
pub struct SelectExpression {
    pub columns: Vec<String>,
    pub source: DataSource,
}

#[derive(Debug, PartialEq)]
pub enum DataSource {
    Datastore { name: String },
    SelectExpression(Box<SelectExpression>),
}

pub fn parse(s: &str) -> Result<SelectExpression, String> {
    let (_, tokens) = tokenize(s).map_err(|e| e.to_string())?;
    let (remaining, parsed) = parse_internal(tokens.as_slice()).map_err(|e| e.to_string())?;
    if !remaining.is_empty() {
        return Err(format!(
            "there are remaining tokens that were not parsed: {:?}",
            &remaining
        ));
    }
    Ok(parsed)
}

#[derive(Debug, PartialEq)]
// TODO: add <I> template and pass it to NomError if it can be helpful
pub enum SqlParseError {
    CustomError(String),
    Eof,
    RemainingTokens(Vec<Token>),
    NomError(ErrorKind),
}

impl Error for SqlParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl fmt::Display for SqlParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SqlParseError::CustomError(e) => {
                write!(f, "Custom error: {}", e)
            }
            SqlParseError::Eof => {
                write!(f, "Unexpected end of file")
            }
            SqlParseError::NomError(kind) => {
                write!(f, "Nom could not parse input - err: {:?}", &kind)
            }
            SqlParseError::RemainingTokens(tokens) => {
                write!(
                    f,
                    "There are remaining tokens which were not parsed: {:?}",
                    &tokens
                )
            }
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

fn parse_internal(s: &[Token]) -> IResult<&[Token], SelectExpression, SqlParseError> {
    let (s, _) = take_keyword("SELECT")(s)?;
    let (s, col) = take_identifier(s)?;
    let (s, _) = take_keyword("FROM")(s)?;
    let (s, table) = take_identifier(s)?;

    Ok((
        s,
        SelectExpression {
            columns: vec![col],
            source: DataSource::Datastore {
                name: table
            },
        },
    ))
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

    Err(nom::Err::Error(SqlParseError::CustomError(
        "identifier not matched".to_owned(),
    )))
}

/// Matches given keyword - case insensitive.
fn take_keyword(
    name: &str,
) -> Box<dyn Fn(&[Token]) -> nom::IResult<&[Token], &[Token], SqlParseError>> {
    let name = name.to_owned();
    Box::new(move |i: &[Token]| {
        let elem = match i.first() {
            Some(v) => v,
            None => {
                return Err(nom::Err::Error(SqlParseError::Eof));
            }
        };

        if let Token::Keyword(v) = elem {
            if v.to_lowercase() == name.to_lowercase(){
                return Ok((&i[1..], &i[..1]));
            }
        }

        Err(nom::Err::Error(SqlParseError::CustomError(format!(
            "keyword: {:?} not matched",
            &name
        ))))
    })
}

fn take_token(
    token: Token,
) -> Box<dyn Fn(&[Token]) -> nom::IResult<&[Token], &[Token], SqlParseError>> {
    Box::new(move |i: &[Token]| {
        let elem = match i.first() {
            Some(v) => v,
            None => {
                return Err(nom::Err::Error(SqlParseError::Eof));
            }
        };

        if &token == elem {
            return Ok((&i[1..], &i[..1]));
        }

        Err(nom::Err::Error(SqlParseError::CustomError(format!(
            "token: {:?} not matched",
            &token
        ))))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_from() {
        assert_eq!(
            parse("SELECT x FROM t").unwrap(),
            SelectExpression {
                columns: vec!["x".to_owned()],
                source: DataSource::Datastore {
                    name: "t".to_owned()
                }
            }
        );
    }
}
