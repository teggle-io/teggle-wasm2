use std::io::{Cursor, Read};
use cosmwasm_std::{Api, debug_print, Extern, Querier, StdError, Storage};
use libflate::gzip::Decoder;
use wasmi::{Module, ModuleInstance, ModuleRef, RuntimeValue};

use crate::errors::{Wasm2EngineError, wasmi_error_to_wasm2_error};
use crate::import_resolver::{create_builder, Wasm2ImportResolver};
use crate::instance::{Wasm2Instance, Wasm2Operation};

pub struct Engine<'d, S: Storage, A: Api, Q: Querier> {
    instance: Wasm2Instance<'d, S, A, Q>,
    module: ModuleRef,
}

impl<'d, S: Storage, A: Api, Q: Querier> Engine<'d, S, A, Q> {
    pub fn new(instance: Wasm2Instance<'d, S, A, Q>, module: ModuleRef) -> Self {
        Self {
            instance,
            module,
        }
    }

    pub fn write_to_memory(&mut self, buffer: &[u8]) -> Result<u32, Wasm2EngineError> {
        self.instance.write_to_memory(buffer)
    }

    pub fn extract_vector(&self, vec_ptr_ptr: u32) -> Result<Vec<u8>, Wasm2EngineError> {
        self.instance.extract_vector(vec_ptr_ptr)
    }

    pub fn handle(&mut self, env_ptr: u32, msg_ptr: u32) -> Result<u32, Wasm2EngineError> {
        debug_print!("WASM2[HOST]: Invoking handle() in wasm");

        // Itzik: leaving this here as an example in case we will want to do something like this in the future

        // let stored_address = read_encrypted_key(
        //     b"key",
        //     &self.instance.context,
        //     &self.instance.contract_key,
        // )
        // .map_err(|_| {
        //     error!("WTF wrong contract key are you crazy???");
        //     EnclaveError::InternalError
        // })?;
        //
        // match stored_address.0 {
        //     Some(addr) => {
        //         if addr != contract_key.to_vec() {
        //             error!("WTF wrong contract key are you crazy???");
        //             return Err(EnclaveError::FailedUnseal);
        //         }
        //         Ok(())
        //     }
        //     None => {
        //         error!("WTF no contract address found you must be trippin' dawg");
        //         Err(EnclaveError::InternalError)
        //     }
        // }?;

        match self
            .module
            .invoke_export(
                "handle",
                &[
                    RuntimeValue::I32(env_ptr as i32),
                    RuntimeValue::I32(msg_ptr as i32),
                ],
                &mut self.instance,
            )
            .map_err(wasmi_error_to_wasm2_error(
                "error calling 'handle' in guest".to_string()))?
        {
            Some(RuntimeValue::I32(offset)) => Ok(offset as u32),
            other => {
                debug_print!("WASM2[HOST]: handle method returned value which wasn't u32: {:?}", other);
                Err(Wasm2EngineError::Panic)
            }
        }
    }

    pub fn query(&mut self, msg_ptr: u32) -> Result<u32, Wasm2EngineError> {
        debug_print!("WASM2[HOST]: Invoking query() in wasm");

        match self
            .module
            .invoke_export(
                "query",
                &[RuntimeValue::I32(msg_ptr as i32)],
                &mut self.instance,
            )
            .map_err(wasmi_error_to_wasm2_error(
                "error calling 'query' in guest".to_string()))?
        {
            Some(RuntimeValue::I32(offset)) => Ok(offset as u32),
            other => {
                debug_print!("WASM2[HOST]: query method returned value which wasn't u32: {:?}", other);
                Err(Wasm2EngineError::Panic)
            }
        }
    }
}

pub fn deflate_wasm(compressed_bytes: &[u8]) -> Result<Vec<u8>, StdError> {
    let mut decoder = Decoder::new(
        Cursor::new(compressed_bytes)).unwrap();
    let mut buf = Vec::new();

    let res = decoder.read_to_end(&mut buf);
    if !res.is_ok() {
        return Err(StdError::GenericErr {
            msg: format!("failed to deflate WASM binary"),
            backtrace: None,
        });
    }

    debug_print!("WASM: deflated contract ({} bytes)", res.unwrap());

    return Ok(buf);
}

pub fn parse_wasm(wasm_binary_u8: &[u8]) -> Result<Module, StdError> {
    return match Module::from_buffer(&wasm_binary_u8) {
        Ok(tree) => {
            debug_print("WASM: parsed module");

            Ok(tree)
        }
        Err(err) => {
            Err(StdError::GenericErr {
                msg: format!("failed to parse WASM binary: {err}"),
                backtrace: None,
            })
        }
    };
}

pub fn start_engine_from_wasm_binary<'d, S: Storage, A: Api, Q: Querier>(
    data: &[u8],
    deps: &'d mut Extern<S, A, Q>,
    operation: Wasm2Operation,
) -> Result<Engine<'d, S, A, Q>, StdError> {
    let wasm = deflate_wasm(&data)?;
    let module = parse_wasm(&wasm.as_slice())?;

    return start_engine(deps, module, operation);
}

pub fn start_engine<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    module: Module,
    operation: Wasm2Operation,
) -> Result<Engine<S, A, Q>, StdError> {
    let resolver = Wasm2ImportResolver {};
    let imports = create_builder(&resolver);

    // Instantiate a module with our imports and assert that there is no `start` function.
    let module_instance = ModuleInstance::new(&module, &imports)
        .map_err(|err| {
            debug_print!("Error in instantiation: {:?}", err);

            return StdError::GenericErr {
                msg: format!("WASM2 module invalid: {err}"),
                backtrace: None,
            };
        })?;
    if module_instance.has_start() {
        return Err(StdError::GenericErr {
            msg: format!("WASM2 module provided should not have 'start' defined"),
            backtrace: None,
        });
    }

    let module_ref = module_instance.not_started_instance().clone();
    let instance = Wasm2Instance::new(deps, module_ref.clone(), operation);

    Ok(Engine::new(instance, module_ref))
}
