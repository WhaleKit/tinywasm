mod testsuite;
use eyre::Result;
use testsuite::TestSuite;
use wasm_testsuite::data::{proposal, Proposal};

fn main() -> Result<()> {
    TestSuite::set_log_level(log::LevelFilter::Off);

    let mut test_suite = TestSuite::new();
    test_suite.skip("simd_lane.wast");
    test_suite.run_files(proposal(&Proposal::Annotations))?;
    test_suite.save_csv("./tests/generated/wasm-annotations.csv", env!("CARGO_PKG_VERSION"))?;
    test_suite.report_status()
}
