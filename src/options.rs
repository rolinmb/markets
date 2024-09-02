use anyhow::{Context, Result};
use playwright::Playwright;
use tokio::time::sleep;
use csv::WriterBuilder;
use std::time::Duration;
use std::fs::File;

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
    ticker: String,
    current_price: f64,
    div_yield: f64,
}

impl OptionChain {
    pub fn save_to_csv(&self, csv_name: &str) -> Result<(), anyhow::Error> {
        let file = File::create(csv_name).context(format!("\nsave_to_csv() :: ERROR -> Could not create CSV file '{}'", csv_name))?;
        let mut wtr = WriterBuilder::new().has_headers(true).from_writer(file);
        let headers = [
            "Ticker", "Expiration Date", "Yte",
            "Call Last", "Call Change", "Call Vol", "Call Bid", "Call Ask", "Call OpenInt",
            "Put Last", "Put Change", "Put Vol", "Put Bid", "Put Ask", "Put OpenInt",
            "Strike",
        ];
        wtr.write_record(&headers)
            .context(format!("\nsave_to_csv() :: ERROR -> Could not write headers to CSV file '{}'", csv_name))?;
        for expiry in &self.expiries {
            for (i, call) in expiry.calls.iter().enumerate() {
                let put = &expiry.puts[i];
                let csv_row = [
                    &self.ticker,
                    &expiry.date,
                    &format!("{:.6}", expiry.yte),
                    &format!("{:.2}", call.last),
                    &format!("{:.2}", call.change),
                    &format!("{:.0}", call.vol),
                    &format!("{:.2}", call.bid),
                    &format!("{:.2}", call.ask),
                    &format!("{:.0}", call.open_int),
                    &format!("{:.2}", put.last),
                    &format!("{:.2}", put.change),
                    &format!("{:.0}", put.vol),
                    &format!("{:.2}", put.bid),
                    &format!("{:.2}", put.ask),
                    &format!("{:.0}", put.open_int),
                    &format!("{:.2}", call.strike),
                ];
                wtr.write_record(&csv_row)
                    .context(format!("\nsave_to_csv() :: ERROR -> Could not write record to CSV file '{}'", csv_name))?;
            }
        }
        wtr.flush().context(format!("\nsave_to_csv() :: ERROR -> Could not flush CSV writer to file '{}'", csv_name))?;
        Ok(())
    }
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

fn remove_ordinal_suffix(s: &str) -> String {
    s.trim_end_matches(|c: char| c.is_ascii_alphabetic()).to_string()
}

#[tokio::main]
pub async fn get_optionchain(ticker: &str, csv_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::initialize()
        .await
        .context("\nget_optionchain() :: ERROR -> Could not initialize Playwright")?;
    playwright.prepare()?;
    let browser = playwright.chromium().launcher().headless(true).launch()
        .await
        .context("\nget_optionchain() :: ERROR -> Could not launch Chromium")?;
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
        .await?
        .ok_or_else(|| anyhow::anyhow!("\nget_optionchain() :: ERROR -> Price element not found"))?
        .inner_text()
        .await
        .context("\nget_optionchain() :: ERROR -> Could not parse current price of underlying from bigcharts.marketwatch.com HTML response")?;
    let current_price = str_to_float(&price_str);
    let yield_str = page
        .query_selector("td.label:has-text('Yield:') + td.aright")
        .await?
        .ok_or_else(|| anyhow::anyhow!("\nget_optionchain() :: ERROR -> Yield element not found"))?
        .inner_text()
        .await
        .context("\nget_optionchain() :: ERROR -> Could not parse underlying dividend yield from bigcharts.marketwatch.com HTML response")?;
    let mut yield_val = 0.0;
    let mut yield_display = "0".to_string();
    if yield_str.trim().to_lowercase() != "n/a" {
        let cleaned_yield = yield_str.replace("%", "");
        yield_val = cleaned_yield.parse::<f64>()
            .context("\nget_optionchain() :: ERROR -> An error occured while parsing underling dividend yield from HTML")? / 100.0;
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
            println!("\nget_optionchain() :: Toggle {} completed; sleeping {:?}", i, sleep_dur);
            sleep(sleep_dur).await;
        }
    }
    let final_sleep = Duration::from_millis(rand_int_range(1000, 1500));
    println!("\nget_optionchain() :: All HTML page toggles completed; sleeping {:?} before continuing", final_sleep);
    sleep(final_sleep).await;
    // TODO: program stops working around here
    let rows = page
        .query_selector_all("table.optionchain tr.chainrow")
        .await
        .context("\nget_optionchain() :: Could not get option chain HTML table rows")?;
    let mut chain = OptionChain {
        expiries: Vec::new(),
        ticker: ticker.to_string(),
        current_price: current_price,
        div_yield: yield_val,
    };
    let mut expiry = OptionExpiry {
        date: "".to_string(),
        yte: 0.0,
        calls: Vec::new(),
        puts: Vec::new(),
    };
    let current_time = chrono::Utc::now();
    let mut current_exp_date = "".to_string();
    let mut current_yte = 0.0;
    for tr in rows {
        let tr_text = tr.text_content().await.unwrap_or_default().expect("\nget_optionchain() :: ERROR ").trim().to_string();
        if tr_text.is_empty() || tr_text.contains("Stock Price »") || tr_text.contains("CALLS") || tr_text.contains("Last") || tr_text.contains("Show") {
            continue;
        }
        if tr_text.contains("Expires") {
            let date_fields: Vec<&str> = tr_text.split_whitespace().collect();
            if date_fields.len() < 4 {
                continue;
            }
            let mut cleaned_day = remove_ordinal_suffix(date_fields[1]);
            cleaned_day = cleaned_day.replace(",", "");
            let new_exp_date = format!("{} {} {}", &date_fields[0], cleaned_day, date_fields[2]);
            let parsed_time = chrono::NaiveDate::parse_from_str(&new_exp_date, "%b %d %Y")
                .context(format!("\nget_optionchain() :: ERROR -> A problem occured parsing new_exp_date '{}'", new_exp_date))?;
            let parsed_datetime = chrono::DateTime::<chrono::Utc>::from_utc(parsed_time.into(), chrono::Utc);
            let duration = current_time.signed_duration_since(parsed_datetime);
            let new_yte = (duration.num_hours().abs() as f64) / 24.0 / 252.0;
            if !current_exp_date.is_empty() && current_exp_date != new_exp_date {
                expiry.date = current_exp_date.clone();
                expiry.yte = current_yte;
                chain.expiries.push(expiry);
                expiry = OptionExpiry {
                    date: "".to_string(),
                    yte: 0.0,
                    calls: Vec::new(),
                    puts: Vec::new(),
                };
                println!("\nget_optionchain() :: Finished parsing expiration date {} (yte = {:.3})", current_exp_date, current_yte);
            }
            current_exp_date = new_exp_date;
            current_yte = new_yte;
            continue;
        }
        let td_cells = tr.query_selector_all("td").await?;
        let mut tr_data: Vec<f64> = Vec::new();
        for td in td_cells {
            let td_text = td.text_content().await.unwrap_or_default().expect("\nget_optionchain() :: ERROR ").trim().to_string();
            if td_text.is_empty() {
                tr_data.push(0.0);
                continue;
            }
            let mut num = 0.0;
            if let Some(first_field) = td_text.split_whitespace().next() {
                let cleaned_num = first_field.replace(",", "");
                num = cleaned_num.parse::<f64>().unwrap_or(0.0);
            }
            tr_data.push(num);
        }
        if tr_data.len() < 13 {
            continue;
        }
        let call = Option {
            last: tr_data[0],
            change: tr_data[1],
            vol: tr_data[2],
            bid: tr_data[3],
            ask: tr_data[4],
            open_int: tr_data[5],
            strike: tr_data[6],
            yte: current_yte,
            is_call: true,
        };
        let put = Option {
            last: tr_data[7],
            change: tr_data[8],
            vol: tr_data[9],
            bid: tr_data[10],
            ask: tr_data[11],
            open_int: tr_data[12],
            strike: tr_data[6],
            yte: current_yte,
            is_call: false,
        };
        expiry.calls.push(call);
        expiry.puts.push(put);
    }
    if !current_exp_date.is_empty() && !expiry.calls.is_empty() {
        expiry.date = current_exp_date.clone();
        expiry.yte = current_yte;
        chain.expiries.push(expiry);
    }
    browser.close()
        .await
        .context("\nget_optionchain() :: ERROR -> Could not close playwright chromium browser")?;
    chain.save_to_csv(csv_name)?;
    println!("\nget_optionchain() :: Successfully created {} with option chain data for {}", csv_name, ticker);
    Ok(())
}