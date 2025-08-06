#![no_std]

extern crate alloc;

pub mod consts;
pub mod proof;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ControllerInputs {
    pub erc20: alloc::string::String,
    pub eth_addr: alloc::string::String,
    pub neutron_addr: alloc::string::String,
}
