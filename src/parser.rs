use crate::lambda::DeBruijn;
use nom::{
    branch::alt,
    bytes::complete::take_while1,
    character::complete::{char, multispace0},
    IResult,
};
use std::collections::HashMap;

/// 文字列からラムダ式をパースする公開関数
pub fn parse(input: &str) -> Result<DeBruijn, String> {
    use std::cell::RefCell;
    let errors = RefCell::new(Vec::new());

    fn parse_impl<'a>(
        env: &Vec<&'a str>,
        input: &'a str,
        errors: &RefCell<Vec<String>>,
    ) -> IResult<&'a str, DeBruijn> {
        application_impl(env, input, errors)
    }

    fn application_impl<'a>(
        env: &Vec<&'a str>,
        input: &'a str,
        errors: &RefCell<Vec<String>>,
    ) -> IResult<&'a str, DeBruijn> {
        let (input, first) = term_impl(env, input, errors)?;
        let (mut input, _) = multispace0(input)?;

        let mut result = first;
        loop {
            match term_impl(env, input, errors) {
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

    fn term_impl<'a>(
        env: &Vec<&'a str>,
        input: &'a str,
        errors: &RefCell<Vec<String>>,
    ) -> IResult<&'a str, DeBruijn> {
        let (input, _) = multispace0(input)?;
        alt((
            |i| abstraction_impl(env, i, errors),
            |i| memo_impl(env, i, errors),
            |i| parens_impl(env, i, errors),
            |i| variable_impl(env, i, errors),
        ))(input)
    }

    fn variable_impl<'a>(
        env: &Vec<&'a str>,
        input: &'a str,
        errors: &RefCell<Vec<String>>,
    ) -> IResult<&'a str, DeBruijn> {
        let (input, var_name) =
            take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '.')(input)?;
        let (input, _) = multispace0(input)?;

        // 環境から変数を探す（内側から外側へ）
        for (i, bound_name) in env.iter().rev().enumerate() {
            if *bound_name == var_name {
                return Ok((input, DeBruijn::Var(i)));
            }
        }

        // 未定義変数の場合
        errors.borrow_mut().push(var_name.to_string());
        // ダミーとして自由変数を返す
        Ok((input, DeBruijn::Var(env.len())))
    }

    fn memo_impl<'a>(
        env: &Vec<&'a str>,
        input: &'a str,
        errors: &RefCell<Vec<String>>,
    ) -> IResult<&'a str, DeBruijn> {
        let (input, _) = char('[')(input)?;
        let (input, _) = multispace0(input)?;
        let (input, expr) = parse_impl(env, input, errors)?;
        let (input, _) = multispace0(input)?;
        let (input, _) = char(']')(input)?;
        let (input, _) = multispace0(input)?;
        Ok((input, DeBruijn::Memo(Box::new(expr))))
    }

    fn parens_impl<'a>(
        env: &Vec<&'a str>,
        input: &'a str,
        errors: &RefCell<Vec<String>>,
    ) -> IResult<&'a str, DeBruijn> {
        let (input, _) = char('(')(input)?;
        let (input, _) = multispace0(input)?;
        let (input, expr) = parse_impl(env, input, errors)?;
        let (input, _) = multispace0(input)?;
        let (input, _) = char(')')(input)?;
        let (input, _) = multispace0(input)?;
        Ok((input, expr))
    }

    fn abstraction_impl<'a>(
        env: &Vec<&'a str>,
        input: &'a str,
        errors: &RefCell<Vec<String>>,
    ) -> IResult<&'a str, DeBruijn> {
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

        let (input, body) = parse_impl(&new_env, input, errors)?;

        // 複数のパラメータがある場合は右から順にネストする
        let result = params
            .iter()
            .rev()
            .fold(body, |acc, _| DeBruijn::Abs(Box::new(acc)));

        Ok((input, result))
    }

    let env = Vec::new();
    match parse_impl(&env, input, &errors) {
        Ok((remaining, expr)) => {
            let errors_list = errors.into_inner();
            if !errors_list.is_empty() {
                return Err(format!("未定義の変数です: {}", errors_list.join(", ")));
            }

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

/// 環境内の変数を考慮してパースする
/// 環境内の変数は自由変数として扱われ、後で実際の式に置換される
pub fn parse_with_env(
    input: &str,
    env_map: &HashMap<String, DeBruijn>,
) -> Result<DeBruijn, String> {
    use std::cell::RefCell;
    let errors = RefCell::new(Vec::new());

    // 環境変数をプレースホルダーとして扱う
    // 各環境変数に対して、自由変数としてインデックスを割り当てる
    // パース後、それらのインデックスを実際の式に置換する

    fn parse_impl_with_env<'a>(
        bound_vars: &Vec<&'a str>,
        env_map: &HashMap<String, DeBruijn>,
        env_order: &Vec<String>,
        input: &'a str,
        errors: &RefCell<Vec<String>>,
    ) -> IResult<&'a str, DeBruijn> {
        application_impl_env(bound_vars, env_map, env_order, input, errors)
    }

    fn application_impl_env<'a>(
        bound_vars: &Vec<&'a str>,
        env_map: &HashMap<String, DeBruijn>,
        env_order: &Vec<String>,
        input: &'a str,
        errors: &RefCell<Vec<String>>,
    ) -> IResult<&'a str, DeBruijn> {
        let (input, first) = term_impl_env(bound_vars, env_map, env_order, input, errors)?;
        let (mut input, _) = multispace0(input)?;

        let mut result = first;
        loop {
            match term_impl_env(bound_vars, env_map, env_order, input, errors) {
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

    fn term_impl_env<'a>(
        bound_vars: &Vec<&'a str>,
        env_map: &HashMap<String, DeBruijn>,
        env_order: &Vec<String>,
        input: &'a str,
        errors: &RefCell<Vec<String>>,
    ) -> IResult<&'a str, DeBruijn> {
        let (input, _) = multispace0(input)?;
        alt((
            |i| abstraction_impl_env(bound_vars, env_map, env_order, i, errors),
            |i| memo_impl_env(bound_vars, env_map, env_order, i, errors),
            |i| parens_impl_env(bound_vars, env_map, env_order, i, errors),
            |i| variable_impl_env(bound_vars, env_map, env_order, i, errors),
        ))(input)
    }

    fn variable_impl_env<'a>(
        bound_vars: &Vec<&'a str>,
        env_map: &HashMap<String, DeBruijn>,
        env_order: &Vec<String>,
        input: &'a str,
        errors: &RefCell<Vec<String>>,
    ) -> IResult<&'a str, DeBruijn> {
        let (input, var_name) =
            take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '.')(input)?;
        let (input, _) = multispace0(input)?;

        // まず束縛変数から探す（内側から外側へ）
        for (i, bound_name) in bound_vars.iter().rev().enumerate() {
            if *bound_name == var_name {
                return Ok((input, DeBruijn::Var(i)));
            }
        }

        // 環境変数から探す
        if let Some(expr) = env_map.get(var_name) {
            // 環境の式を返す（シフトが必要な場合もある）
            let shifted = expr.shift(bound_vars.len() as isize, 0);
            return Ok((input, shifted));
        }

        // 未定義変数の場合
        errors.borrow_mut().push(var_name.to_string());
        // ダミーとして自由変数を返す
        Ok((input, DeBruijn::Var(bound_vars.len() + env_order.len())))
    }

    fn memo_impl_env<'a>(
        bound_vars: &Vec<&'a str>,
        env_map: &HashMap<String, DeBruijn>,
        env_order: &Vec<String>,
        input: &'a str,
        errors: &RefCell<Vec<String>>,
    ) -> IResult<&'a str, DeBruijn> {
        let (input, _) = char('[')(input)?;
        let (input, _) = multispace0(input)?;
        let (input, expr) = parse_impl_with_env(bound_vars, env_map, env_order, input, errors)?;
        let (input, _) = multispace0(input)?;
        let (input, _) = char(']')(input)?;
        let (input, _) = multispace0(input)?;
        Ok((input, DeBruijn::Memo(Box::new(expr))))
    }

    fn parens_impl_env<'a>(
        bound_vars: &Vec<&'a str>,
        env_map: &HashMap<String, DeBruijn>,
        env_order: &Vec<String>,
        input: &'a str,
        errors: &RefCell<Vec<String>>,
    ) -> IResult<&'a str, DeBruijn> {
        let (input, _) = char('(')(input)?;
        let (input, _) = multispace0(input)?;
        let (input, expr) = parse_impl_with_env(bound_vars, env_map, env_order, input, errors)?;
        let (input, _) = multispace0(input)?;
        let (input, _) = char(')')(input)?;
        let (input, _) = multispace0(input)?;
        Ok((input, expr))
    }

    fn abstraction_impl_env<'a>(
        bound_vars: &Vec<&'a str>,
        env_map: &HashMap<String, DeBruijn>,
        env_order: &Vec<String>,
        input: &'a str,
        errors: &RefCell<Vec<String>>,
    ) -> IResult<&'a str, DeBruijn> {
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
        let mut new_bound_vars = bound_vars.clone();
        for param in &params {
            new_bound_vars.push(param);
        }

        let (input, body) =
            parse_impl_with_env(&new_bound_vars, env_map, env_order, input, errors)?;

        // 複数のパラメータがある場合は右から順にネストする
        let result = params
            .iter()
            .rev()
            .fold(body, |acc, _| DeBruijn::Abs(Box::new(acc)));

        Ok((input, result))
    }

    let bound_vars = Vec::new();
    let env_order: Vec<String> = env_map.keys().cloned().collect();

    match parse_impl_with_env(&bound_vars, env_map, &env_order, input, &errors) {
        Ok((remaining, expr)) => {
            let errors_list = errors.into_inner();
            if !errors_list.is_empty() {
                // 重複を排除
                let mut unique_errors: Vec<String> = errors_list.into_iter().collect();
                unique_errors.sort();
                unique_errors.dedup();
                return Err(format!("未定義の変数です: {}", unique_errors.join(", ")));
            }

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
        // 未定義変数 x はエラーになる
        let result = parse("x");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "未定義の変数です: x");
    }

    #[test]
    fn test_parse_abstraction() {
        // λx.x -> λ.0
        let result = parse("\\x.x").unwrap();
        assert_eq!(result, DeBruijn::Abs(Box::new(DeBruijn::Var(0))));

        // λx.y -> yは未定義変数
        let result = parse("λx.y");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "未定義の変数です: y");
    }

    #[test]
    fn test_parse_application() {
        // x y -> 両方未定義変数
        let result = parse("x y");
        assert!(result.is_err());

        // a b c (左結合) -> 未定義変数
        let result = parse("a b c");
        assert!(result.is_err());

        // 正しいケース: \x.\y.\z. x y z
        let result = parse("\\x.\\y.\\z. x y z").unwrap();
        assert_eq!(
            result,
            DeBruijn::Abs(Box::new(DeBruijn::Abs(Box::new(DeBruijn::Abs(Box::new(
                DeBruijn::App(
                    Box::new(DeBruijn::App(
                        Box::new(DeBruijn::Var(2)), // x
                        Box::new(DeBruijn::Var(1))  // y
                    )),
                    Box::new(DeBruijn::Var(0)) // z
                )
            ))))))
        );
    }

    #[test]
    fn test_parse_complex() {
        // (λx.x) y -> yが未定義変数
        let result = parse("(\\x.x) y");
        assert!(result.is_err());

        // λx.λy.x (K combinator) -> λ.λ.1
        let result = parse("\\x.\\y.x").unwrap();
        assert_eq!(
            result,
            DeBruijn::Abs(Box::new(DeBruijn::Abs(Box::new(DeBruijn::Var(1)))))
        );
    }

    #[test]
    fn test_parse_multi_param() {
        // λx y.x (複数パラメータ) -> λ.λ.1
        let result = parse("\\x y.x").unwrap();
        assert_eq!(
            result,
            DeBruijn::Abs(Box::new(DeBruijn::Abs(Box::new(DeBruijn::Var(1)))))
        );
    }

    #[test]
    fn test_parse_nested_app() {
        // (λx.x x) (λy.y) -> (λ.0 0) (λ.0)
        let result = parse("(\\x.x x) (\\y.y)").unwrap();
        let left = DeBruijn::Abs(Box::new(DeBruijn::App(
            Box::new(DeBruijn::Var(0)),
            Box::new(DeBruijn::Var(0)),
        )));
        let right = DeBruijn::Abs(Box::new(DeBruijn::Var(0)));
        assert_eq!(result, DeBruijn::App(Box::new(left), Box::new(right)));
    }

    #[test]
    fn test_parse_with_spaces() {
        // 空白を含む式 λx.x -> λ.0
        let result = parse("  \\x . x  ").unwrap();
        assert_eq!(result, DeBruijn::Abs(Box::new(DeBruijn::Var(0))));
    }

    #[test]
    fn test_parse_s_combinator() {
        // S combinator: λx.λy.λz.(x z) (y z) -> λ.λ.λ.(2 0) (1 0)
        let result = parse("\\x.\\y.\\z.x z (y z)").unwrap();
        let expected = DeBruijn::Abs(Box::new(DeBruijn::Abs(Box::new(DeBruijn::Abs(Box::new(
            DeBruijn::App(
                Box::new(DeBruijn::App(
                    Box::new(DeBruijn::Var(2)),
                    Box::new(DeBruijn::Var(0)),
                )),
                Box::new(DeBruijn::App(
                    Box::new(DeBruijn::Var(1)),
                    Box::new(DeBruijn::Var(0)),
                )),
            ),
        ))))));
        assert_eq!(result, expected);
    }
}
