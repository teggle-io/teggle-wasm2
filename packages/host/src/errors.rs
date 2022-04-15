use cosmwasm_std::{debug_print, StdError};
use derive_more::Display;
use serde_json_wasm::ser::Error;
use wasmi::{Error as InterpreterError, HostError};

#[derive(Debug, Display)]
#[non_exhaustive]
pub enum Wasm2EngineError {
    HostMisbehavior,
    OutOfGas,
    Panic,

    EncryptionError,
    DecryptionError,
    SerializationError,
    DeserializationError,
    // This is for unexpected error while processing base32 data.
    Base32Error,

    MemoryAllocationError,
    MemoryReadError,
    MemoryWriteError,
    /// The contract attempted to write to storage during a query
    UnauthorizedWrite,

    NonExistentImportFunction,
}

impl HostError for Wasm2EngineError {}

pub fn wasmi_error_to_wasm2_error(msg: String) -> impl Fn(InterpreterError) -> Wasm2EngineError {
    move |err| -> Wasm2EngineError {
        debug_print!(
            "WASM2[HOST]: WASMI host error - {}: {}", &msg, err.to_string()
        );
        Wasm2EngineError::Panic
    }
}

pub fn wasm2_error_to_stderr(msg: String) -> impl Fn(Wasm2EngineError) -> StdError {
    move |err| -> StdError {
        debug_print!(
            "WASM2[HOST]: WASM2 engine error - {}: {}", &msg, err.to_string()
        );
        StdError::GenericErr {
            msg: err.to_string(),
            backtrace: None
        }
    }
}

pub fn serde_error_to_stderr(msg: String) -> impl Fn(Error) -> StdError {
    move |err| -> StdError {
        debug_print!(
            "WASM2[HOST]: WASM2 engine error - {}: {}", &msg, err.to_string()
        );
        StdError::GenericErr {
            msg: err.to_string(),
            backtrace: None
        }
    }
}