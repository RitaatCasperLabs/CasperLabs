use engine_core::engine_state::Error;

use crate::{
    support::test_support::{ExecuteRequestBuilder, InMemoryWasmTestBuilder},
    test::DEFAULT_GENESIS_CONFIG,
};

const CONTRACT_EE_532_REGRESSION: &str = "ee_532_regression.wasm";
const UNKNOWN_ADDR: [u8; 32] = [42u8; 32];

#[ignore]
#[test]
fn should_run_ee_532_get_uref_regression_test() {
    // This test runs a contract that's after every call extends the same key with
    // more data

    let exec_request =
        ExecuteRequestBuilder::standard(UNKNOWN_ADDR, CONTRACT_EE_532_REGRESSION, ()).build();

    let result = InMemoryWasmTestBuilder::default()
        .run_genesis(&DEFAULT_GENESIS_CONFIG)
        .exec(exec_request)
        .commit()
        .finish();

    let deploy_result = result
        .builder()
        .get_exec_response(0)
        .expect("should have exec response")
        .get_success()
        .get_deploy_results()
        .get(0)
        .expect("should have at least one deploy result");

    assert!(
        deploy_result.has_precondition_failure(),
        "expected precondition failure"
    );

    let message = deploy_result.get_precondition_failure().get_message();
    assert_eq!(
        message,
        format!("{}", Error::AuthorizationError),
        "expected AuthorizationError"
    )
}
