mod testsuite;
use eyre::Result;

fn main() -> Result<()> {
    println!("Skipping Wasm Custom Page Sizes tests (Wast doesn't support the syntax yet)");
    Ok(())
}
