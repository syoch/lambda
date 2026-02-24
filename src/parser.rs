use crate::lambda::DeBruijn;
use nom::{
    branch::alt,
    bytes::complete::take_while1,
    character::complete::{char, multispace0},
    IResult,
};

/// 文字列からラムダ式をパースする公開関数
pub fn parse(input: &str) -> Result<DeBruijn, String> {
    fn parse_impl<'a>(env: &Vec<&'a str>, input: &'a str) -> IResult<&'a str, DeBruijn> {
        application_impl(env, input)
    }

    fn application_impl<'a>(env: &Vec<&'a str>, input: &'a str) -> IResult<&'a str, DeBruijn> {
        let (input, first) = term_impl(env, input)?;
        let (mut input, _) = multispace0(input)?;

        let mut result = first;
        loop {
            match term_impl(env, input) {
                Ok((new_input, term)) => {
                    result = DeBruijn::App(Box::new(result), Box::new(term));
                    let (i, _) = multispace0(new_input)?;
                    input = i;
                }
                Err(_) => break,
            }
        }

        Ok((input, result))
    }

    fn term_impl<'a>(env: &Vec<&'a str>, input: &'a str) -> IResult<&'a str, DeBruijn> {
        let (input, _) = multispace0(input)?;
        alt((
            |i| abstraction_impl(env, i),
            |i| parens_impl(env, i),
            |i| variable_impl(env, i),
        ))(input)
    }

    fn variable_impl<'a>(env: &Vec<&'a str>, input: &'a str) -> IResult<&'a str, DeBruijn> {
        let (input, var_name) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;
        let (input, _) = multispace0(input)?;

        // 環境から変数を探す（内側から外側へ）
        for (i, bound_name) in env.iter().rev().enumerate() {
            if *bound_name == var_name {
                return Ok((input, DeBruijn::Var(i)));
            }
        }

        // 自由変数の場合
        Ok((input, DeBruijn::Var(env.len())))
    }

    fn parens_impl<'a>(env: &Vec<&'a str>, input: &'a str) -> IResult<&'a str, DeBruijn> {
        let (input, _) = char('(')(input)?;
        let (input, _) = multispace0(input)?;
        let (input, expr) = parse_impl(env, input)?;
        let (input, _) = multispace0(input)?;
        let (input, _) = char(')')(input)?;
        let (input, _) = multispace0(input)?;
        Ok((input, expr))
    }

    fn abstraction_impl<'a>(env: &Vec<&'a str>, input: &'a str) -> IResult<&'a str, DeBruijn> {
        let (input, _) = alt((char('λ'), char('\\')))(input)?;
        let (input, _) = multispace0(input)?;

        let (mut input, _) = multispace0(input)?;
        let mut params = Vec::new();

        loop {
            match take_while1::<_, _, nom::error::Error<&str>>(|c: char| {
                c.is_alphanumeric() || c == '_'
            })(input)
            {
                Ok((new_input, param)) => {
                    params.push(param);
                    let (i, _) = multispace0(new_input)?;
                    input = i;
                }
                Err(_) => break,
            }
        }

        let (input, _) = char('.')(input)?;
        let (input, _) = multispace0(input)?;

        // 新しい環境を作成
        let mut new_env = env.clone();
        for param in &params {
            new_env.push(param);
        }

        let (input, body) = parse_impl(&new_env, input)?;

        // 複数のパラメータがある場合は右から順にネストする
        let result = params
            .iter()
            .rev()
            .fold(body, |acc, _| DeBruijn::Abs(Box::new(acc)));

        Ok((input, result))
    }

    let env = Vec::new();
    match parse_impl(&env, input) {
        Ok((remaining, expr)) => {
            let remaining = remaining.trim();
            if remaining.is_empty() {
                Ok(expr)
            } else {
                Err(format!(
                    "パース後に予期しない文字列が残っています: '{}'",
                    remaining
                ))
            }
        }
        Err(e) => Err(format!("パースエラー: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_variable() {
        // 自由変数 x -> Var(0) (環境が空なので)
        let result = parse("x").unwrap();
        assert_eq!(result, DeBruijn::Var(0));
    }

    #[test]
    fn test_parse_abstraction() {
        // λx.x -> λ.0
        let result = parse("\\x.x").unwrap();
        assert_eq!(result, DeBruijn::Abs(Rc::new(DeBruijn::Var(0))));

        // λx.y -> λ.1 (yは自由変数)
        let result = parse("λx.y").unwrap();
        assert_eq!(result, DeBruijn::Abs(Rc::new(DeBruijn::Var(1))));
    }

    #[test]
    fn test_parse_application() {
        // x y -> 0 0 (両方自由変数)
        let result = parse("x y").unwrap();
        assert_eq!(
            result,
            DeBruijn::App(Rc::new(DeBruijn::Var(0)), Rc::new(DeBruijn::Var(0)))
        );

        // a b c (左結合) -> (0 0) 0
        let result = parse("a b c").unwrap();
        assert_eq!(
            result,
            DeBruijn::App(
                Rc::new(DeBruijn::App(
                    Rc::new(DeBruijn::Var(0)),
                    Rc::new(DeBruijn::Var(0))
                )),
                Rc::new(DeBruijn::Var(0))
            )
        );
    }

    #[test]
    fn test_parse_complex() {
        // (λx.x) y -> (λ.0) 0
        let result = parse("(\\x.x) y").unwrap();
        assert_eq!(
            result,
            DeBruijn::App(
                Rc::new(DeBruijn::Abs(Rc::new(DeBruijn::Var(0)))),
                Rc::new(DeBruijn::Var(0))
            )
        );

        // λx.λy.x (K combinator) -> λ.λ.1
        let result = parse("\\x.\\y.x").unwrap();
        assert_eq!(
            result,
            DeBruijn::Abs(Rc::new(DeBruijn::Abs(Rc::new(DeBruijn::Var(1)))))
        );
    }

    #[test]
    fn test_parse_multi_param() {
        // λx y.x (複数パラメータ) -> λ.λ.1
        let result = parse("\\x y.x").unwrap();
        assert_eq!(
            result,
            DeBruijn::Abs(Rc::new(DeBruijn::Abs(Rc::new(DeBruijn::Var(1)))))
        );
    }

    #[test]
    fn test_parse_nested_app() {
        // (λx.x x) (λy.y) -> (λ.0 0) (λ.0)
        let result = parse("(\\x.x x) (\\y.y)").unwrap();
        let left = DeBruijn::Abs(Rc::new(DeBruijn::App(
            Rc::new(DeBruijn::Var(0)),
            Rc::new(DeBruijn::Var(0)),
        )));
        let right = DeBruijn::Abs(Rc::new(DeBruijn::Var(0)));
        assert_eq!(result, DeBruijn::App(Rc::new(left), Rc::new(right)));
    }

    #[test]
    fn test_parse_with_spaces() {
        // 空白を含む式 λx.x -> λ.0
        let result = parse("  \\x . x  ").unwrap();
        assert_eq!(result, DeBruijn::Abs(Rc::new(DeBruijn::Var(0))));
    }

    #[test]
    fn test_parse_s_combinator() {
        // S combinator: λx.λy.λz.(x z) (y z) -> λ.λ.λ.(2 0) (1 0)
        let result = parse("\\x.\\y.\\z.x z (y z)").unwrap();
        let expected = DeBruijn::Abs(Rc::new(DeBruijn::Abs(Rc::new(DeBruijn::Abs(Rc::new(
            DeBruijn::App(
                Rc::new(DeBruijn::App(
                    Rc::new(DeBruijn::Var(2)),
                    Rc::new(DeBruijn::Var(0)),
                )),
                Rc::new(DeBruijn::App(
                    Rc::new(DeBruijn::Var(1)),
                    Rc::new(DeBruijn::Var(0)),
                )),
            ),
        ))))));
        assert_eq!(result, expected);
    }
}
