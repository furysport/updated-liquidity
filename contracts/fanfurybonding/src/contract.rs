#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, from_binary,
    WasmMsg, WasmQuery, QueryRequest, Order, Addr, CosmosMsg, QuerierWrapper, Storage
};
use cw2::{get_contract_version, set_contract_version};
use crate::util::Denom;
use cw_storage_plus::Bound;
use cw_utils::{maybe_addr};
use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, BondStateResponse, BondingRecord, AllBondStateResponse
};
use reqwest;
use serde::Deserialize;

struct CoinGeckoPriceResponse {
    #[serde(rename = "usd")]
    usd_price: f64,
    // Add other fields if needed
}

use crate::state::{
    Config, CONFIG, BONDING
};
use cw20::Balance;
use crate::util;
use crate::util::{NORMAL_DECIMAL, THOUSAND};
use wasmswap::msg::{QueryMsg as WasmswapQueryMsg, Token1ForToken2PriceResponse, Token2ForToken1PriceResponse};
// Version info, for migration info
const CONTRACT_NAME: &str = "fanfurybonding";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: msg.owner,
        pool_address: msg.pool_address,
        treasury_address: msg.treasury_address,
        fury_token_address: msg.fury_token_address,
        lock_days: msg.lock_days,
        discount: msg.discount,
        usdc_denom: msg.usdc_denom,
        is_native_bonding: msg.is_native_bonding,
        tx_fee: msg.tx_fee,
        platform_fee: msg.platform_fee,
        enabled: true,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwner{owner} => execute_update_owner(deps, env, info, owner),
        ExecuteMsg::UpdateEnabled{enabled} => execute_update_enabled(deps, env, info, enabled),
        ExecuteMsg::UpdateConfig{lock_days, discount, tx_fee, platform_fee} => execute_update_config(deps, env, info, lock_days, discount, tx_fee, platform_fee),
        ExecuteMsg::Bond {  } => execute_bond(deps, env, info),
        ExecuteMsg::LpBond {address, amount} => execute_lp_bond(deps, env, info, address, amount),
        ExecuteMsg::Unbond { index } => execute_unbond(deps, env, info, index),
        ExecuteMsg::Withdraw { amount } => execute_withdraw(deps, env, info, amount)
    }
}



pub fn check_enabled(
    storage: &mut dyn Storage,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(storage)?;
    if !cfg.enabled {
        return Err(ContractError::Disabled {})
    }
    Ok(Response::new().add_attribute("action", "check_enabled"))
}

pub fn check_owner(
    storage: &mut dyn Storage,
    address: Addr
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(storage)?;
    if cfg.owner != address {
        return Err(ContractError::Disabled {})
    }
    Ok(Response::new().add_attribute("action", "check_owner"))
}

pub fn execute_update_owner(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: Addr
) -> Result<Response, ContractError> {

    check_owner(deps.storage, info.sender.clone())?;

    let mut cfg = CONFIG.load(deps.storage)?;

    CONFIG.save(deps.storage, &cfg)?;

    return Ok(Response::new()
        .add_attributes(vec![
            attr("action", "update_owner"),
            attr("owner", owner),
        ]));
}


pub fn execute_update_enabled(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    enabled: bool
) -> Result<Response, ContractError> {

    check_owner(deps.storage, info.sender.clone())?;

    let mut cfg = CONFIG.load(deps.storage)?;

    cfg.enabled = enabled;
    CONFIG.save(deps.storage, &cfg)?;

    return Ok(Response::new()
        .add_attributes(vec![
            attr("action", "update_enabled"),
            attr("enabled", enabled.to_string()),
        ]));
}


pub fn execute_update_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lock_days: u64,
    discount: u64,
    tx_fee: u64,
    platform_fee: u64
) -> Result<Response, ContractError> {
    check_owner(deps.storage, info.sender.clone())?;

    let mut cfg = CONFIG.load(deps.storage)?;

    cfg.lock_days = lock_days;
    cfg.discount = discount;
    cfg.tx_fee = tx_fee;
    cfg.platform_fee = platform_fee;
    CONFIG.save(deps.storage, &cfg)?;

    return Ok(Response::new()
        .add_attributes(vec![
            attr("action", "update_config"),
            attr("lock_days", lock_days.to_string()),
            attr("discount", discount.to_string()),
        ]));
}



pub fn execute_bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {

    check_enabled(deps.storage)?;


    let cfg = CONFIG.load(deps.storage)?;

    if !cfg.is_native_bonding {
        return Err(ContractError::NotAllowedBondingType {  })
    }

    let balance = Balance::from(info.funds);

    let usdc_amount = util::get_amount_of_denom(balance, Denom::Native(cfg.usdc_denom.clone()))?;

    if usdc_amount == Uint128::zero() {
        return Err(ContractError::NativeInputZero {  })
    }
    let fee_amount = usdc_amount * Uint128::from(THOUSAND - cfg.tx_fee - cfg.platform_fee) / Uint128::from(THOUSAND);
    let real_amount = usdc_amount - fee_amount;

    let token2_price_response: Token1ForToken2PriceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: cfg.pool_address.clone().into(),
        msg: to_binary(&WasmswapQueryMsg::Token1ForToken2Price {
            token1_amount: real_amount
        })?,
    }))?;

    let receiving_amount = token2_price_response.token2_amount * Uint128::from(THOUSAND) / Uint128::from(THOUSAND - cfg.discount);

    let mut messages:Vec<CosmosMsg> = vec![];
    messages.push(util::transfer_token_message(Denom::Native(cfg.usdc_denom.clone()), usdc_amount, cfg.treasury_address.clone())?);

    let mut list:Vec<BondingRecord> = BONDING.load(deps.storage, info.sender.clone()).unwrap_or(vec![]);
    list.push(BondingRecord {
        amount: receiving_amount,
        timestamp: env.block.time.seconds() + 86400 * cfg.lock_days
    });
    BONDING.save(deps.storage, info.sender.clone(), &list)?;


    return Ok(Response::new()
        .add_attributes(vec![
            attr("action", "bond"),
            attr("bond_usdc_amount", real_amount),
            attr("receiving_amount", receiving_amount),
            attr("address", info.sender.clone()),
        ]));
}


pub fn execute_lp_bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: Addr,
    amount: Uint128
) -> Result<Response, ContractError> {

    check_enabled(deps.storage)?;

    let cfg = CONFIG.load(deps.storage)?;

    if info.sender.clone() != cfg.pool_address {
        return Err(ContractError::Unauthorized {  });
    }

    if amount == Uint128::zero() {
        return Err(ContractError::Cw20InputZero {  })
    }

    if cfg.is_native_bonding {
        return Err(ContractError::NotAllowedBondingType {  })
    }

    // On lp bonding, the platform fee and tx fee is already stolen from swap contract
    let receiving_amount = amount * Uint128::from(THOUSAND) / Uint128::from(THOUSAND - cfg.discount);
    let mut list:Vec<BondingRecord> = BONDING.load(deps.storage, info.sender.clone()).unwrap_or(vec![]);
    list.push(BondingRecord {
        amount: receiving_amount,
        timestamp: env.block.time.seconds() + 86400 * cfg.lock_days
    });
    BONDING.save(deps.storage, info.sender.clone(), &list)?;

    return Ok(Response::new()
        .add_attributes(vec![
            attr("action", "lp_bond"),
            attr("bond_fury_amount", amount),
            attr("receiving_amount", receiving_amount),
            attr("address", info.sender.clone()),
        ]));
}

pub fn execute_unbond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    index: u64
) -> Result<Response, ContractError> {

    check_enabled(deps.storage)?;

    let cfg = CONFIG.load(deps.storage)?;

    let mut list = BONDING.load(deps.storage, info.sender.clone())?;

    if list.len() >= index as usize {
        return Err(ContractError::InvalidInput {  })
    }

    let (_index, record) =list.iter().enumerate().find(|(i, c)| i == &(index as usize)).unwrap();

    // let record = list.clone().get(index as usize).unwrap();
    if record.timestamp > env.block.time.seconds() {
        return Err(ContractError::StillInBonding {})
    }

    let balance = Balance::from(info.funds);

    //calculate tx fee
    let usdc_amount = util::get_amount_of_denom(balance, Denom::Native(cfg.usdc_denom.clone()))?;
    let token2_price_response: Uint128 = query_native_token_price(real_amount)?;

// Example function for querying native token price
    fn query_native_token_price(amount: Uint128) -> StdResult<Uint128> {
    // Replace "YOUR_FANFURY_API_KEY" with your actual FanFury API key
    let api_key = "YOUR_FANFURY_API_KEY";
    let token_symbol = "FURY";  // Update with your native token symbol

    // Build the URL for the CoinGecko API request
    let url = format!("https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd", token_symbol);

    // Make the HTTP request
    let response = reqwest::blocking::get(&url)
        .map_err(|_| ContractError::ApiRequestFailed)?;

    // Parse the response JSON
    let price_response: CoinGeckoPriceResponse = response.json().map_err(|_| ContractError::ApiParsingFailed)?;

    // Calculate the uFury price based on the amount and the retrieved USD price
    let usd_price = price_response.usd_price;
    let fury_price = (usd_price * amount.u128() as f64) as u128;

    Ok(Uint128::from(fury_price))
    }
    if usdc_amount < token1_price_response.token1_amount * Uint128::from(cfg.platform_fee + cfg.tx_fee) / Uint128::from(THOUSAND) {
        return Err(ContractError::InsufficientFee { })
    }

    let fury_balance = util::get_native_balance(&cfg.usdc_denom, &env.contract.address)?;
    if fury_balance < record.amount {
        return Err(ContractError::InsufficientFury {})
    }

    let mut messages:Vec<CosmosMsg> = vec![];
    messages.push(util::transfer_token_message(Denom::Cw20(cfg.fury_token_address.clone()), record.amount, info.sender.clone())?);
    messages.push(util::transfer_token_message(Denom::Native(cfg.usdc_denom.clone()), usdc_amount, cfg.treasury_address.clone())?);

    let mut list = BONDING.load(deps.storage, info.sender.clone())?;
    list.remove(index as usize);
    BONDING.save(deps.storage, info.sender.clone(), &list)?;

    return Ok(Response::new()
        .add_attributes(vec![
            attr("action", "unbond"),
            attr("receiving_amount", record.amount),
            attr("address", info.sender.clone()),
        ]));
}

pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128
) -> Result<Response, ContractError> {

    check_owner(deps.storage, info.sender.clone())?;

    let cfg = CONFIG.load(deps.storage)?;

    let fury_balance = util::get_native_balance(&cfg.usdc_denom, &env.contract.address)?;
    if fury_balance < record.amount {
        return Err(ContractError::InsufficientFury {})
    }

    let mut messages:Vec<CosmosMsg> = vec![];
    messages.push(util::transfer_token_message(Denom::Cw20(cfg.fury_token_address.clone()), amount, info.sender.clone())?);

    return Ok(Response::new()
        .add_attributes(vec![
            attr("action", "withdraw"),
            attr("receiving_amount", amount),
            attr("address", info.sender.clone()),
        ]));
}



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {}
            => to_binary(&query_config(deps)?),
        QueryMsg::BondState {address}
            => to_binary(&query_bond_state(deps, address)?),
        QueryMsg::AllBondState {start_after, limit}
            => to_binary(&query_all_bond_state(deps, start_after, limit)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: cfg.owner,
        pool_address: cfg.pool_address,
        treasury_address: cfg.treasury_address,
        fury_token_address: cfg.fury_token_address,
        lock_days: cfg.lock_days,
        discount: cfg.discount,
        usdc_denom: cfg.usdc_denom,
        is_native_bonding: cfg.is_native_bonding,
        tx_fee: cfg.tx_fee,
        platform_fee: cfg.platform_fee,
        enabled: cfg.enabled
    })
}

pub fn query_bond_state(deps: Deps, address: Addr) -> StdResult<BondStateResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    let list = BONDING.load(deps.storage, address.clone()).unwrap_or(vec![]);

    Ok(BondStateResponse {
        address,
        list
    })
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn map_bonding(
    item: StdResult<(Addr, Vec<BondingRecord>)>,
) -> StdResult<BondStateResponse> {
    item.map(|(address, list)| {
        BondStateResponse {
            address,
            list
        }
    })
}


fn query_all_bond_state(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllBondStateResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let addr = maybe_addr(deps.api, start_after)?;
    let start = addr.map(|addr| Bound::exclusive(addr));

    let list:StdResult<Vec<_>> = BONDING
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| map_bonding(item))
        .collect();

    Ok(AllBondStateResponse { list: list? })
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }
    Ok(Response::default())
}

