use anyhow::{Context, Result};
use playwright::Playwright;
use std::time::Duration;
use tokio::time::sleep;

const OPTIONBASEURL: &str = "https://bigcharts.marketwatch.com/quickchart/options.asp?symb=";

struct Option {
    last: f64,
    change: f64,
    vol: f64,
    bid: f64,
    ask: f64,
    open_int: f64,
    strike: f64,
    yte: f64,
    is_call: bool,
}

struct OptionExpiry {
    date: String,
    yte: f64,
    calls: Vec<Option>,
    puts: Vec<Option>,
}

struct OptionChain {
    expiries: Vec<OptionExpiry>,
    ticekr: String,
    current_price: f64,
    div_yield: f64,
}

fn str_to_float(s: &str) -> f64 {
    s.replace(",", "")
        .parse::<f64>()
        .unwrap_or(0.0)
}

fn rand_int_range(min: u64, max: u64) -> u64 {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    rng.gen_range(min..=max)
}

#[tokio::main]
pub async fn get_optionchain(ticker: &str, csv_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::initialize().await.context("\nget_optionchain() :: ERROR -> Could not initialize Playwright")?;
    playwright.prepare()?;
    let browser = playwright.chromium().launcher().headless(true).launch().await.context("\nget_optionchain() :: ERROR -> Could not launch Chromium")?;
    let context = browser.context_builder().build().await?;
    let page = context.new_page().await?;
    let oc_url = format!("{}{}", OPTIONBASEURL, ticker);
    println!("\nget_optionchain() :: Fetching HTML from bigcharts.marketwatch.com for {}", ticker);
    page.goto_builder(&oc_url)
        .goto()
        .await
        .context("\nget_optionchain() :: ERROR -> Could not navigate to bigcharts.marketwatch.com")?;
    println!("\nget_optionchain() :: Successfully navigated to bigcharts.marketwatch.com for {}", ticker);
    let price_str = page
        .query_selector(".fright .price")
        .await
        .context("\nget_optionchain() :: ERROR -> Could not locate div of current underlying price from bigcharts.marketwatch.com HTML response")?
        .ok_or_else(|| anyhow::anyhow!("\nget_optionchain() :: ERROR -> Price element not found"))?
        .inner_text()
        .await
        .context("\nget_optionchain() :: ERROR -> Could not parse current price of underlying from bigcharts.marketwatch.com HTML response")?;
    let current_price = str_to_float(&price_str);
    let yield_str = page
        .query_selector("td.label:has-text('Yield:') + td.aright")
        .await
        .context("\nget_optionchain() :: ERROR -> Could not locate div of underlying yield from bigcharts.marketwatch.com HTML response")?
        .ok_or_else(|| anyhow::anyhow!("\nget_optionchain() :: ERROR -> Yield element not found"))?
        .inner_text()
        .await
        .context("\nget_optionchain() :: ERROR -> Could not parse underlying dividend yield from bigcharts.marketwatch.com HTML response")?;
    let mut yield_val = 0.0;
    let mut yield_display = "0".to_string();
    if yield_str.trim().to_lowercase() != "n/a" {
        let cleaned_yield = yield_str.replace("%", "");
        yield_val = cleaned_yield
            .parse::<f64>()
            .context("\nget_optionchain() :: ERROR -> An error occured while parsing underling dividend yield from HTML")?
            / 100.0;
        yield_display = cleaned_yield;
    }
    println!("\nget_optionchain() :: Current {} Price: ${:.2}; Dividend Yield: {:.4} ({}%)", ticker, current_price, yield_val, yield_display);
    let sleep_duration = Duration::from_millis(rand_int_range(1000, 2000));
    sleep(sleep_duration).await;
    println!("\nget_optionchain() :: Sleeping {:?}", sleep_duration);
    let toggles = page
        .query_selector_all("table.optionchain tr.optiontoggle td.caption form.ajaxpartial")
        .await
        .context("\nget_optionchain() :: ERROR -> Could not get all toggle elements from bigcharts.marketwatch.com HTML response")?;
    if toggles.len() > 1 {
        for (i, toggle) in toggles.into_iter().enumerate().skip(1) {
            toggle.click_builder().click().await.context("\nget_optionchain() :: ERROR -> Could not click toggle")?;
            let sleep_dur = Duration::from_millis(rand_int_range(1500, 3000));
            println!("\nget_optionchain() :: Toggle {} completed, sleeping {:?}", i, sleep_dur);
            sleep(sleep_dur).await;
        }
    }
    let final_sleep = Duration::from_millis(rand_int_range(100000, 110000));
    println!("\nget_optionchain() :: All HTML page toggles completed; sleeping {:?}", final_sleep);
    sleep(final_sleep).await;
    /*let rows = page
        .query_selector_all("table.optionchain tr.chainrow")
        .await
        .context("\nget_optionchain() :: Could not get option chain HTML table rows")?;*/
    // TODO: Parse the HTML for the option chain data then write to csv_name when finished

    browser.close().await?;
    //println!("\nget_optionchain() :: Successfully created {} with option chain data for {}", csv_name, ticker);
    Ok(())
}