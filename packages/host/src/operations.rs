use cosmwasm_std::{Api, debug_print, Env, Extern, HandleResponse, Querier, StdResult, Storage};
use serde::{Deserialize, Serialize};

use crate::{start_engine_from_wasm_binary, Wasm2Operation};
use crate::errors::{serde_error_to_stderr, wasm2_error_to_stderr};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    DoNothing {}
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    data: &[u8],
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let mut engine = start_engine_from_wasm_binary(&data, deps,
                                                   Wasm2Operation::Handle)?;

    let env_bytes = serde_json_wasm::to_vec(&env)
        .map_err(serde_error_to_stderr(
            "got an error while trying to serialize 'Env' into bytes".to_string()))?;

    // TODO: something more than a static msg.
    let msg = HandleMsg::DoNothing {};
    let msg_bytes = serde_json_wasm::to_vec(&msg)
        .map_err(serde_error_to_stderr(
            "got an error while trying to serialize 'HandleMsg' into bytes".to_string()))?;

    let env_ptr = engine.write_to_memory(&env_bytes)
        .map_err(wasm2_error_to_stderr(
            "failed to write 'Env' to memory for WASM2 guest".to_string()))?;
    let msg_ptr = engine.write_to_memory(&msg_bytes)
        .map_err(wasm2_error_to_stderr(
            "failed to write 'HandleMsg' to memory for WASM2 guest".to_string()))?;

    let res_vec_ptr = engine.handle(env_ptr, msg_ptr)
        .map_err(wasm2_error_to_stderr(
            "got an error while calling 'handle' on WASM2 guest".to_string()))?;

    let res_vec = engine.extract_vector(res_vec_ptr)
        .map_err(wasm2_error_to_stderr(
            "got an error extracting the results vector during 'handle'".to_string()))?;

    debug_print!("WASM2[HOST]: handle call successful, bytes {} returned", res_vec.len());

    Ok(HandleResponse::default())
}