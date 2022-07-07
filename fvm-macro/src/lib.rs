extern crate core;

pub mod blockstore;

pub use contract::contract;
pub use blockstore::Blockstore;
pub use fvm_sdk;
pub use fvm_shared;
pub use std::convert::TryFrom;
pub use anyhow::{anyhow, Result};
pub use cid::multihash::Code;
pub use cid::Cid;
pub use fvm_ipld_blockstore::Block;
pub use fvm_sdk as sdk;
pub use fvm_ipld_encoding::{to_vec, CborStore, DAG_CBOR, RawBytes};
pub use fvm_ipld_encoding::tuple::{Deserialize_tuple, Serialize_tuple};
pub use serde_tuple;
pub use serde;
pub use fvm_sdk::message::{params_raw, NO_DATA_BLOCK_ID};

#[macro_export]
macro_rules! abort {
  ($code:ident, $msg:literal $(, $ex:expr)*) => {
      fvm_sdk::vm::abort(
          fvm_shared::error::ExitCode::$code.value(),
          Some(format!($msg, $($ex,)*).as_str()),
      )
  };
}

pub trait State {
    fn load() -> Self;
    fn save(&self) -> Cid;
}