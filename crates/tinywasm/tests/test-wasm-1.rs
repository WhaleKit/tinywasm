mod testsuite;
use eyre::Result;

fn main() -> Result<()> {
    println!("Skipping 1.0 tests (False positives due to relaxed validation in later versions)");
    Ok(())
    // TestSuite::set_log_level(log::LevelFilter::Off);

    // let mut test_suite = TestSuite::new();
    // test_suite.run_files(spec(&SpecVersion::V1))?;
    // test_suite.save_csv("./tests/generated/wasm-1.csv", env!("CARGO_PKG_VERSION"))?;
    // test_suite.report_status()
}
