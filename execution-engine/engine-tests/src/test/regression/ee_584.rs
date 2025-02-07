use contract_ffi::value::Value;
use engine_shared::transform::Transform;

use crate::{
    support::test_support::{ExecuteRequestBuilder, InMemoryWasmTestBuilder},
    test::{DEFAULT_ACCOUNT_ADDR, DEFAULT_GENESIS_CONFIG},
};

const CONTRACT_EE_584_REGRESSION: &str = "ee_584_regression.wasm";

#[ignore]
#[test]
fn should_run_ee_584_no_errored_session_transforms() {
    let exec_request =
        ExecuteRequestBuilder::standard(DEFAULT_ACCOUNT_ADDR, CONTRACT_EE_584_REGRESSION, ())
            .build();

    let mut builder = InMemoryWasmTestBuilder::default();

    builder
        .run_genesis(&DEFAULT_GENESIS_CONFIG)
        .exec(exec_request);

    assert!(builder.is_error());

    let transforms = builder.get_transforms();

    assert!(transforms[0]
        .iter()
        .find(|(_, t)| if let Transform::Write(Value::String(s)) = t {
            s == "Hello, World!"
        } else {
            false
        })
        .is_none());
}
