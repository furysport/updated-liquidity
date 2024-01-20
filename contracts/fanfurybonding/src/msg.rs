// msg.rs

use cosmwasm_std::{Addr, Uint128};
use cw20::Denom;

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum ExecuteMsg {
    UpdateOwner { owner: Addr },
    UpdateEnabled { enabled: bool },
    UpdateConfig {
        lock_days: u64,
        discount: u64,
        tx_fee: u64,
        platform_fee: u64,
    },
    Bond {},
    LpBond { address: Addr, amount: Uint128 },
    Unbond { index: u64 },
    Withdraw { amount: Uint128 },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum QueryMsg {
    Config {},
    BondState { address: Addr },
    AllBondState { start_after: Option<String>, limit: Option<u32> },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum MigrateMsg {
    // Define migration-related messages if needed
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ConfigResponse {
    pub owner: Addr,
    pub pool_address: Addr,
    pub treasury_address: Addr,
    pub fury_token_address: Addr,
    pub lock_days: u64,
    pub discount: u64,
    pub usdc_denom: String,
    pub is_native_bonding: bool,
    pub tx_fee: u64,
    pub platform_fee: u64,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct BondStateResponse {
    pub address: Addr,
    pub list: Vec<BondingRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct AllBondStateResponse {
    pub list: Vec<BondStateResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct BondingRecord {
    pub amount: Uint128,
    pub timestamp: u64,
}
