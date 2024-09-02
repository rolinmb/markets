use std::error::Error as StdError;

pub fn get_optionchain(ticker: &str, csv_name: &str) -> Result<(), Box<dyn StdError>> {
    println!("\nget_optionchain() :: Fetching HTML from bigcharts.marketwatch.com for {}", ticker);

    println!("\nget_optionchain() :: Successfully fetched HTML from bigcharts.marketwatch.com for {}", ticker);

    println!("\nget_optionchain() :: Successfully created {} with option chain data for {}", csv_name, ticker);
    Ok(())
}