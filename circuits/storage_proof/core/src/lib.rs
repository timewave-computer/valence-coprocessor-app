#![no_std]

extern crate alloc;

pub mod consts;
pub mod proof;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ControllerInputs {
    pub erc20_addr: alloc::string::String,
    pub erc20_balances_map_storage_index: u64,
    pub eth_addr: alloc::string::String,
    pub neutron_addr: alloc::string::String,
}
