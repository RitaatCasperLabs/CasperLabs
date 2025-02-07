use crate::support::test_support::{ExecuteRequestBuilder, InMemoryWasmTestBuilder};
use contract_ffi::value::account::PublicKey;

use crate::test::{DEFAULT_ACCOUNT_ADDR, DEFAULT_GENESIS_CONFIG};

const CONTRACT_EE_401_REGRESSION: &str = "ee_401_regression.wasm";
const CONTRACT_EE_401_REGRESSION_CALL: &str = "ee_401_regression_call.wasm";

#[ignore]
#[test]
fn should_execute_contracts_which_provide_extra_urefs() {
    let exec_request_1 =
        ExecuteRequestBuilder::standard(DEFAULT_ACCOUNT_ADDR, CONTRACT_EE_401_REGRESSION, ())
            .build();

    let exec_request_2 = ExecuteRequestBuilder::standard(
        DEFAULT_ACCOUNT_ADDR,
        CONTRACT_EE_401_REGRESSION_CALL,
        (PublicKey::new(DEFAULT_ACCOUNT_ADDR),),
    )
    .build();
    let _result = InMemoryWasmTestBuilder::default()
        .run_genesis(&DEFAULT_GENESIS_CONFIG)
        .exec(exec_request_1)
        .expect_success()
        .commit()
        .exec(exec_request_2)
        .expect_success()
        .commit()
        .finish();
}
