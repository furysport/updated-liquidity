use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw20::Denom;
use cw_storage_plus::Item;

pub const LP_TOKEN: Item<Addr> = Item::new("lp_token");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Token {
    pub reserve: Uint128,
    pub denom: Denom,
}

pub const TOKEN1: Item<Token> = Item::new("token1");
pub const TOKEN2: Item<Token> = Item::new("token2");


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub staking_contract_address: Addr,
    pub staking_token_address: Addr,
    pub staking_funds_amount: Uint128
}

pub const CONFIG: Item<Config> = Item::new("config");