use crate::lambda::DeBruijn;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashSet;
use std::time::Instant;
use std::fmt;

/// 組み合わせ構造を表す型
#[derive(Debug, Clone)]
pub enum Combination {
    /// 基本要素 X
    Base,
    /// 関数適用: App(func, arg)
    App(Box<Combination>, Box<Combination>),
}

impl fmt::Display for Combination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Combination::Base => write!(f, "X"),
            Combination::App(func, arg) => {
                let func_str = match func.as_ref() {
                    Combination::Base => format!("{}", func),
                    Combination::App(_, _) => format!("{}", func),
                };
                let arg_str = match arg.as_ref() {
                    Combination::Base => format!("{}", arg),
                    Combination::App(_, _) => format!("({})", arg),
                };
                write!(f, "{}{}", func_str, arg_str)
            }
        }
    }
}

/// 探索結果
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// 構成された式（de Bruijn表現）
    pub expr: DeBruijn,
    /// 使用したXの個数
    pub n: usize,
    /// 組み合わせ構造
    pub combination: Combination,
}

/// 探索オプション
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// プログレスバーを表示するか
    pub show_progress: bool,
    /// 詳細ログを出力するか
    pub verbose: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            show_progress: true,
            verbose: true,
        }
    }
}

impl SearchOptions {
    /// プログレスバーなし、ログなしの静かなモード
    pub fn quiet() -> Self {
        Self {
            show_progress: false,
            verbose: false,
        }
    }

    /// プログレスバーあり、ログなしのモード
    #[allow(dead_code)]
    pub fn progress_only() -> Self {
        Self {
            show_progress: true,
            verbose: false,
        }
    }
}

/// de Bruijn式と組み合わせ構造のペア（標準形付き）
#[derive(Clone)]
struct DeBruijnWithCombination {
    /// 元の式（de Bruijn表現）
    expr_db: DeBruijn,
    /// 組み合わせ構造
    combination: Combination,
}

/// X を最大 n 個組み合わせて Y が構成できるかを探索（全通りの組み合わせを試す版）
///
/// # 引数
///
/// * `x` - 組み合わせる基本式
/// * `y` - 目標となる式
/// * `max_n` - 最大組み合わせ数
/// * `max_reduce_steps` - 標準形への変換の最大ステップ数
/// * `options` - 探索オプション（プログレスバー、ログ出力の制御）
///
/// # 戻り値
///
/// 成功時: `Some(SearchResult)`
/// 失敗時: `None`
pub fn search_combination(
    x: &DeBruijn,
    y: &DeBruijn,
    max_n: usize,
    max_reduce_steps: usize,
    options: &SearchOptions,
) -> Option<SearchResult> {
    let start_time = Instant::now();
    eprintln!("[+0.000s] search_combination 開始: max_n={}, max_reduce_steps={}", max_n, max_reduce_steps);
    
    if options.verbose {
        println!(
            "探索開始: X を最大 {} 個まで組み合わせて Y を構成できるか探索\n",
            max_n
        );
        println!("X = {}", x);
        println!("Y = {}\n", y);
    }

    // de Bruijnで標準形を計算
    let norm_start = Instant::now();
    let x_normal_db = x.clone().normalize(max_reduce_steps);
    let y_normal_db = y.clone().normalize(max_reduce_steps);
    eprintln!("[+{:.3}s] 初期正規化完了: {:.3}秒", 
             start_time.elapsed().as_secs_f64(), norm_start.elapsed().as_secs_f64());

    // n=1: X 自体をチェック
    if x_normal_db == y_normal_db {
        let result = SearchResult {
            expr: x.clone(),
            n: 1,
            combination: Combination::Base,
        };
        if options.verbose {
            println!("✓ 発見! n=1 で Y を構成できました");
        }
        return Some(result);
    }

    if options.verbose {
        println!("n=1: 1 個の組み合わせをチェック - 見つかりませんでした");
    }

    // すべての世代の組み合わせを保持: all_generations[i] は n=i+1 の組み合わせ
    let mut all_generations: Vec<Vec<DeBruijnWithCombination>> =
        vec![vec![DeBruijnWithCombination {
            expr_db: x.clone(),
            combination: Combination::Base,
        }]];

    // 重複チェック用のセット（de Bruijn表現を使用）
    let mut seen = HashSet::new();
    seen.insert(x_normal_db.clone());

    for n in 2..=max_n {
        let n_start = Instant::now();
        eprintln!("[+{:.3}s] n={} 開始", start_time.elapsed().as_secs_f64(), n);
        
        // n1 + n2 = n となる組み合わせを探索（i + j = n の直積空間）
        // 全ペア数を計算してプログレスバーを準備
        let mut total_pairs = 0;
        for n1 in 1..n {
            let n2 = n - n1;
            if n1 <= all_generations.len() && n2 <= all_generations.len() {
                total_pairs += all_generations[n1 - 1].len() * all_generations[n2 - 1].len();
            }
        }

        let gen_time = n_start.elapsed();
        eprintln!("[+{:.3}s] n={} 組み合わせ生成完了: {} pairs, {:.3}秒経過", 
                 start_time.elapsed().as_secs_f64(), n, total_pairs, gen_time.as_secs_f64());
        
        let pb = if options.show_progress {
            let pb = ProgressBar::new(total_pairs as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
                    .unwrap()
                    .progress_chars("##-"),
            );
            pb.set_message(format!("n={} の組み合わせを生成・チェック中", n));
            Some(pb)
        } else {
            None
        };

        let mut new_combinations = Vec::new();
        let mut checked_count = 0;
        let mut normalize_total_time = 0.0;
        let mut normalize_count = 0;

        // i + j = n を満たす (i, j) の直積空間を探索
        for n1 in 1..n {
            let n2 = n - n1;
            if n1 > all_generations.len() || n2 > all_generations.len() {
                continue;
            }

            let gen1 = &all_generations[n1 - 1];
            let gen2 = &all_generations[n2 - 1];

            if options.verbose {
                println!(
                    "  n1={}, n2={}: {} x {} = {} 組み合わせをチェック中...",
                    n1,
                    n2,
                    gen1.len(),
                    gen2.len(),
                    gen1.len() * gen2.len()
                );
            }

            for db1_with_comb in gen1 {
                for db2_with_comb in gen2 {
                    let pair_start = Instant::now();
                    
                    // n=14で各ペア開始を記録
                    if n == 14 {
                        eprintln!(
                            "[+{:.3}s] n=14, pair {}/{} starting: n1={}, n2={}",
                            start_time.elapsed().as_secs_f64(),
                            checked_count + 1,
                            total_pairs,
                            n1,
                            n2
                        );
                    }
                    
                    // de Bruijnで expr1 expr2 を生成
                    let expr1 = &db1_with_comb.expr_db;
                    let expr2 = &db2_with_comb.expr_db;
                    
                    let app_db = DeBruijn::App(
                        Box::new(expr1.clone()),
                        Box::new(expr2.clone()),
                    );

                    // 組み合わせ構造を生成
                    let app_comb = Combination::App(
                        Box::new(db1_with_comb.combination.clone()),
                        Box::new(db2_with_comb.combination.clone()),
                    );

                    // プログレスバーに現在の組み合わせを表示
                    if let Some(ref pb) = pb {
                        pb.set_message(format!("n={} 判定中: {}", n, app_comb));
                    }

                    // de Bruijnで標準形を計算
                    // app_dbをcloneしてから normalize（所有権を消費）
                    let norm_start = Instant::now();
                    let app_db_for_norm = app_db.clone();
                    let initial_node_count = app_db_for_norm.node_count();
                    let app_normal_db = app_db_for_norm.normalize(max_reduce_steps);
                    let final_node_count = app_normal_db.node_count();
                    let norm_time = norm_start.elapsed().as_secs_f64();
                    normalize_total_time += norm_time;
                    normalize_count += 1;
                    
                    // ノード数が大きく増加したケースを記録
                    if final_node_count > initial_node_count * 10 && final_node_count > 1000 {
                        eprintln!(
                            "[+{:.3}s] n={}, pair {}/{}: NODE EXPLOSION! initial={}, final={}, comb={}",
                            start_time.elapsed().as_secs_f64(),
                            n,
                            checked_count + 1,
                            total_pairs,
                            initial_node_count,
                            final_node_count,
                            app_comb
                        );
                    }
                    
                    // 遅いnormalize呼び出しを記録
                    if norm_time > 0.1 {
                        eprintln!(
                            "[+{:.3}s] n={}, pair {}/{}: normalize took {:.3}s (slow!), nodes: {} -> {}",
                            start_time.elapsed().as_secs_f64(),
                            n,
                            checked_count + 1,
                            total_pairs,
                            norm_time,
                            initial_node_count,
                            final_node_count
                        );
                    }

                    if app_normal_db == y_normal_db {
                        if let Some(ref pb) = pb {
                            pb.finish_with_message(format!("n={} で解を発見!", n));
                        }

                        // app_dbをそのまま返す
                        return Some(SearchResult {
                            expr: app_db,
                            n,
                            combination: app_comb,
                        });
                    }

                    // 重複チェックして格納（de Bruijn表現で判定）
                    if seen.insert(app_normal_db) {
                        new_combinations.push(DeBruijnWithCombination {
                            expr_db: app_db,
                            combination: app_comb,
                        });
                    }

                    checked_count += 1;
                    if let Some(ref pb) = pb {
                        pb.set_position(checked_count);
                    }
                    
                    let pair_time = pair_start.elapsed().as_secs_f64();
                    
                    // n=14の各ペアの処理時間を記録
                    if n == 14 {
                        eprintln!(
                            "[+{:.3}s] n={}, pair {}/{}: pair_time={:.6}s, norm_time={:.6}s",
                            start_time.elapsed().as_secs_f64(),
                            n,
                            checked_count,
                            total_pairs,
                            pair_time,
                            norm_time
                        );
                    } else if checked_count % 10 == 0 {
                        // 10回ごとに進捗を出力
                        eprintln!(
                            "[+{:.3}s] n={}, progress: {}/{} ({:.1}%), avg_normalize={:.6}s",
                            start_time.elapsed().as_secs_f64(),
                            n,
                            checked_count,
                            total_pairs,
                            (checked_count as f64 / total_pairs as f64) * 100.0,
                            if normalize_count > 0 { normalize_total_time / normalize_count as f64 } else { 0.0 }
                        );
                    }
                }
            }
        }

        if let Some(pb) = pb {
            pb.finish_and_clear();
        }

        eprintln!(
            "[+{:.3}s] n={} 完了: checked={}, new={}, normalize_calls={}, normalize_total={:.3}s, normalize_avg={:.6}s",
            start_time.elapsed().as_secs_f64(),
            n,
            checked_count,
            new_combinations.len(),
            normalize_count,
            normalize_total_time,
            if normalize_count > 0 { normalize_total_time / normalize_count as f64 } else { 0.0 }
        );

        if options.verbose {
            println!(
                "n={}: {} 個の組み合わせをチェック、{} 個のユニークな組み合わせを生成",
                n,
                total_pairs,
                new_combinations.len()
            );
        }

        if new_combinations.is_empty() {
            if options.verbose {
                println!("これ以上新しい組み合わせを生成できません");
            }
            break;
        }

        // 新世代を追加
        let n_elapsed = n_start.elapsed();
        eprintln!("[+{:.3}s] n={} 完了: checked={}, new_combinations={}, n内処理時間={:.3}秒",
                 start_time.elapsed().as_secs_f64(), n, checked_count, 
                 new_combinations.len(), n_elapsed.as_secs_f64());
        eprintln!("  └ normalize: count={}, total={:.3}s, avg={:.6}s", 
                 normalize_count, normalize_total_time, 
                 if normalize_count > 0 { normalize_total_time / normalize_count as f64 } else { 0.0 });
        
        all_generations.push(new_combinations);
    }

    None
}
