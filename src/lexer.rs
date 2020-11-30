use nom::branch::alt;
use nom::bytes::complete::{is_not, tag, take_while, take_while1};
use nom::combinator::{all_consuming, eof};
use nom::error::{Error, ErrorKind, ParseError};
use nom::multi::many0;
use nom::sequence::delimited;

// https://codeandbitters.com/lets-build-a-parser/

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Token {
    // TODO: add numbers + operators (later)
    Keyword(String),
    Identifier(String),
    Literal(String),
    Comma,
    Parens(Vec<Token>),
}

pub fn tokenize(s: &str) -> nom::IResult<&str, Vec<Token>> {
    all_consuming(tokenize_internal)(s)
}

fn parens(s: &str) -> nom::IResult<&str, Option<Token>> {
    let (s, _) = tag("(")(s)?;
    let (s, inner) = tokenize_internal(s)?;
    let (s, _) = tag(")")(s)?;
    Ok((s, Some(Token::Parens(inner))))
}

fn is_literal_character(c: char) -> bool {
    c != '"'
}

fn literal(s: &str) -> nom::IResult<&str, Option<Token>> {
    let (s, _) = tag("\"")(s)?;
    let (s, text) = take_while(is_literal_character)(s)?;
    let (s, _) = tag("\"")(s)?;
    Ok((s, Some(Token::Literal(text.to_owned()))))
}

// TODO: remove generic version if it's not used much
fn const_token(name: &str, token: Token) -> Box<dyn Fn(&str) -> nom::IResult<&str, Option<Token>>> {
    let name = name.to_owned();
    Box::new(move |s| tag(&*name)(s).map(|(x, _)| (x, Some(token.clone()))))
}

// TODO: check out multispace0
fn whitespace(s: &str) -> nom::IResult<&str, Option<Token>> {
    let (s, _) = take_while1(|c| c == ' ' || c == '\n' || c == '\t')(s)?;
    Ok((s, None)) // `None` because this token is not relevant
}

fn identifier(s: &str) -> nom::IResult<&str, Option<Token>> {
    let (s, r) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(s)?;
    Ok((s, Some(Token::Identifier(r.to_owned()))))
}

fn line_comment(s: &str) -> nom::IResult<&str, Option<Token>> {
    let (s, _) = tag("--")(s)?;
    let (s, _) = take_while(|c| c != '\n')(s)?;
    let (s, _) = alt((tag("\n"), eof))(s)?;
    Ok((s, None))
}

// TODO: use `delimited` to simplify other functions?
fn block_comment(s: &str) -> nom::IResult<&str, Option<Token>> {
    let (s, _) = delimited(tag("/*"), is_not("*/"), tag("*/"))(s)?;
    Ok((s, None))
}

fn comment(s: &str) -> nom::IResult<&str, Option<Token>> {
    alt((line_comment, block_comment))(s)
}

// case insensitive
fn take_identifier(name: &str) -> Box<dyn Fn(&[Token]) -> nom::IResult<&[Token], &[Token]>> {
    let name = name.to_owned();
    Box::new(move |i: &[Token]| {
        let elem = match i.first() {
            Some(v) => v,
            None => {
                return Err(nom::Err::Error(Error::from_error_kind(i, ErrorKind::Eof)));
            }
        };

        if let Token::Identifier(curr) = elem {
            if curr.to_lowercase() == name.to_lowercase() {
                return Ok((&i[1..], &i[..1]));
            }
        }

        Err(nom::Err::Error(Error::from_error_kind(  // TODO: create custom errors
            i,
            ErrorKind::TagBits,
        )))
    })
}

fn take_any(i: &[Token]) -> nom::IResult<&[Token], Token> {
    match i.first() {
        Some(v) => Ok((&i[1..], v.to_owned())),
        None => Err(nom::Err::Error(Error::from_error_kind(i, ErrorKind::Eof))),
    }
}

/// identifiers are split by " " token
/// keywords are returned in uppercase
fn keyword(name: &str) -> Box<dyn Fn(&[Token]) -> nom::IResult<&[Token], Token>> {
    let name = name.to_owned();
    Box::new(move |s| {
        let mut x = s;
        for n in name.split(" ") {
            x = take_identifier(n)(x)?.0;
        }
        Ok((&x, Token::Keyword(name.to_uppercase())))
    })
}

fn resolve_keywords(s: &[Token]) -> nom::IResult<&[Token], Vec<Token>> {
    // Keywords contained by other keywords must be placed after them - e.g.
    // "join" must be further in the list than "left join".
    let (s, r) = many0(alt((
        keyword("select"),
        keyword("from"),
        keyword("where"),
        keyword("left join"),
        keyword("join"),
        take_any,
    )))(s)?;

    Ok((s, r))
}

fn tokenize_internal(s: &str) -> nom::IResult<&str, Vec<Token>> {
    let (s, r) = many0(alt((
        comment,
        parens,
        literal,
        const_token(",", Token::Comma),
        whitespace,
        identifier,
    )))(s)?;

    let tokens: Vec<Token> = r.into_iter().flatten().collect();
    let (_, tokens) =
        all_consuming(resolve_keywords)(tokens.as_slice()).expect("resolve keywords failed"); // TODO: fix error passing  // NOTE: I don't think resolve keywords should ever fail as it accepts every token, so maybe there is no point of fighting with errors

    Ok((s, tokens))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::{Error, ErrorKind};
    use nom::Err;

    #[test]
    fn test_parens() {
        assert_eq!(tokenize("()").unwrap().1, vec![Token::Parens(vec![])]);
        assert_eq!(
            tokenize("(())").unwrap().1,
            vec![Token::Parens(vec![Token::Parens(vec![])])]
        );
        assert_eq!(
            tokenize(r#"(")(")"#).unwrap().1,
            vec![Token::Parens(vec![Token::Literal(")(".to_owned())])]
        );
    }

    #[test]
    fn test_unmatched_paren() {
        assert_eq!(
            tokenize("()(").unwrap_err(),
            Err::Error(Error {
                input: "(",
                code: ErrorKind::Eof
            })
        );
    }

    #[test]
    fn test_comma() {
        assert_eq!(
            tokenize(r#""a",,"b""#).unwrap().1,
            vec![
                Token::Literal("a".to_owned()),
                Token::Comma,
                Token::Comma,
                Token::Literal("b".to_owned())
            ]
        )
    }

    #[test]
    fn test_whitespace() {
        assert_eq!(
            tokenize(" \t\n\" a\t\n \"\t\n ").unwrap().1,
            vec![Token::Literal(" a\t\n ".to_owned()),]
        )
    }

    #[test]
    fn test_identifier() {
        assert_eq!(
            tokenize("id").unwrap().1,
            vec![Token::Identifier("id".to_owned())]
        )
    }

    #[test]
    fn test_line_comment() {
        assert_eq!(
            tokenize("id1--id2\nid3").unwrap().1,
            vec![
                Token::Identifier("id1".to_owned()),
                Token::Identifier("id3".to_owned())
            ]
        )
    }

    #[test]
    fn test_keywords() {
        assert_eq!(
            tokenize("SELECT x,y FROM t1 LEFT JOIN t2 JOIN t3")
                .unwrap()
                .1,
            vec![
                Token::Keyword("SELECT".to_owned()),
                Token::Identifier("x".to_owned()),
                Token::Comma,
                Token::Identifier("y".to_owned()),
                Token::Keyword("FROM".to_owned()),
                Token::Identifier("t1".to_owned()),
                Token::Keyword("LEFT JOIN".to_owned()),
                Token::Identifier("t2".to_owned()),
                Token::Keyword("JOIN".to_owned()),
                Token::Identifier("t3".to_owned()),
            ]
        );
    }
}
