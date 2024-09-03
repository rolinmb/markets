use anyhow::{Context, Result};
use playwright::Playwright;

const OURLP1: &str = "https://bigcharts.marketwatch.com/quickchart/options.asp?symb=";
const OURLP2: &str = "&showAll=True";
//const HTMLDIR: &str = "html_out/";

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
struct OptionExpiry {
    date: String,
    yte: f64,
    calls: Vec<Option>,
    puts: Vec<Option>,
}

#[derive(Debug, Clone)]
struct OptionChain {
    expiries: Vec<OptionExpiry>,
    ticker: String,
    current_price: f64,
    div_yield: f64,
}

fn str_to_float(s: &str) -> f64 {
    s.replace(",", "").parse::<f64>().unwrap_or(0.0)
}

#[tokio::main]
pub async fn fetch_option_chain(ticker: &str, csv_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let playwright = Playwright::initialize()
        .await
        .context("\nfetch_option_chain() :: ERROR -> Could not initialize Playwright")?;
    playwright.prepare()?;
    let browser = playwright.chromium().launcher().headless(true).launch()
        .await
        .context("\nfetch_option_chain() :: ERROR -> Could not launch Chromium")?;
    let context = browser.context_builder().build().await?;
    let page = context.new_page().await?;
    let oc_url = format!("{}{}{}", OURLP1, ticker, OURLP2);
    println!("\nfetch_option_chain() :: Fetching HTML from bigcharts.marketwatch.com for {}", ticker);
    page.goto_builder(&oc_url)
        .goto()
        .await
        .context("\nfetch_option_chain() :: ERROR -> Could not navigate to bigcharts.marketwatch.com")?;
    println!("\nfetch_option_chain() :: Successfully navigated to bigcharts.marketwatch.com for {}", ticker);
    page.wait_for_selector_builder("table.optionchain tr.chainrow:last-child")
        .wait_for_selector()
        .await
        .context("\nfetch_option_chain() :: ERROR -> Timed out waiting for the last row of the option chain table")?;
    /*let html_content = page.content().await?;
    let html_name = format!("{}{}.html", HTMLDIR, ticker);
    let mut html_file = File::create(&html_name)?;
    html_file.write_all(html_content.as_bytes())?;
    println!("\nfetch_option_chain() :: Successfully saved full HTML as {}", html_name);*/
    let price_str = page
        .query_selector(".fright .price")
        .await?
        .ok_or_else(|| anyhow::anyhow!("\nfetch_option_chain() :: ERROR -> Price element not found"))?
        .inner_text()
        .await
        .context("\nfetch_option_chain() :: ERROR -> Could not parse current price of underlying from bigcharts.marketwatch.com HTML response")?;
    let current_price = str_to_float(&price_str);
    let yield_str = page
        .query_selector("td.label:has-text('Yield:') + td.aright")
        .await?
        .ok_or_else(|| anyhow::anyhow!("\nfetch_option_chain() :: ERROR -> Yield element not found"))?
        .inner_text()
        .await
        .context("\nfetch_option_chain() :: ERROR -> Could not parse underlying dividend yield from bigcharts.marketwatch.com HTML response")?;
    let mut yield_val = 0.0;
    //let mut yield_display = "0".to_string();
    if yield_str.trim().to_lowercase() != "n/a" {
        let cleaned_yield = yield_str.replace("%", "");
        yield_val = cleaned_yield.parse::<f64>()
            .context("\nfetch_option_chain() :: ERROR -> An error occured while parsing underling dividend yield from HTML")? / 100.0;
        //yield_display = cleaned_yield;
    }
    //println!("\nfetch_option_chain() :: Current {} Price: ${:.2}; Dividend Yield: {:.4} ({}%)", ticker, current_price, yield_val, yield_display);
    let rows = page
        .query_selector_all("table.optionchain tr.chainrow")
        .await
        .context("\nfetch_option_chain() :: Could not get option chain HTML table rows")?;
    let mut chain = OptionChain {
        expiries: Vec::new(),
        ticker: ticker.to_string(),
        current_price: current_price,
        div_yield: yield_val,
    };
    let mut current_exp_date = "".to_string();
    let mut current_yte = 0.0;
    let mut expiry = OptionExpiry {
        date: "".to_string(),
        yte: 0.0,
        calls: Vec::new(),
        puts: Vec::new(),
    };
    let current_time = chrono::Utc::now();
    let mut i: i128 = 0;
    for tr in rows {
        i += 1;
        let tr_text_result = tr.text_content().await;
        if let Err(e) = &tr_text_result {
            eprintln!("\nfetch_option_chain() :: ERROR -> Could not get text content from <tr> element: {:?}", e);
            continue;
        }
        let tr_text = tr_text_result.unwrap_or_default().expect("\nfetch_option_chain() :: ERROR -> Could not unwrap <tr> element text content").trim().to_string();
        if tr_text.is_empty() || tr_text.contains("Stock Price »") || tr_text.contains("CALLS") || tr_text.contains("Last") || tr_text.contains("Show") {
            continue;
        }
        if tr_text.contains("Expires") {
            if !expiry.calls.is_empty() || !expiry.puts.is_empty() {
                expiry.date = current_exp_date.clone();
                expiry.yte = current_yte;
                chain.expiries.push(expiry.clone());
                //println!("\nfetch_option_chain() :: Finished parsing expiration date {} (yte = {:.3})", current_exp_date, current_yte);
                //println!("{:?}", expiry);
                expiry = OptionExpiry {
                    date: "".to_string(),
                    yte: 0.0,
                    calls: Vec::new(),
                    puts: Vec::new(),
                };
            }
            let date_fields: Vec<&str> = tr_text.split_whitespace().collect();
            //println!("\nfetch_option_chain() :: date_fields: {:?}", date_fields);
            if date_fields.len() < 4 {
                eprintln!("\nfetch_option_chain() :: ERROR -> Insufficient date fields: {:?}", date_fields);
                continue;
            }
            current_exp_date = format!("{} {} {}", &date_fields[1], date_fields[2].replace(",", ""), date_fields[3]);
            let parsed_time = match chrono::NaiveDate::parse_from_str(&current_exp_date, "%B %d %Y") {
                Ok(dt) => dt,
                Err(e) => {
                    eprintln!("\nfetch_option_chain() :: ERROR -> A problem occurred parsing new_exp_date '{}': {:?}", current_exp_date, e);
                    continue;
                },
            };
            let parsed_datetime = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(parsed_time.into(), chrono::Utc);
            let duration = current_time.signed_duration_since(parsed_datetime);
            current_yte = (duration.num_hours().abs() as f64) / 24.0 / 252.0;
            continue;
        }
        let td_cells_result = tr.query_selector_all("td").await;
        if let Err(e) = &td_cells_result {
            eprintln!("\nfetch_option_chain() :: ERROR -> Could not get table cells from <tr> element: {:?}", e);
            continue;
        }
        let td_cells = td_cells_result.unwrap_or_default();
        let mut tr_data: Vec<f64> = Vec::new();
        for td in td_cells {
            let td_text_result = td.text_content().await;
            if let Err(e) = &td_text_result {
                eprintln!("\nfetch_option_chain() :: ERROR -> Could not get text content from <td> element: {:?}", e);
                continue;
            }
            let td_text = td_text_result.unwrap_or_default().expect("\nfetch_option_chain() :: ERROR -> Could not unwrap <td> element text content").trim().to_string();
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
            eprintln!("\nfetch_option_chain() :: ERROR -> Insufficient data in tr_data {} by end of populating: {:?}", i, tr_data);
            continue;
        }
        //println!("\nfetch_option_chain() :: Exctracted <td> element data from <tr> row element {}: {:?}", i, tr_data);
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
        //println!("\nfetch_option_chain() :: <tr> {} Call Option {:?} and Put Option {:?}", i, call, put);
        expiry.calls.push(call);
        expiry.puts.push(put);
    }
    if !expiry.calls.is_empty() || !expiry.puts.is_empty() {
        expiry.date = current_exp_date.clone();
        expiry.yte = current_yte;
        chain.expiries.push(expiry.clone());
        //println!("\nfetch_option_chain() :: Finished parsing expiration date {} (yte = {:.3})", current_exp_date, current_yte);
        //println!("{:?}", expiry);
    }
    println!("\nfetch_option_chain() :: Successfully created OptionChain struct for {} from bigcharts.marketwatch.com", ticker);
    //prtinln!("{:?}", chain);
    browser.close()
        .await
        .context("\nfetch_option_chain() :: ERROR -> Could not close playwright chromium browser")?;
    if !chain.expiries.is_empty() {
        let mut wtr = csv::Writer::from_path(&csv_name).context("\nfetch_option_chain() :: ERROR -> Could not open file for CSV writing")?;
        wtr.write_record(&["UNDERLYING", "EXPIRATION", "STRIKE", "CALL(c)/PUT(p)", "LAST", "CHANGE", "VOLUME", "BID", "ASK", "OPENINT", "YTE"])?;
        for expiry in &chain.expiries {
            for call in &expiry.calls {
                wtr.write_record(&[
                    &chain.ticker,
                    &expiry.date,
                    &call.strike.to_string(),
                    "c",
                    &call.last.to_string(),
                    &call.change.to_string(),
                    &call.vol.to_string(),
                    &call.bid.to_string(),
                    &call.ask.to_string(),
                    &call.open_int.to_string(),
                    &call.yte.to_string(),
                ])?;
            }
            for put in &expiry.puts {
                wtr.write_record(&[
                    &chain.ticker,
                    &expiry.date,
                    &put.strike.to_string(),
                    "p",
                    &put.last.to_string(),
                    &put.change.to_string(),
                    &put.vol.to_string(),
                    &put.bid.to_string(),
                    &put.ask.to_string(),
                    &put.open_int.to_string(),
                    &put.yte.to_string(),
                ])?;
            }
        }
        wtr.flush().context("\nfetch_option_chain() :: ERROR -> Could not flush CSV writer")?;
        println!("\nfetch_option_chain() :: Successfully created {} with option chain data for {}", csv_name, ticker);
    } else {
        eprintln!("\nfetch_option_chain() :: ERROR -> Unsuccessful at parsing HTML into OptionChain struct; no csv output to be made");
    }
    Ok(())
}