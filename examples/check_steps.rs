use lambda::parser::parse;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut e2 = parse(r"\_. \x. (\y. \_. y) ( (\z. z x) ) x")?;
    println!("Step 0: {}", e2.pretty_print(0));
    let mut step = 1;
    while let Some(next) = e2.normalize_step() {
        println!("Step {}: {}", step, next.pretty_print(0));
        e2 = next;
        step += 1;
    }

    Ok(())
}
