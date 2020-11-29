use nom::branch::alt;
use nom::bytes::complete::{is_not, tag, take_while, take_while1};
use nom::combinator::{all_consuming, eof};
use nom::multi::many0;
use nom::sequence::delimited;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Token {
    // TODO: add keywords here? they are reserved in sql
    // TODO: add numbers + operators (later)
    Identifier(String),
    Literal(String),
    Comma,
    Parens(Vec<Token>),  // TODO: ask Lukasz if it's normal to match parens in Tokenizer
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

fn tokenize_internal(s: &str) -> nom::IResult<&str, Vec<Token>> {
    let (s, r) = many0(alt((
        comment,
        parens,
        literal,
        const_token(",", Token::Comma),
        whitespace,
        identifier,
    )))(s)?;

    Ok((s, r.into_iter().flatten().collect()))
}

pub fn tokenize(s: &str) -> nom::IResult<&str, Vec<Token>> {
    all_consuming(tokenize_internal)(s)
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
            vec![Token::Identifier("id1".to_owned()), Token::Identifier("id3".to_owned())]
        )
    }
}
