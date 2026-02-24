use lambda::parser::parse;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 単純な式の例
    let id = parse(r"\x.x")?;
    println!("Identity function:");
    println!("  100 steps: {}", id.normalize(100));
    println!("  200 steps: {}", id.normalize(200));

    // targetの正規化
    let target = parse(r"\x.\a.\b.x (\x.\b.b (x a)) (\y.b) (\x.x)")?;
    println!("\nTarget (complex):");
    println!("  100 steps: {}", target.normalize(100));
    println!("  200 steps: {}", target.normalize(200));
    println!("  400 steps: {}", target.normalize(400));
    println!(
        "  Same? 100==200: {}",
        target.normalize(100) == target.normalize(200)
    );
    println!(
        "  Same? 200==400: {}",
        target.normalize(200) == target.normalize(400)
    );

    Ok(())
}
