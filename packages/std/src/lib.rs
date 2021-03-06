// Exposed on all platforms

mod addresses;
mod coins;
mod debug_print;
mod encoding;
mod entry_points;
mod errors;
mod init_handle;
#[cfg(feature = "iterator")]
mod iterator;
mod math;
mod query;
mod serde;
mod storage;
mod traits;
mod types;

pub use crate::addresses::{CanonicalAddr, HumanAddr};
pub use crate::coins::{coin, coins, has_coins, Coin};
pub use crate::debug_print::debug_print;
pub use crate::encoding::Binary;
pub use crate::errors::{StdError, StdResult, SystemError, SystemResult};
pub use crate::init_handle::{
    log, plaintext_log, BankMsg, Context, CosmosMsg, GovMsg, HandleResponse, HandleResult,
    LogAttribute,  StakingMsg, VoteOption,  WasmMsg,
};
#[cfg(feature = "iterator")]
pub use crate::iterator::{Order, KV};
pub use crate::math::{Decimal, Uint128};
pub use crate::query::{
    AllBalanceResponse, AllDelegationsResponse, BalanceResponse, BankQuery, BondedDenomResponse,
    BondedRatioResponse, Delegation, DistQuery, FullDelegation, GovQuery, InflationResponse,
    MintQuery, ProposalsResponse, QueryRequest, QueryResponse, QueryResult, RewardsResponse,
    StakingQuery, UnbondingDelegationsResponse, Validator, ValidatorsResponse, WasmQuery,
};
pub use crate::serde::{from_binary, from_slice, to_binary, to_vec};
pub use crate::storage::MemoryStorage;
pub use crate::traits::{Api, Extern, Querier, QuerierResult, ReadonlyStorage, Storage};
pub use crate::types::{BlockInfo, ContractInfo, Empty, Env, MessageInfo};

// Exposed in wasm build only

#[cfg(target_arch = "wasm32")]
mod exports;
#[cfg(target_arch = "wasm32")]
mod imports;
#[cfg(target_arch = "wasm32")]
pub mod memory; // Used by exports and imports only. This assumes pointers are 32 bit long, which makes it untestable on dev machines.

// TODO: REMOVE PUB MEMORY

#[cfg(target_arch = "wasm32")]
pub use crate::exports::{do_handle, do_query};
#[cfg(target_arch = "wasm32")]
pub use crate::imports::{ExternalApi, ExternalQuerier, ExternalStorage};

// Exposed for testing only
// Both unit tests and integration tests are compiled to native code, so everything in here does not need to compile to Wasm.

#[cfg(not(target_arch = "wasm32"))]
mod mock;
#[cfg(not(target_arch = "wasm32"))]
pub mod testing {
    pub use crate::mock::{
        mock_dependencies, mock_dependencies_with_balances, mock_env, BankQuerier, MockApi,
        MockQuerier, MockQuerierCustomHandlerResult, MockStorage, StakingQuerier,
        MOCK_CONTRACT_ADDR,
    };
}
