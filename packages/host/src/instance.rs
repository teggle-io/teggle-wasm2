use cosmwasm_std::{Api, CanonicalAddr, debug_print, Extern, HumanAddr, Querier, Storage};
use wasmi::{Error as InterpreterError, MemoryInstance, MemoryRef, ModuleRef, RuntimeValue, Trap};

use crate::errors::Wasm2EngineError;
use crate::traits::Wasm2Api;

#[derive(Clone, Copy, Debug)]
pub enum Wasm2Operation {
    Handle,
    Query,
    Verify
}

#[allow(unused)]
impl Wasm2Operation {
    fn is_handle(&self) -> bool {
        matches!(self, Wasm2Operation::Handle)
    }

    fn is_query(&self) -> bool {
        matches!(self, Wasm2Operation::Query)
    }
}

/// Wasm2Instance maps function index to implementation
/// When instantiating a module we give it the Wasm2ImportResolver resolver
/// When invoking a function inside the module we give it this runtime which is the actual functions implementation ()
pub struct Wasm2Instance<'d, S: Storage, A: Api, Q: Querier> {
    pub deps: &'d mut Extern<S, A, Q>,
    pub memory: MemoryRef,
    pub module: ModuleRef,
    operation: Wasm2Operation,
}

impl<'d, S: Storage, A: Api, Q: Querier> Wasm2Instance<'d, S, A, Q> {
    pub fn new(
        deps: &'d mut Extern<S, A, Q>,
        module: ModuleRef,
        operation: Wasm2Operation,
    ) -> Self {
        let memory = (&*module)
            .export_by_name("memory")
            .expect("Module expected to have 'memory' export")
            .as_memory()
            .cloned()
            .expect("'memory' export should be of memory type");

        Self {
            deps,
            memory,
            module,
            operation,
        }
    }

    fn get_memory(&self) -> &MemoryInstance {
        &*self.memory
    }

    /// extract_vector extracts a vector from the wasm memory space
    pub fn extract_vector(&self, vec_ptr_ptr: u32) -> Result<Vec<u8>, Wasm2EngineError> {
        self.extract_vector_inner(vec_ptr_ptr).map_err(|err| {
            debug_print!(
                "WASM2[HOST]: error while trying to read the buffer at {:?} : {:?}",
                vec_ptr_ptr, err
            );
            Wasm2EngineError::MemoryReadError
        })
    }

    fn extract_vector_inner(&self, vec_ptr_ptr: u32) -> Result<Vec<u8>, InterpreterError> {
        let ptr: u32 = self.get_memory().get_value(vec_ptr_ptr)?;

        if ptr == 0 {
            return Err(InterpreterError::Memory(String::from(
                "Trying to read from null pointer in WASM memory",
            )));
        }

        let len: u32 = self.get_memory().get_value(vec_ptr_ptr + 8)?;

        #[allow(deprecated)]
        self.get_memory().get(ptr, len as usize)
    }

    pub fn allocate(&mut self, len: u32) -> Result<u32, Wasm2EngineError> {
        self.allocate_inner(len).map_err(|err| {
            debug_print!("WASM2[HOST]: Failed to allocate {} bytes in wasm: {}", len, err);
            Wasm2EngineError::MemoryAllocationError
        })
    }

    fn allocate_inner(&mut self, len: u32) -> Result<u32, InterpreterError> {
        match self.module.clone().invoke_export(
            "allocate",
            &[RuntimeValue::I32(len as i32)],
            self,
        )? {
            Some(RuntimeValue::I32(0)) => Err(InterpreterError::Memory(String::from(
                "Allocate returned null pointer from WASM",
            ))),
            Some(RuntimeValue::I32(offset)) => Ok(offset as u32),
            other => Err(InterpreterError::Value(format!(
                "allocate method returned value which wasn't u32: {:?}",
                other
            ))),
        }
    }

    pub fn write_to_allocated_memory(
        &mut self,
        buffer: &[u8],
        ptr_to_region_in_wasm_vm: u32,
    ) -> Result<u32, Wasm2EngineError> {
        self.write_to_allocated_memory_inner(buffer, ptr_to_region_in_wasm_vm)
            .map_err(|err| {
                debug_print!(
                    "WASM2[HOST]: error while trying to write the buffer {:?} to the destination buffer at {:?} : {:?}",
                    buffer, ptr_to_region_in_wasm_vm, err
                );
                Wasm2EngineError::MemoryWriteError
            })
    }

    fn write_to_allocated_memory_inner(
        &mut self,
        buffer: &[u8],
        ptr_to_region_in_wasm_vm: u32,
    ) -> Result<u32, InterpreterError> {
        // WASM pointers are pointers to "Region"
        // Region is a struct that looks like this:
        // ptr_to_region -> | 4byte = buffer_addr | 4bytes = buffer_cap | 4bytes = buffer_len |

        // extract the buffer pointer from the region
        let buffer_addr_in_wasm: u32 = self
            .get_memory()
            .get_value::<u32>(ptr_to_region_in_wasm_vm)?;

        if buffer_addr_in_wasm == 0 {
            return Err(InterpreterError::Memory(String::from(
                "Trying to write to null pointer in WASM memory",
            )));
        }

        let buffer_cap_in_wasm: u32 = self
            .get_memory()
            .get_value::<u32>(ptr_to_region_in_wasm_vm + 4)?;

        if buffer_cap_in_wasm < buffer.len() as u32 {
            return Err(InterpreterError::Memory(format!(
                "Tried to write {} bytes but only got {} bytes in destination buffer",
                buffer.len(),
                buffer_cap_in_wasm
            )));
        }

        self.get_memory().set(buffer_addr_in_wasm, buffer)?;

        self.get_memory()
            .set_value::<u32>(ptr_to_region_in_wasm_vm + 8, buffer.len() as u32)?;

        // return the WASM pointer
        Ok(ptr_to_region_in_wasm_vm)
    }

    pub fn write_to_memory(&mut self, buffer: &[u8]) -> Result<u32, Wasm2EngineError> {
        // allocate return a pointer to a region
        let ptr_to_region_in_wasm_vm = self.allocate(buffer.len() as u32)?;
        self.write_to_allocated_memory(buffer, ptr_to_region_in_wasm_vm)
    }
}

impl<'d, S: Storage, A: Api, Q: Querier> Wasm2Api for Wasm2Instance<'d, S, A, Q> {
    /// Args:
    /// 1. "key" to read from Tendermint (buffer of bytes)
    /// key is a pointer to a region "struct" of "pointer" and "length"
    /// A Region looks like { ptr: u32, len: u32 }
    fn read_db_index(&mut self, state_key_ptr_ptr: i32) -> Result<Option<RuntimeValue>, Trap> {
        let state_key_name = self
            .extract_vector(state_key_ptr_ptr as u32)
            .map_err(|err| {
                debug_print!("WASM2[HOST]: read_db() error while trying to read state_key_name from wasm memory");
                err
            })?;

        debug_print!(
            "WASM2[HOST]: read_db() was called from WASM code with state_key_name: {:?}",
            String::from_utf8_lossy(&state_key_name)
        );

        let value = self.deps.storage.get(state_key_name.as_slice());
        let value = match value {
            None => return Ok(Some(RuntimeValue::I32(0))),
            Some(value) => value,
        };

        let ptr_to_region_in_wasm_vm = self.write_to_memory(&value).map_err(|err| {
            debug_print!(
                "WASM2[HOST]: read_db() error while trying to allocate {} bytes for the value",
                value.len(),
            );
            err
        })?;

        // Return pointer to the allocated buffer with the value written to it
        Ok(Some(RuntimeValue::I32(ptr_to_region_in_wasm_vm as i32)))
    }

    /// Args:
    /// 1. "key" to delete from Tendermint (buffer of bytes)
    /// key is a pointer to a region "struct" of "pointer" and "length"
    /// A Region looks like { ptr: u32, len: u32 }
    #[cfg(feature = "query-only")]
    fn remove_db_index(&mut self, _state_key_ptr_ptr: i32) -> Result<Option<RuntimeValue>, Trap> {
        Err(Wasm2EngineError::UnauthorizedWrite.into())
    }

    /// Args:
    /// 1. "key" to delete from Tendermint (buffer of bytes)
    /// key is a pointer to a region "struct" of "pointer" and "length"
    /// A Region looks like { ptr: u32, len: u32 }
    #[cfg(not(feature = "query-only"))]
    fn remove_db_index(&mut self, state_key_ptr_ptr: i32) -> Result<Option<RuntimeValue>, Trap> {
        if self.operation.is_query() {
            return Err(Wasm2EngineError::UnauthorizedWrite.into());
        }

        let state_key_name = self
            .extract_vector(state_key_ptr_ptr as u32)
            .map_err(|err| {
                debug_print!("WASM2[HOST]: remove_db() error while trying to read state_key_name from wasm memory");
                err
            })?;

        debug_print!(
            "WASM2[HOST]: remove_db() was called from WASM code with state_key_name: {:?}",
            String::from_utf8_lossy(&state_key_name)
        );

        self.deps.storage.remove(state_key_name.as_slice());

        Ok(None)
    }

    /// Args:
    /// 1. "key" to write to Tendermint (buffer of bytes)
    /// 2. "value" to write to Tendermint (buffer of bytes)
    /// Both of them are pointers to a region "struct" of "pointer" and "length"
    /// Lets say Region looks like { ptr: u32, len: u32 }
    #[cfg(feature = "query-only")]
    fn write_db_index(
        &mut self,
        _state_key_ptr_ptr: i32,
        _value_ptr_ptr: i32,
    ) -> Result<Option<RuntimeValue>, Trap> {
        Err(Wasm2EngineError::UnauthorizedWrite.into())
    }

    /// Args:
    /// 1. "key" to write to Tendermint (buffer of bytes)
    /// 2. "value" to write to Tendermint (buffer of bytes)
    /// Both of them are pointers to a region "struct" of "pointer" and "length"
    /// Lets say Region looks like { ptr: u32, len: u32 }
    #[cfg(not(feature = "query-only"))]
    fn write_db_index(
        &mut self,
        state_key_ptr_ptr: i32,
        value_ptr_ptr: i32,
    ) -> Result<Option<RuntimeValue>, Trap> {
        if self.operation.is_query() {
            return Err(Wasm2EngineError::UnauthorizedWrite.into());
        }

        let state_key_name = self
            .extract_vector(state_key_ptr_ptr as u32)
            .map_err(|err| {
                debug_print!("WASM2[HOST]: write_db() error while trying to read state_key_name from wasm memory");
                err
            })?;
        let value = self.extract_vector(value_ptr_ptr as u32).map_err(|err| {
            debug_print!("WASM2[HOST]: write_db() error while trying to read value from wasm memory");
            err
        })?;

        debug_print!(
            "WASM2[HOST]: write_db() was called from WASM code with state_key_name: {:?} value: {:?}",
            String::from_utf8_lossy(&state_key_name),
            String::from_utf8_lossy(&value),
        );

        self.deps.storage.set(state_key_name.as_slice(), value.as_slice());

        Ok(None)
    }

    /// Args:
    /// 1. "human" to convert to canonical address (string)
    /// 2. "canonical" a buffer to write the result into (buffer of bytes)
    /// Both of them are pointers to a region "struct" of "pointer" and "length"
    /// A Region looks like { ptr: u32, len: u32 }
    fn canonicalize_address_index(
        &mut self,
        human_ptr_ptr: i32,
        canonical_ptr_ptr: i32,
    ) -> Result<Option<RuntimeValue>, Trap> {
        let human = self.extract_vector(human_ptr_ptr as u32).map_err(|err| {
            debug_print!(
                "WASM2[HOST]: canonicalize_address() error while trying to read human address from wasm memory"
            );
            err
        })?;

        debug_print!(
            "WASM2[HOST]: canonicalize_address() was called from WASM code with {:?}",
            String::from_utf8_lossy(&human)
        );

        // Turn Vec<u8> to str
        let human_addr_str = match std::str::from_utf8(&human) {
            Err(err) => {
                debug_print!(
                    "WASM2[HOST]: canonicalize_address() error while trying to parse human address from bytes to string: {:?}",
                    err
                );
                return Ok(Some(RuntimeValue::I32(
                    self.write_to_memory(b"input is not valid UTF-8")? as i32,
                )));
            }
            Ok(x) => x,
        };

        let human_addr = HumanAddr::from(human_addr_str);

        let canonical = self.deps.api.canonical_address(&human_addr)
            .map_err(|err| {
                debug_print!(
                    "WASM2[HOST]: canonical_address() error {:?}",
                    err,
                );
                Wasm2EngineError::Panic
            })?;

        self.write_to_allocated_memory(&canonical.as_slice(), canonical_ptr_ptr as u32)
            .map_err(|err| {
                debug_print!(
                    "WASM2[HOST]: canonicalize_address() error while trying to write the answer {:?} to the destination buffer",
                    canonical,
                );
                err
            })?;

        // return 0 == ok
        Ok(Some(RuntimeValue::I32(0)))
    }

    /// Args:
    /// 1. "canonical" to convert to human address (buffer of bytes)
    /// 2. "human" a buffer to write the result (humanized string) into (buffer of bytes)
    /// Both of them are pointers to a region "struct" of "pointer" and "length"
    /// A Region looks like { ptr: u32, len: u32 }
    fn humanize_address_index(
        &mut self,
        canonical_ptr_ptr: i32,
        human_ptr_ptr: i32,
    ) -> Result<Option<RuntimeValue>, Trap> {
        let canonical = self
            .extract_vector(canonical_ptr_ptr as u32)
            .map_err(|err| {
                debug_print!(
                    "WASM2[HOST]: humanize_address() error while trying to read canonical address from wasm memory",
                );
                err
            })?;

        debug_print!(
            "WASM2[HOST]: humanize_address() was called from WASM code with {:?}",
            canonical
        );

        let canonical_addr = CanonicalAddr::from(canonical.as_slice());
        let human = self.deps.api.human_address(&canonical_addr)
            .map_err(|err| {
                debug_print!(
                    "WASM2[HOST]: human_address() error {:?}",
                    err,
                );
                Wasm2EngineError::Panic
            })?;

        let human_bytes = human.0.as_bytes();

        self.write_to_allocated_memory(&human_bytes, human_ptr_ptr as u32)
            .map_err(|err| {
                debug_print!(
                    "WASM2[HOST]: humanize_address() error while trying to write the answer {:?} to the destination buffer",
                    &human_bytes,
                );
                err
            })?;

        // return 0 == ok
        Ok(Some(RuntimeValue::I32(0)))
    }

    // stub, for now
    fn query_chain_index(&mut self, query_ptr_ptr: i32) -> Result<Option<RuntimeValue>, Trap> {
        let query_buffer = self.extract_vector(query_ptr_ptr as u32).map_err(|err| {
            debug_print!("WASM2[HOST]: query_chain() error while trying to read canonical address from wasm memory",);
            err
        })?;

        debug_print!(
            "WASM2[HOST]: query_chain() was called from WASM code with {:?}",
            String::from_utf8_lossy(&query_buffer)
        );

        unimplemented!("query_chain_index is not implemented.");

        // TODO:
        //self.deps.querier.query();

        /*

        debug_print!(
            "WASM2[HOST]: query_chain() got answer from outside with gas {} and result {:?}",
            gas_used,
            String::from_utf8_lossy(&answer)
        );

        let ptr_to_region_in_wasm_vm = self.write_to_memory(&answer).map_err(|err| {
            debug_print!(
                "WASM2[HOST]: query_chain() error while trying to allocate and write the answer {:?} to the WASM VM",
                answer,
            );
            err
        })?;

        // Return pointer to the allocated buffer with the value written to it
        Ok(Some(RuntimeValue::I32(ptr_to_region_in_wasm_vm as i32)))
         */
    }

    #[cfg(feature = "debug-print")]
    fn debug_print_index(&self, message_ptr_ptr: i32) -> Result<Option<RuntimeValue>, Trap> {
        let message_buffer = self.extract_vector(message_ptr_ptr as u32).map_err(|err| {
            debug_print!("WASM2[HOST]: debug_print() error while trying to read message from wasm memory",);
            err
        })?;

        let message =
            String::from_utf8(message_buffer)
                .map_err(|err| {
                    debug_print!(
                        "WASM2[HOST]: debug_print_index() error wncoding string {}",
                        &err,
                    );
                    Wasm2EngineError::Panic
                })?;

        // TODO: customise the version e.t.c.
        debug_print!("WASM2[cortex.v1]: {:?}", message);

        Ok(None)
    }
}
