use failure::Fail;

use engine_shared::newtypes::Blake2bHash;

use contract_ffi::{bytesrepr, system_contracts::mint};

use crate::execution;
use contract_ffi::value::ProtocolVersion;

#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "Invalid hash length: expected {}, actual {}", _0, _1)]
    InvalidHashLength { expected: usize, actual: usize },
    #[fail(display = "Invalid public key length: expected {}, actual {}", _0, _1)]
    InvalidPublicKeyLength { expected: usize, actual: usize },
    #[fail(display = "Invalid protocol version: {}", _0)]
    InvalidProtocolVersion(ProtocolVersion),
    #[fail(display = "Invalid upgrade config")]
    InvalidUpgradeConfig,
    #[fail(display = "Wasm preprocessing error: {}", _0)]
    WasmPreprocessingError(engine_wasm_prep::PreprocessingError),
    #[fail(display = "Wasm serialization error: {:?}", _0)]
    WasmSerializationError(parity_wasm::SerializationError),
    #[fail(display = "Execution error: {}", _0)]
    ExecError(execution::Error),
    #[fail(display = "Storage error: {}", _0)]
    StorageError(engine_storage::error::Error),
    #[fail(display = "Authorization failure: not authorized.")]
    AuthorizationError,
    #[fail(display = "Insufficient payment")]
    InsufficientPaymentError,
    #[fail(display = "Deploy error")]
    DeployError,
    #[fail(display = "Payment finalization error")]
    FinalizationError,
    #[fail(display = "Missing system contract association: {}", _0)]
    MissingSystemContractError(String),
    #[fail(display = "Serialization error: {}", _0)]
    SerializationError(bytesrepr::Error),
    #[fail(display = "Mint error: {}", _0)]
    MintError(mint::Error),
}

impl From<engine_wasm_prep::PreprocessingError> for Error {
    fn from(error: engine_wasm_prep::PreprocessingError) -> Self {
        Error::WasmPreprocessingError(error)
    }
}

impl From<parity_wasm::SerializationError> for Error {
    fn from(error: parity_wasm::SerializationError) -> Self {
        Error::WasmSerializationError(error)
    }
}

impl From<execution::Error> for Error {
    fn from(error: execution::Error) -> Self {
        Error::ExecError(error)
    }
}

impl From<engine_storage::error::Error> for Error {
    fn from(error: engine_storage::error::Error) -> Self {
        Error::StorageError(error)
    }
}

impl From<bytesrepr::Error> for Error {
    fn from(error: bytesrepr::Error) -> Self {
        Error::SerializationError(error)
    }
}

impl From<mint::Error> for Error {
    fn from(error: mint::Error) -> Self {
        Error::MintError(error)
    }
}

impl From<!> for Error {
    fn from(error: !) -> Self {
        match error {}
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RootNotFound(pub Blake2bHash);
