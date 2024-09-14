use alloy::{
    primitives::{Address, U256},
    providers::Provider,
};
#[allow(unused_imports)]
use alloy_erc20::Erc20ProviderExt;
use alloy_erc20::LazyToken;

pub async fn get_percentage_token_supply<M: Provider>(
    provider: &M,
    token_address: Address,
    percentage: f64,
) -> alloy::primitives::Uint<256, 4> {
    let supported_precision = 8; // Support up to 8 decimal places
    let base: u64 = 10_u64.pow(supported_precision as u32);

    let token = LazyToken::new(token_address, provider);
    let total_supply = token.total_supply().await.unwrap();

    let percentage_scaled = (percentage * base as f64).round() as u64;

    let scaled_total_supply = total_supply * U256::from(percentage_scaled);
    let scaled_base = U256::from(base as u64 * 100); // Multiply base by 100 to handle 1.00% as 100

    let tokens_amount = scaled_total_supply / scaled_base;

    tokens_amount
}
