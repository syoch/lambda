use crate::lambda::DeBruijn;
use crate::parser::parse_with_env;
use crate::search_combination::{search_combination, SearchOptions};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_till, take_until, take_while1},
    character::complete::{char, digit1, line_ending, multispace0, not_line_ending, space0},
    combinator::opt,
    multi::many0,
    sequence::{delimited, tuple},
    IResult,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 実行ファイルの位置から標準ライブラリパスを取得
/// 実行ファイルが A/bin/lambda の場合、A/lib/lambda を返す
fn get_lib_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;

    // bin ディレクトリの場合、lib/lambda に変更
    if exe_dir.file_name()? == "bin" {
        let base = exe_dir.parent()?;
        Some(base.join("lib").join("lambda"))
    } else {
        None
    }
}

/// ファイルパスを解決（相対パスまたは標準ライブラリパスから探す）
fn resolve_file_path(
    path: &str,
    base_path: Option<&Path>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // まず相対パスで試す
    let relative_path = if let Some(base) = base_path {
        base.join(path)
    } else {
        Path::new(path).to_path_buf()
    };

    if relative_path.exists() {
        return Ok(relative_path);
    }

    // 標準ライブラリパスで試す
    if let Some(lib_path) = get_lib_path() {
        let lib_file = lib_path.join(path);
        if lib_file.exists() {
            return Ok(lib_file);
        }
    }

    // どちらも見つからない場合は相対パスを返す（エラーは後で発生）
    Ok(relative_path)
}

/// スクリプトのステートメント
#[derive(Debug, Clone)]
pub enum Statement {
    /// コメント行
    Comment,
    /// 空行
    Empty,
    /// 変数定義: name = expr
    Definition { name: String, expr: String },
    /// アサーション: assert[(steps)]: left == right
    Assertion {
        steps: Option<usize>,
        left: String,
        right: String,
    },
    /// 簡約ステップ表示: reduce_steps expr
    ReduceSteps { expr: String, steps: Option<usize> },
    /// 組み合わせ探索: search(n, steps) base_expr -> target_expr
    Search {
        base_expr: String,
        target_expr: String,
        max_n: Option<usize>,
        max_steps: Option<usize>,
    },
    /// ファイルインクルード: include "path" [as namespace]
    Include {
        path: String,
        namespace: Option<String>,
    },
    /// 選択的インポート: from "path" import name1, name2, ...
    FromImport { path: String, names: Vec<String> },
}

/// スクリプト全体をパースする
pub fn parse_script(input: &str) -> Result<Vec<Statement>, Box<dyn std::error::Error>> {
    match script_parser(input) {
        Ok((remaining, statements)) => {
            if !remaining.trim().is_empty() {
                eprintln!("Warning: Unparsed content: {:?}", remaining);
            }
            Ok(statements)
        }
        Err(e) => Err(format!("Script parse error: {}", e).into()),
    }
}

/// スクリプトファイルを実行する
pub fn run_script(
    file: &Path,
    max_steps: usize,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut env: HashMap<String, DeBruijn> = HashMap::new();
    let mut test_count = 0;
    let mut passed_count = 0;

    println!("=== Lambda Script Executor ===");
    println!("File: {}", file.display());
    println!();

    process_file(
        file,
        file.parent(),
        &mut env,
        &mut test_count,
        &mut passed_count,
        max_steps,
        verbose,
    )?;

    println!("\n=== Test Results ===");
    println!(
        "Total: {}, Passed: {}, Failed: {}",
        test_count,
        passed_count,
        test_count - passed_count
    );

    if passed_count == test_count {
        println!("✓ All tests passed!");
        Ok(())
    } else {
        Err(format!("{} test(s) failed", test_count - passed_count).into())
    }
}

/// ファイルを処理する（再帰的に呼ばれる）
fn process_file(
    file: &Path,
    base_path: Option<&Path>,
    env: &mut HashMap<String, DeBruijn>,
    test_count: &mut usize,
    passed_count: &mut usize,
    max_steps: usize,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(file)?;

    // スクリプトをパース
    let statements = parse_script(&content)?;

    for statement in statements {
        match statement {
            Statement::Comment | Statement::Empty => {
                // スキップ
            }
            Statement::Include { path, namespace } => {
                // ファイルパスを解決（相対パスまたは標準ライブラリパス）
                let include_path = resolve_file_path(&path, base_path)?;

                if verbose {
                    println!("Include: {} (as {:?})", include_path.display(), namespace);
                }

                // 別環境でファイルを処理
                let mut included_env = HashMap::new();
                let mut dummy_test_count = 0;
                let mut dummy_passed_count = 0;

                process_file(
                    &include_path,
                    include_path.parent(),
                    &mut included_env,
                    &mut dummy_test_count,
                    &mut dummy_passed_count,
                    max_steps,
                    verbose,
                )?;

                // 名前空間付きまたは直接環境に追加
                if let Some(ns) = namespace {
                    // 名前空間付き: namespace.name としてアクセス
                    for (name, expr) in included_env {
                        env.insert(format!("{}.{}", ns, name), expr);
                    }
                } else {
                    // 名前空間なし: 直接マージ
                    env.extend(included_env);
                }
            }
            Statement::FromImport { path, names } => {
                // ファイルパスを解決（相対パスまたは標準ライブラリパス）
                let include_path = resolve_file_path(&path, base_path)?;

                if verbose {
                    println!("From {:?} import {:?}", include_path.display(), names);
                }

                // 別環境でファイルを処理
                let mut included_env = HashMap::new();
                let mut dummy_test_count = 0;
                let mut dummy_passed_count = 0;

                process_file(
                    &include_path,
                    include_path.parent(),
                    &mut included_env,
                    &mut dummy_test_count,
                    &mut dummy_passed_count,
                    max_steps,
                    verbose,
                )?;

                // 指定された名前のみをインポート
                for name in names {
                    if let Some(expr) = included_env.get(&name) {
                        env.insert(name, expr.clone());
                    } else {
                        return Err(format!(
                            "Name '{}' not found in {}",
                            name,
                            include_path.display()
                        )
                        .into());
                    }
                }
            }
            Statement::Definition { name, expr } => {
                let parsed_expr = parse_with_env(&expr, &env)?;

                if verbose {
                    println!("Define: {} = {}", name, parsed_expr);
                }

                env.insert(name, parsed_expr);
            }
            Statement::Assertion { steps, left, right } => {
                *test_count += 1;

                // 環境内の変数を考慮してパース
                let left_expr = parse_with_env(&left, &env)?;
                let right_expr = parse_with_env(&right, &env)?;

                if verbose {
                    println!("Test {}: {} == {}", test_count, left, right);
                    println!("  Left expr:  {}", left_expr);
                    println!("  Right expr: {}", right_expr);
                }

                let steps = steps.unwrap_or(max_steps);
                let left_normal = left_expr.normalize(steps);
                let right_normal = right_expr.normalize(steps);

                if verbose {
                    println!("  Left normal:  {}", left_normal);
                    println!("  Right normal: {}", right_normal);
                }

                if left_normal == right_normal {
                    println!("✓ Test {}: PASS", test_count);
                    *passed_count += 1;
                } else {
                    println!("✗ Test {}: FAIL", test_count);
                    println!("  Expected: {}", right_normal);
                    println!("  Got:      {}", left_normal);
                }

                if verbose {
                    println!();
                }
            }
            Statement::ReduceSteps { expr, steps } => {
                let parsed_expr = parse_with_env(&expr, &env)?;
                let max = steps.unwrap_or(max_steps);

                // println!("\n=== Reduction Steps ===");
                // println!("Expression: {}", expr);
                // println!();

                let mut current = parsed_expr;

                if max == 0 {
                    let normalized = current.normalize(max_steps);
                    println!("Normalized: {}", normalized);
                } else {
                    println!("Step 0: {}", current);
                    for step in 1..=max {
                        if let Some(next) = current.beta_reduce_step() {
                            println!("Step {}: {}", step, next);
                            current = next;
                        } else {
                            println!("\nReached normal form at step {}.", step - 1);
                            break;
                        }
                    }
                }

                println!();
            }
            Statement::Search {
                base_expr,
                target_expr,
                max_n,
                max_steps: search_steps,
            } => {
                let base = parse_with_env(&base_expr, &env)?;
                let target = parse_with_env(&target_expr, &env)?;
                let n = max_n.unwrap_or(10);
                let steps = search_steps.unwrap_or(400);

                println!("\n=== Combination Search ===");
                println!("Base expression: {}", base_expr);
                println!("Target expression: {}", target_expr);
                println!("Max combinations: {}", n);
                println!("Max steps: {}", steps);
                println!();

                let options = SearchOptions {
                    show_progress: false,
                    verbose: false,
                };

                if let Some(result) = search_combination(&base, &target, n, steps, &options) {
                    println!("✓ Found solution with {} applications:", result.n);
                    println!("  Combination: {}", result.combination);
                    println!("  Expression:  {}", result.expr);
                } else {
                    println!("✗ No solution found up to {} combinations.", n);
                }

                println!();
            }
        }
    }

    Ok(())
}

/// スクリプトパーサー
fn script_parser(input: &str) -> IResult<&str, Vec<Statement>> {
    let (input, statements) = many0(statement_parser)(input)?;
    let (input, _) = multispace0(input)?; // 末尾の空白を許容
    Ok((input, statements))
}

/// ステートメントパーサー
fn statement_parser(input: &str) -> IResult<&str, Statement> {
    alt((
        empty_line_parser,
        comment_parser,
        from_import_parser,
        include_parser,
        reduce_steps_parser,
        search_parser,
        assertion_parser,
        definition_parser_combinator,
    ))(input)
}

/// 空行パーサー（改行のみの行、または空白のみの行）
fn empty_line_parser(input: &str) -> IResult<&str, Statement> {
    use nom::bytes::complete::is_a;
    use nom::combinator::recognize;

    let (input, _) = recognize(tuple((
        opt(is_a(" \t")), // オプションの水平空白のみ
        line_ending,
    )))(input)?;

    Ok((input, Statement::Empty))
}

/// コメント行パーサー
fn comment_parser(input: &str) -> IResult<&str, Statement> {
    let (input, _) = multispace0(input)?;
    let (input, _) = char('#')(input)?;
    let (input, _) = not_line_ending(input)?;
    let (input, _) = line_ending(input)?;
    Ok((input, Statement::Comment))
}

/// includeパーサー: include "path" [as namespace]
fn include_parser(input: &str) -> IResult<&str, Statement> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("include")(input)?;
    let (input, _) = space0(input)?; // 水平空白のみ

    // パス（ダブルクォートで囲まれた文字列）
    let (input, _) = char('"')(input)?;
    let (input, path) = take_till(|c| c == '"')(input)?;
    let (input, _) = char('"')(input)?;

    // オプションの "as namespace"
    let (input, namespace) = opt(tuple((
        space0, // 水平空白のみ
        tag("as"),
        space0, // 水平空白のみ
        take_while1(|c: char| c.is_alphanumeric() || c == '_'),
    )))(input)?;

    let namespace = namespace.map(|(_, _, _, ns)| ns.to_string());

    let (input, _) = space0(input)?; // 水平空白のみ
    let (input, _) = line_ending(input)?;

    Ok((
        input,
        Statement::Include {
            path: path.to_string(),
            namespace,
        },
    ))
}

/// from-importパーサー: from "path" import name1, name2, ...
fn from_import_parser(input: &str) -> IResult<&str, Statement> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("from")(input)?;
    let (input, _) = space0(input)?;

    // パス
    let (input, _) = char('"')(input)?;
    let (input, path) = take_till(|c| c == '"')(input)?;
    let (input, _) = char('"')(input)?;

    let (input, _) = space0(input)?;
    let (input, _) = tag("import")(input)?;
    let (input, _) = space0(input)?;

    // 名前のリスト（カンマ区切り）
    let (input, first_name) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;
    let mut names = vec![first_name.to_string()];

    let (input, rest_names) = many0(tuple((
        space0,
        char(','),
        space0,
        take_while1(|c: char| c.is_alphanumeric() || c == '_'),
    )))(input)?;

    for (_, _, _, name) in rest_names {
        names.push(name.to_string());
    }

    let (input, _) = space0(input)?;
    let (input, _) = line_ending(input)?;

    Ok((
        input,
        Statement::FromImport {
            path: path.to_string(),
            names,
        },
    ))
}

/// 組み合わせ探索パーサー
/// search[(max_n, max_steps)] <base_expr> -> <target_expr>
fn search_parser(input: &str) -> IResult<&str, Statement> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("search")(input)?;

    // オプションのパラメータ (max_n, max_steps)
    let (input, params) = opt(delimited(
        tuple((multispace0, char('('))),
        tuple((
            digit1,
            opt(tuple((multispace0, char(','), multispace0, digit1))),
        )),
        tuple((char(')'), multispace0)),
    ))(input)?;

    let (max_n, max_steps) = if let Some((n, steps_opt)) = params {
        let n_val = n.parse().ok();
        let steps_val = steps_opt.and_then(|(_, _, _, s)| s.parse().ok());
        (n_val, steps_val)
    } else {
        (None, None)
    };

    let (input, _) = multispace0(input)?;

    // base_expr ("->" まで)
    let (input, base_expr) = take_until("->")(input)?;
    let (input, _) = tag("->")(input)?;
    let (input, _) = multispace0(input)?;

    // target_expr (行末まで)
    let (input, target_expr) = not_line_ending(input)?;
    let (input, _) = line_ending(input)?;

    Ok((
        input,
        Statement::Search {
            base_expr: base_expr.trim().to_string(),
            target_expr: target_expr.trim().to_string(),
            max_n,
            max_steps,
        },
    ))
}

/// 簡約ステップ表示パーサー
fn reduce_steps_parser(input: &str) -> IResult<&str, Statement> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("reduce_steps")(input)?;

    // オプションのステップ数
    let (input, steps) = opt(delimited(
        tuple((multispace0, char('('))),
        digit1,
        tuple((char(')'), multispace0)),
    ))(input)?;

    let (input, _) = multispace0(input)?;

    // 式（行末まで）
    let (input, expr) = not_line_ending(input)?;
    let (input, _) = line_ending(input)?;

    Ok((
        input,
        Statement::ReduceSteps {
            steps: steps.and_then(|s| s.parse().ok()),
            expr: expr.trim().to_string(),
        },
    ))
}

/// アサーションパーサー
fn assertion_parser(input: &str) -> IResult<&str, Statement> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("assert")(input)?;

    // オプションのステップ数
    let (input, steps) = opt(delimited(
        tuple((multispace0, char('('))),
        digit1,
        tuple((char(')'), multispace0)),
    ))(input)?;

    // オプションのコロン
    let (input, _) = opt(tuple((multispace0, char(':'), multispace0)))(input)?;
    let (input, _) = multispace0(input)?;

    // 左辺（"=="まで）
    let (input, left) = take_until("==")(input)?;
    let (input, _) = tag("==")(input)?;
    let (input, _) = multispace0(input)?;

    // 右辺（行末まで）
    let (input, right) = not_line_ending(input)?;
    let (input, _) = line_ending(input)?;

    Ok((
        input,
        Statement::Assertion {
            steps: steps.and_then(|s| s.parse().ok()),
            left: left.trim().to_string(),
            right: right.trim().to_string(),
        },
    ))
}

/// 変数定義パーサー
fn definition_parser_combinator(input: &str) -> IResult<&str, Statement> {
    let (input, _) = multispace0(input)?;

    // 識別子
    let (input, name) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('=')(input)?;
    let (input, _) = multispace0(input)?;

    // 式（行末まで）
    let (input, expr) = not_line_ending(input)?;
    let (input, _) = line_ending(input)?;

    Ok((
        input,
        Statement::Definition {
            name: name.to_string(),
            expr: expr.trim().to_string(),
        },
    ))
}
