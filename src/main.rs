use clap::{Parser, Subcommand};
use lambda::parser::parse;
use lambda::script::run_script;
use lambda::search_combination::{search_combination, SearchOptions};
use std::path::PathBuf;

/// Lambda Calculus CLI - ラムダ計算の簡約化と組み合わせ探索ツール
#[derive(Parser)]
#[command(name = "lambda")]
#[command(version = "0.1.0")]
#[command(about = "ラムダ計算の簡約化と組み合わせ探索を行うCLIツール", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// ラムダ式を簡約化（正規形に変換）
    Reduce {
        /// 簡約化するラムダ式（例: "\\x.x" は恒等関数）
        #[arg(value_name = "EXPRESSION")]
        expression: String,

        /// 最大簡約ステップ数
        #[arg(short, long, default_value = "1000")]
        max_steps: usize,

        /// 簡約の各ステップを表示
        #[arg(short, long)]
        verbose: bool,
    },
    /// 式Xを組み合わせて目標の式Yを構成できるか探索
    Search {
        /// 組み合わせる基本式 X
        #[arg(short = 'x', long, value_name = "EXPRESSION")]
        base_expr: String,

        /// 目標となる式 Y
        #[arg(short = 'y', long, value_name = "EXPRESSION")]
        target_expr: String,

        /// 最大組み合わせ数
        #[arg(short = 'n', long, default_value = "10")]
        max_n: usize,

        /// 正規化の最大ステップ数
        #[arg(short = 's', long, default_value = "400")]
        max_steps: usize,

        /// プログレスバーを表示しない
        #[arg(short = 'q', long)]
        quiet: bool,

        /// 詳細ログを表示
        #[arg(short, long)]
        verbose: bool,
    },
    /// .lambdaスクリプトファイルを実行
    Run {
        /// 実行する.lambdaファイルのパス
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// 最大簡約ステップ数
        #[arg(short, long, default_value = "1000")]
        max_steps: usize,

        /// 詳細ログを表示
        #[arg(short, long)]
        verbose: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Reduce {
            expression,
            max_steps,
            verbose,
        } => {
            let expr = parse(&expression)?;

            println!("入力式: {}", expr);
            println!("ノード数: {}", expr.node_count());
            println!();

            if verbose {
                let mut current = expr.clone();
                println!("簡約過程:");
                println!("  Step 0: {}", current);

                for step in 1..=max_steps {
                    if let Some(next) = current.beta_reduce_step() {
                        println!("  Step {}: {}", step, next);
                        current = next;
                    } else {
                        println!("\nステップ {} で正規形に到達しました。", step - 1);
                        break;
                    }
                }

                if current.beta_reduce_step().is_some() {
                    println!("\n最大ステップ数 {} に到達しました。", max_steps);
                }
            } else {
                let normalized = expr.clone().normalize(max_steps);
                println!("正規形: {}", normalized);
                println!("ノード数: {}", normalized.node_count());
            }
        }
        Commands::Search {
            base_expr,
            target_expr,
            max_n,
            max_steps,
            quiet,
            verbose,
        } => {
            let x = parse(&base_expr)?;
            let y = parse(&target_expr)?;

            let options = if quiet {
                SearchOptions::quiet()
            } else if verbose {
                SearchOptions::default()
            } else {
                SearchOptions::progress_only()
            };

            println!("=== 組み合わせ探索 ===");
            println!("基本式 X: {}", x);
            println!("目標式 Y: {}", y);
            println!("最大組み合わせ数: {}", max_n);
            println!("正規化最大ステップ: {}", max_steps);
            println!();

            match search_combination(&x, &y, max_n, max_steps, &options) {
                Some(result) => {
                    println!("\n========================================");
                    println!("✓ 成功: {} 個の X で Y を構成できました", result.n);
                    println!("========================================");
                    println!("\n構成方法: {}", result.combination);
                    println!("構成された式: {}", result.expr);
                }
                None => {
                    println!("\n========================================");
                    println!(
                        "✗ 失敗: {} 個までの組み合わせでは見つかりませんでした",
                        max_n
                    );
                    println!("========================================");
                }
            }
        }
        Commands::Run {
            file,
            max_steps,
            verbose,
        } => {
            run_script(&file, max_steps, verbose)?;
        }
    }

    Ok(())
}
