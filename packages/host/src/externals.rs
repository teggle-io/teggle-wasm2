use cosmwasm_std::{Api, debug_print, Querier, Storage};
use wasmi::{Externals, RuntimeArgs, RuntimeValue, Trap};

use crate::instance::Wasm2Instance;
use crate::errors::Wasm2EngineError;
use crate::traits::Wasm2Api;

#[derive(PartialEq, Eq)]
pub enum HostFunctions {
    ReadDbIndex = 0,
    WriteDbIndex = 1,
    RemoveDbIndex = 2,
    CanonicalizeAddressIndex = 3,
    HumanizeAddressIndex = 4,
    QueryChainIndex = 6,
    #[cfg(feature = "debug-print")]
    DebugPrintIndex = 254,
    Unknown,
}

impl From<usize> for HostFunctions {
    fn from(v: usize) -> Self {
        match v {
            x if x == HostFunctions::ReadDbIndex as usize => HostFunctions::ReadDbIndex,
            x if x == HostFunctions::WriteDbIndex as usize => HostFunctions::WriteDbIndex,
            x if x == HostFunctions::RemoveDbIndex as usize => HostFunctions::RemoveDbIndex,
            x if x == HostFunctions::CanonicalizeAddressIndex as usize => {
                HostFunctions::CanonicalizeAddressIndex
            }
            x if x == HostFunctions::HumanizeAddressIndex as usize => {
                HostFunctions::HumanizeAddressIndex
            }
            x if x == HostFunctions::QueryChainIndex as usize => HostFunctions::QueryChainIndex,
            #[cfg(feature = "debug-print")]
            x if x == HostFunctions::DebugPrintIndex as usize => HostFunctions::DebugPrintIndex,
            _ => HostFunctions::Unknown,
        }
    }
}

impl Into<usize> for HostFunctions {
    fn into(self) -> usize {
        self as usize
    }
}

/// Wasm2 Trait implementation
impl <'d, S: Storage, A: Api, Q: Querier> Externals for Wasm2Instance<'d, S, A, Q>  {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match HostFunctions::from(index) {
            HostFunctions::ReadDbIndex => {
                let key: i32 = args.nth_checked(0).map_err(|err| {
                    debug_print!(
                        "WASM2[HOST]: read_db() error reading arguments, stopping wasm: {:?}",
                        err
                    );
                    err
                })?;
                self.read_db_index(key)
            }
            HostFunctions::RemoveDbIndex => {
                let key: i32 = args.nth_checked(0).map_err(|err| {
                    debug_print!(
                        "WASM2[HOST]: remove_db() error reading arguments, stopping wasm: {:?}",
                        err
                    );
                    err
                })?;
                self.remove_db_index(key)
            }
            HostFunctions::WriteDbIndex => {
                let key: i32 = args.nth_checked(0).map_err(|err| {
                    debug_print!(
                        "WASM2[HOST]: write_db() error reading arguments, stopping wasm: {:?}",
                        err
                    );
                    err
                })?;
                // Get pointer to the region of the value
                let value: i32 = args.nth_checked(1)?;

                self.write_db_index(key, value)
            }
            HostFunctions::CanonicalizeAddressIndex => {
                let human: i32 = args.nth_checked(0).map_err(|err| {
                    debug_print!(
                        "WASM2[HOST]: canonicalize_address() error reading arguments, stopping wasm: {:?}",
                        err
                    );
                    err
                })?;

                let canonical: i32 = args.nth_checked(1)?;

                self.canonicalize_address_index(human, canonical)
            }
            // fn humanize_address(canonical: *const c_void, human: *mut c_void) -> i32;
            HostFunctions::HumanizeAddressIndex => {
                let canonical: i32 = args.nth_checked(0).map_err(|err| {
                    debug_print!(
                        "WASM2[HOST]: humanize_address() error reading first argument, stopping wasm: {:?}",
                        err
                    );
                    err
                })?;

                let human: i32 = args.nth_checked(1).map_err(|err| {
                    debug_print!(
                        "WASM2[HOST]: humanize_address() error reading second argument, stopping wasm: {:?}",
                        err
                    );
                    err
                })?;

                self.humanize_address_index(canonical, human)
            }
            HostFunctions::QueryChainIndex => {
                let query: i32 = args.nth_checked(0).map_err(|err| {
                    debug_print!(
                        "WASM2[HOST]: query_chain() error reading argument, stopping wasm: {:?}",
                        err
                    );
                    err
                })?;

                self.query_chain_index(query)
            }
            #[cfg(feature = "debug-print")]
            HostFunctions::DebugPrintIndex => {
                let message: i32 = args.nth_checked(0).map_err(|err| {
                    debug_print!(
                        "WASM2[HOST]: debug_print() error reading argument, stopping wasm: {:?}",
                        err
                    );
                    err
                })?;

                self.debug_print_index(message)
            }
            HostFunctions::Unknown => {
                debug_print!("WASM2[HOST]: unknown function index");
                Err(Wasm2EngineError::NonExistentImportFunction.into())
            }
        }
    }
}
