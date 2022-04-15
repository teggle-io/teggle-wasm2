extern crate libflate;
extern crate wasmi;

pub mod engine;
pub mod instance;
pub mod externals;
pub mod import_resolver;
pub mod errors;
pub mod traits;

pub use engine::{Engine, deflate_wasm, parse_wasm, start_engine_from_wasm_binary, start_engine};
pub use instance::{Wasm2Instance, Wasm2Operation};
pub use errors::Wasm2EngineError;
