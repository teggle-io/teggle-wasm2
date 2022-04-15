use cosmwasm_std::debug_print;
use derive_more::Display;
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

pub fn wasmi_error_to_wasm2_error(wasmi_error: InterpreterError) -> Wasm2EngineError {
    debug_print!("WASM2[HOST]: wasmi host error {wasmi_error}");

    Wasm2EngineError::Panic
}