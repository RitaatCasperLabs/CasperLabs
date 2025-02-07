use crate::support::test_support::{ExecuteRequestBuilder, InMemoryWasmTestBuilder};
use contract_ffi::value::account::Weight;

use crate::test::{DEFAULT_ACCOUNT_ADDR, DEFAULT_GENESIS_CONFIG};

const CONTRACT_EE_539_REGRESSION: &str = "ee_539_regression.wasm";

#[ignore]
#[test]
fn should_run_ee_539_serialize_action_thresholds_regression() {
    // This test runs a contract that's after every call extends the same key with
    // more data
    let exec_request = ExecuteRequestBuilder::standard(
        DEFAULT_ACCOUNT_ADDR,
        CONTRACT_EE_539_REGRESSION,
        (Weight::new(4), Weight::new(3)),
    )
    .build();

    let _result = InMemoryWasmTestBuilder::default()
        .run_genesis(&DEFAULT_GENESIS_CONFIG)
        .exec(exec_request)
        .expect_success()
        .commit()
        .finish();
}
