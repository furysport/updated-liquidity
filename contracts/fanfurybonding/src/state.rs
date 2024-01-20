// state.rs

use cosmwasm_std::{Addr, Storage, Uint128};
use cw20::Denom;
use cw_storage_plus::Item;

// Config struct to store contract configuration
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Config {
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

// Item to store the Config struct in storage
pub const CONFIG: Item<Config> = Item::new("config");

// BondingRecord struct to represent individual bonding records
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct BondingRecord {
    pub amount: Uint128,
    pub timestamp: u64,
}

// BONDING mapping to store bonding records for each address
pub const BONDING: Item<Vec<BondingRecord>> = Item::new("bonding");

// Function to check if the contract is enabled
pub fn check_enabled(storage: &dyn Storage) -> Result<(), ContractError> {
    let cfg = CONFIG.load(storage)?;
    if !cfg.enabled {
        return Err(ContractError::Disabled {});
    }
    Ok(())
}

// Function to check if the sender is the owner of the contract
pub fn check_owner(storage: &dyn Storage, address: Addr) -> Result<(), ContractError> {
    let cfg = CONFIG.load(storage)?;
    if cfg.owner != address {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

// Enum to represent contract errors
#[derive(Debug, PartialEq, Eq)]
pub enum ContractError {
    Disabled,
    Unauthorized,
    // Define additional contract errors if needed
}

// Implement std::fmt::Display for ContractError
impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// Implement std::error::Error for ContractError
impl std::error::Error for ContractError {}
