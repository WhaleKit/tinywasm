mod testsuite;
use eyre::{eyre, Result};
use owo_colors::OwoColorize;
use testsuite::TestSuite;

fn main() -> Result<()> {
    TestSuite::set_log_level(log::LevelFilter::Off);

    let mut test_suite = TestSuite::new();
    test_suite.skip("memory64/array.wast");
    test_suite.skip("memory64/extern.wast");
    test_suite.skip("memory64/global.wast");
    test_suite.skip("memory64/i31.wast");
    test_suite.skip("memory64/ref_null.wast");
    test_suite.skip("memory64/select.wast");
    test_suite.skip("memory64/simd_address.wast");
    test_suite.skip("memory64/simd_lane.wast");
    test_suite.skip("memory64/struct.wast");
    test_suite.skip("memory64/table.wast");

    test_suite.run_files(proposal(&Proposal::Memory64))?;
    test_suite.save_csv("./tests/generated/wasm-memory64.csv", env!("CARGO_PKG_VERSION"))?;
    test_suite.report_status()
}
