use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use crate::msg::BondingRecord;
pub enum Denom {
    Native(String),
    Cw20(Addr),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub pool_address: Addr,
    pub treasury_address: Addr,
    pub fury_token_address: Denom,
    pub lock_days: u64,
    pub discount: u64,
    pub usdc_denom: String,
    pub is_native_bonding: bool,
    pub tx_fee: u64,
    pub platform_fee: u64,
    pub enabled: bool
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const BONDING_KEY: &str = "bonding";
pub const BONDING: Map<Addr, Vec<BondingRecord>> = Map::new(BONDING_KEY);
