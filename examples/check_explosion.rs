use lambda::lambda::DeBruijn;
use lambda::parser::parse;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 元のX
    let x = parse(r"\f1. f1 (\g1. \y1. \s1. g1 s1 (y1 s1)) (\a1. \b1. a1)")?;
    
    // 問題の式: X(X(X(X(XX)))(X(X(X(XX)))(XX))X)
    // これをパースするのは複雑なので、段階的に構築
    
    println!("=== ノード爆発の分析 ===\n");
    
    // まずシンプルな自己適用から
    let omega = parse(r"(\x. x x)(\x. x x)")?; // Ω コンビネータ
    println!("Ω = (λx.x x)(λx.x x)");
    println!("  初期ノード数: {}", omega.node_count());
    
    // 数ステップ簡約してみる
    let mut current = omega.clone();
    for step in 1..=5 {
        if let Some(next) = current.beta_reduce_step() {
            current = next;
            println!("  step {}: ノード数 = {}", step, current.node_count());
        } else {
            println!("  step {}: 正規形に到達", step);
            break;
        }
    }
    
    println!("\n=== X X の振る舞い ===");
    let xx = parse(
        r"(\f1. f1 (\g1. \y1. \s1. g1 s1 (y1 s1)) (\a1. \b1. a1))(\f1. f1 (\g1. \y1. \s1. g1 s1 (y1 s1)) (\a1. \b1. a1))"
    )?;
    println!("初期ノード数: {}", xx.node_count());
    
    let mut current = xx.clone();
    for step in 1..=20 {
        if let Some(next) = current.beta_reduce_step() {
            current = next;
            println!("  step {}: ノード数 = {}", step, current.node_count());
        } else {
            println!("  step {}: 正規形に到達", step);
            println!("  最終形: {}", current);
            break;
        }
    }
    
    Ok(())
}
