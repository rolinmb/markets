use anyhow::{Context, Result};
use playwright::Playwright;
use csv::ReaderBuilder;
use super::finmath::{cnd, npd, brentq, black_scholes};
use super::utils::str_to_float;
use std::fs::File;
use std::error::Error;

const OURLP1: &str = "https://bigcharts.marketwatch.com/quickchart/options.asp?symb=";
const OURLP2: &str = "&showAll=True";
//const HTMLDIR: &str = "html_out/";

#[derive(Debug, Clone)]
pub struct Option {
    pub last: f64,
    pub change: f64,
    pub vol: f64,
    pub bid: f64,
    pub ask: f64,
    pub open_int: f64,
    pub strike: f64,
    pub yte: f64,
    pub is_call: bool,
}

impl Option {
    /*pub fn new(
        last: f64,
        change: f64,
        vol: f64,
        bid: f64,
        ask: f64,
        open_int: f64,
        strike: f64,
        yte: f64,
        is_call: bool,
    ) -> Self {
        Option {
            last: last,
            change: change,
            vol: vol,
            bid: bid,
            ask: ask,
            open_int: open_int,
            strike: strike,
            yte: yte,
            is_call: is_call,
        }
    }*/
    pub fn get_imp_vol(&self, s: f64, q: f64) -> f64 {
        let f = |x: f64| black_scholes(x, s, self.strike, self.yte, q, self.is_call) - self.last;
        match brentq(f, 0.0, 15.0, 1e-6) {
            Ok(iv) => iv,
            Err(_) => 0.0,
        }
    }
    pub fn get_delta(&self, q: f64, d1: f64) -> f64{
        if self.is_call {
            (-1.0 * q * self.yte).exp() * cnd(d1)
        } else {
            -1.0 * (-1.0 * q * self.yte) * cnd(d1)
        }
    }
    pub fn get_elasticity(&self, s: f64, delta: f64) -> f64 {
        delta * (s / self.last)
    }
    pub fn get_vega(&self, d2: f64) -> f64 {
        let vega = self.strike * npd(d2) * self.yte.sqrt();
        if vega.is_nan() {
            0.0
        } else {
            vega
        }
    }
    pub fn get_theta(&self, iv: f64, s: f64, q: f64, d1: f64, d2: f64, fed_funds: f64) -> f64 {
        if self.is_call {
            let ctheta = (-1.0 * q * self.yte).exp()
                * ((s * npd(d1) * iv) / (2.0 * self.yte.sqrt()))
                - fed_funds * self.strike * (-1.0 * fed_funds * self.yte).exp() * cnd(d2)
                + q * s * (-1.0 * q * self.yte).exp() * cnd(d1);

            if ctheta.is_nan() {
                0.0
            } else {
                ctheta
            }
        } else {
            let ptheta = (-1.0 * q * self.yte).exp()
                * ((s * npd(d1) * iv) / (2.0 * self.yte.sqrt()))
                + fed_funds * self.strike * (-1.0 * fed_funds * self.yte).exp() * cnd(-d2)
                - q * s * (-1.0 * q * self.yte).exp() * cnd(-d1);

            if ptheta.is_nan() {
                0.0
            } else {
                ptheta
            }
        }
    }
    pub fn get_rho(&self, d2: f64, fed_funds: f64) -> f64 {
        if self.is_call {
            let c_rho = self.strike * self.yte * (-1.0 * fed_funds * self.yte).exp() * cnd(d2);
            if c_rho.is_nan() {
                0.0
            } else {
                c_rho
            }
        } else {
            let p_rho = -1.0 * self.strike * self.yte * (-1.0 * fed_funds * self.yte).exp() * cnd(-d2);
            if p_rho.is_nan() {
                0.0
            } else {
                p_rho
            }
        }
    }
    pub fn get_epsilon(&self, s: f64, q: f64, d1: f64) -> f64 {
        if self.is_call {
            let c_eps = -1.0 * s * self.strike * self.yte * (-1.0 * q * self.yte).exp() * cnd(d1);
            if c_eps.is_nan() {
                0.0
            } else {
                c_eps
            }
        } else {
            let p_eps = s * self.yte * (-1.0 * q * self.yte).exp() * cnd(-d1);
            if p_eps.is_nan() {
                0.0
            } else {
                p_eps
            }
        }
    }
    pub fn get_gamma(&self, iv: f64, s: f64, d2: f64, fed_funds: f64) -> f64 {
        let gamma = self.strike * (-1.0 * fed_funds * self.yte).exp() * (npd(d2) / (s * s * iv * self.yte.sqrt()));
        if gamma.is_nan() {
            0.0
        } else {
            gamma
        }
    }
    pub fn get_vanna(&self, iv: f64, vega: f64, s: f64, d1: f64) -> f64 {
        let vanna = (vega / s) * (1.0 - (d1 / (iv * self.yte.sqrt())));
        if vanna.is_nan() {
            0.0
        } else {
            vanna
        }
    }
    pub fn get_charm(&self, iv: f64, q: f64, d1: f64, d2: f64, fed_funds: f64) -> f64 {
        if self.is_call {
            let c_charm = (q * (-1.0 * q * self.yte).exp() * cnd(d1))
                - ((-1.0 * q * self.yte).exp() * npd(d1) * ((2.0 * (fed_funds - q) * self.yte) - (d2 * iv * self.yte.sqrt()))
                / (2.0 * self.yte * iv * self.yte.sqrt()));
            if c_charm.is_nan() {
                0.0
            } else {
                c_charm
            }
        } else {
            let p_charm = (-1.0 * q * (-1.0 * q * self.yte).exp() * cnd(-d1))
                - ((-1.0 * q * self.yte).exp() * npd(d1) * ((2.0 * (fed_funds - q) * self.yte) - (d2 * iv * self.yte.sqrt()))
                / (2.0 * self.yte * iv * self.yte.sqrt()));
            if p_charm.is_nan() {
                0.0
            } else {
                p_charm
            }
        }
    }
    pub fn get_vomma(&self, iv: f64, vega: f64, d1: f64, d2: f64) -> f64 {
        let vomma = (vega * d1 * d2) / iv;
        if vomma.is_nan() {
            0.0
        } else {
            vomma
        }
    }
    pub fn get_veta(&self, iv: f64, s: f64, q: f64, d1: f64, d2: f64, fed_funds: f64) -> f64 {
        let factor = -1.0 * s * (-1.0 * q * self.yte).exp() * npd(d1) * self.yte.sqrt();
        let veta = factor * (q + (((fed_funds - q) * d1) / (iv * self.yte.sqrt()))
            - ((1.0 + (d1 * d2)) / (2.0 * self.yte)));
        if veta.is_nan() {
            0.0
        } else {
            veta
        }
    }
    pub fn get_speed(&self, iv: f64, gamma: f64, s: f64, d1: f64) -> f64 {
        let speed = (-1.0 * gamma / s) * ((d1 / (iv * self.yte.sqrt())) + 1.0);
        if speed.is_nan() {
            0.0
        } else {
            speed
        }
    }
    pub fn get_zomma(&self, iv: f64, gamma: f64, d1: f64, d2: f64) -> f64 {
        let zomma = gamma * (((d1 * d2) - 1.0) / iv);
        if zomma.is_nan() {
            0.0
        } else {
            zomma
        }
    }
    pub fn get_color(&self, iv: f64, s: f64, q: f64, d1: f64, d2: f64, fed_funds: f64) -> f64 {
        let factor1 = -1.0 * (-1.0 * q * self.yte).exp() * npd(d1) / (2.0 * s * self.yte * iv * self.yte.sqrt());
        let factor2 = (((2.0 * (fed_funds - q) * self.yte) - (d2 * iv * self.yte.sqrt()))
            / (iv * self.yte.sqrt())) * d1;
        let color = factor1 * ((2.0 * q * self.yte) + 1.0 + factor2);
        if color.is_nan() {
            0.0
        } else {
            color
        }
    }
    pub fn get_ultima(&self, iv: f64, vega: f64, d1: f64, d2: f64) -> f64 {
        let factor = (-1.0 * vega) / (iv * iv);
        let ultima = factor * (((d1 * d2) * (1.0 - (d1 * d2))) + (d1 * d1) + (d2 * d2));
        if ultima.is_nan() {
            0.0
        } else {
            ultima
        }
    }
}

impl Default for Option {
    fn default() -> Self {
        Option {
            last: 0.0,
            change: 0.0,
            vol: 0.0,
            bid: 0.0,
            ask: 0.0,
            open_int: 0.0,
            strike: 0.0,
            yte: 0.0,
            is_call: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptionExpiry {
    pub date: String,
    pub yte: f64,
    pub calls: Vec<Option>,
    pub puts: Vec<Option>,
}

#[derive(Debug, Clone)]
pub struct OptionChain {
    pub expiries: Vec<OptionExpiry>,
    pub ticker: String,
    pub current_price: f64,
    pub div_yield: f64,
}

impl OptionChain {
    pub fn total_contract_volume(&self) -> f64 {
        let mut sum = 0.0;
        for expiry in self.expiries.clone() {
            for call in expiry.calls {
                sum += call.vol;
            }
            for put in expiry.puts {
                sum += put.vol;
            }
        }
        sum
    }
    pub fn total_open_interest(&self) -> f64 {
        let mut sum = 0.0;
        for expiry in self.expiries.clone() {
            for call in expiry.calls {
                sum += call.open_int;
            }
            for put in expiry.puts {
                sum += put.open_int;
            }
        }
        sum
    }
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

pub fn chain_from_csv(csv_file: &str) -> Result<OptionChain, Box<dyn Error>> {
    let file = File::open(csv_file)?;
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(file);
    let mut expiries: Vec<OptionExpiry> = Vec::new();
    let mut current_expiry: OptionExpiry = OptionExpiry {
        date: String::new(),
        yte: 0.0,
        calls: Vec::new(),
        puts: Vec::new(),
    };
    let mut current_ticker: String = String::new();
    let current_price: f64 = 0.0;
    let div_yield: f64 = 0.0;
    for result in rdr.records() {
        let record = result?;
        if current_ticker.is_empty() {
            current_ticker = record[0].to_string();
        }
        let expiry_date = record[1].to_string();
        let strike = record[2].parse::<f64>()?;
        let is_call = record[3] == *"c";
        let last = record[4].parse::<f64>()?;
        let change = record[5].parse::<f64>()?;
        let vol = record[6].parse::<f64>()?;
        let bid = record[7].parse::<f64>()?;
        let ask = record[8].parse::<f64>()?;
        let open_int = record[9].parse::<f64>()?;
        let yte = record[10].parse::<f64>()?;
        if expiry_date != current_expiry.date && !current_expiry.date.is_empty() {
            expiries.push(current_expiry.clone());
            current_expiry = OptionExpiry {
                date: expiry_date.clone(),
                yte,
                calls: Vec::new(),
                puts: Vec::new(),
            };
        }
        let opt = Option {
            last: last,
            change: change,
            vol: vol,
            bid: bid,
            ask: ask,
            open_int: open_int,
            strike: strike,
            yte: yte,
            is_call: is_call,
        };
        if is_call {
            current_expiry.calls.push(opt);
        } else {
            current_expiry.puts.push(opt);
        }
    }
    if !current_expiry.calls.is_empty() || !current_expiry.puts.is_empty() {
        expiries.push(current_expiry);
    }
    let option_chain = OptionChain {
        expiries,
        ticker: current_ticker,
        current_price,
        div_yield,
    };
    Ok(option_chain)
}

/*pub fn get_atm_options(chain_csv_name: &str, cp_flag: bool) -> (Option, Option) {
    let chain = match chain_from_csv(chain_csv_name) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("\nget_atm_options() :: ERROR -> Failed to load option chain with chain_from_csv: {}", e);
            return (Option::default(), Option::default());
        }
    };
    let mut otm: Option = Option::default();
    let mut itm: Option = Option::default();
    if cp_flag {
        for (i, call) in chain.expiries[0].calls.iter().enumerate() {
            if call.strike > chain.current_price {
                otm = call.clone();
                itm = chain.expiries[0].calls[i - 1].clone();
                break;
            }
        }
    } else {
        for (j, put) in chain.expiries[0].puts.iter().enumerate() {
            if put.strike > chain.current_price {
                otm = chain.expiries[0].puts[j - 1].clone();
                itm = put.clone();
                break;
            }
        }
    }
    (otm, itm)
}

pub fn get_atm_straddle(chain_csv_name: &str) -> (f64, Option, Option) {
    let (atm_call, _otm_call) = get_atm_options(chain_csv_name, true);
    let (atm_put, _otm_put) = get_atm_options(chain_csv_name, false);
    let straddle_value = atm_call.last + atm_put.last;
    (straddle_value, atm_call, atm_put)
}

pub fn get_atm_credit_spread(chain_csv_name: &str, cp_flag: bool) -> (f64, Option, Option) {
    let (otm, itm) = get_atm_options(chain_csv_name, cp_flag);
    let credit_spread_value = itm.last - otm.last;
    (credit_spread_value, otm, itm)
}

pub fn get_atm_debit_spread(chain_csv_name: &str, cp_flag: bool) -> (f64, Option, Option) {
    let (otm, itm) = get_atm_options(chain_csv_name, cp_flag);
    let debit_spread_value = otm.last - itm.last;
    (debit_spread_value, otm, itm)
}*/