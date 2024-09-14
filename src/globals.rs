use lazy_static::lazy_static;
use alloy::primitives::Address;

lazy_static! {
    pub static ref WETH_ADDRESS: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse::<Address>().unwrap();
    pub static ref V2_ROUTER_ADDRESS: Address = "0x7a250d5630b4cf539739df2c5dacb4c659f2488d".parse::<Address>().unwrap();
    pub static ref V2_FACTORY_ADDRESS: Address = "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f".parse::<Address>().unwrap();
}
