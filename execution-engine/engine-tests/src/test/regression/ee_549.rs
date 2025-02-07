use crate::{
    support::test_support::{ExecuteRequestBuilder, InMemoryWasmTestBuilder},
    test::{DEFAULT_ACCOUNT_ADDR, DEFAULT_GENESIS_CONFIG},
};

const CONTRACT_EE_549_REGRESSION: &str = "ee_549_regression.wasm";

#[ignore]
#[test]
fn should_run_ee_549_set_refund_regression() {
    let exec_request =
        ExecuteRequestBuilder::standard(DEFAULT_ACCOUNT_ADDR, CONTRACT_EE_549_REGRESSION, ())
            .build();

    let mut builder = InMemoryWasmTestBuilder::default();

    builder
        .run_genesis(&DEFAULT_GENESIS_CONFIG)
        .exec(exec_request);

    // Execution should encounter an error because set_refund
    // is not allowed to be called during session execution.
    assert!(builder.is_error());
}
