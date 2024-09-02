use csv::{Writer};
use reqwest::Error;
use select::document::Document;
use select::predicate::{Class, Name};
use std::fs::File;
use std::error::Error as StdError;
use std::collections::{HashMap, HashSet};

const URLP1: &str = "https://www.finviz.com/quote.ashx?t=";
const URLP2: &str = "&p=d";

fn fetch_html(url: &str) -> Result<String, Error> {
    let response = reqwest::blocking::get(url)?.text()?;
    Ok(response)
}

fn parse_html_table(html: &str) -> Result<Vec<(String, String)>, Box<dyn StdError>> {
    let mut data = Vec::new();
    for tr in Document::from(html)
        .find(Class("js-snapshot-table"))
        .filter(|n| n.attr("class").unwrap_or("").contains("snapshot-table2"))
        .filter(|n| n.attr("class").unwrap_or("").contains("screener_snapshot-table-body")) {
        //println!("parse_html_table() :: \n<tr>{}</tr>\n", tr.html());
        let mut i = 0;
        let mut label = String::new();
        for td in tr.find(Name("td")) {
        if i == 0 {
            //print!("{} = ", td.text());
            label = td.text().trim().to_string();
            i = 1;
        } else {
            let raw_value = td.text();
            let final_value = if raw_value.trim().is_empty() || raw_value == "-" {
            "N/A".to_string()
            } else {
            raw_value.to_string()
            };
            //print!("{}\n", final_value);
            data.push((label.clone(), final_value));
            i = 0;
        }
        }
    }
    Ok(data)
}

/*fn finviz_data_from_csv(csv_name: &str) -> Result<HashMap<String, String>, Box<dyn StdError>> {
    let mut reader = Reader::from_path(csv_name)?;
    let mut data_map = HashMap::new();
    for result in reader.records() {
        let record = result?;
        let key = record.get(0).unwrap().to_string();
        let val = record.get(1).unwrap().to_string();
        data_map.insert(key, val);
    }
    Ok(data_map)
}*/

fn parse_finval(value: &str) -> Result<f64, std::num::ParseFloatError> {
    if let Some(pos) = value.find('B') {
        value[..pos].parse::<f64>().map(|v| v * 1_000_000_000.0)
    } else if let Some(pos) = value.find('M') {
        value[..pos].parse::<f64>().map(|v| v * 1_000_000.0)
    } else if let Some(pos) = value.find('T') {
        value[..pos].parse::<f64>().map(|v| v * 1_000_000_000_000.0)
    } else {
        value.parse::<f64>()
    }
}

fn compute_additional_financials(data: &HashMap<String, String>) -> HashMap<String, String> {
    let mut metrics = HashMap::new();
    if let (Some(dividend), Some(price)) = (data.get("Dividend TTM"), data.get("Price")) {
        if let (Ok(dividend), Ok(price)) = (dividend.parse::<f64>(), price.parse::<f64>()) {
            if price != 0.0 {
            let dividend_yield = (dividend / price) * 100.0;
            metrics.insert("Dividend Yield".to_string(), format!("{:.2}%", dividend_yield));
            }
        }
    }
    if let (Some(dividend), Some(eps)) = (data.get("Dividend TTM"), data.get("EPS (ttm)")) {
        if let (Ok(dividend), Ok(eps)) = (dividend.parse::<f64>(), eps.parse::<f64>()) {
            if eps != 0.0 {
            let payout_ratio = (dividend / eps) * 100.0;
            metrics.insert("Dividend Payout Ratio".to_string(), format!("{:.2}%", payout_ratio));
            }
        }
    }
    if let (Some(sales), Some(total_assets)) = (data.get("Sales"), data.get("Market Cap")) {
        if let (Ok(sales), Ok(total_assets)) = (parse_finval(sales), parse_finval(total_assets)) {
            if total_assets != 0.0 {
            let asset_turnover = sales / total_assets;
            metrics.insert("Asset Turnover Ratio (ATR)".to_string(), format!("{:.2}", asset_turnover));
            }
        }
    }
    if let (Some(net_income), Some(total_assets)) = (data.get("Income"), data.get("Market Cap")) {
        if let (Ok(net_income), Ok(total_assets)) = (parse_finval(net_income), parse_finval(total_assets)) {
            if total_assets != 0.0 {
            let rota = (net_income / total_assets) * 100.0;
            metrics.insert("Return on Total Assets (ROTA)".to_string(), format!("{:.2}%", rota));
            }
        }
    }
    if let (Some(p_fcf), Some(market_cap)) = (data.get("P/FCF"), data.get("Market Cap")) {
        if let (Ok(p_fcf), Ok(market_cap)) = (p_fcf.parse::<f64>(), parse_finval(market_cap)) {
            if p_fcf != 0.0 {
            let free_cash_flow_yield = 100.0 / p_fcf;
            metrics.insert("Free Cash Flow Yield (FCFY)".to_string(), format!("{:.2}%", free_cash_flow_yield));
            }
        }
    }
    if let (Some(eps), Some(price)) = (data.get("EPS (ttm)"), data.get("Price")) {
        if let (Ok(eps), Ok(price)) = (eps.parse::<f64>(), price.parse::<f64>()) {
            if price != 0.0 {
            let earnings_yield = (eps / price) * 100.0;
            metrics.insert("Earnings Yield (EPS/Share Price)".to_string(), format!("{:.2}%", earnings_yield));
            }
        }
    }
    metrics
}

pub fn fetch_finviz_info(ticker: &str, csv_name: &str) -> Result<(), Box<dyn StdError>> {
    let ticker = ticker.to_uppercase();
    let mut all_data: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut all_labels: HashSet<String> = HashSet::new();
    let full_url = format!("{}{}{}", URLP1, ticker, URLP2);
    println!("\nfetch_finviz_info() :: Fetching HTML from finviz.com for {}", ticker);
    match fetch_html(&full_url) {
        Ok(full_html) => {
            println!("\nfetch_finviz_info() :: Successfully fetched finviz.com HTML for {}", ticker);
            match parse_html_table(&full_html) {
                Ok(data) => {
                    let data_map: HashMap<String, String> = data.into_iter().collect();
                    let af = compute_additional_financials(&data_map);
                    let mut combined_data = data_map.clone();
                    combined_data.extend(af);
                    for key in combined_data.keys() {
                        all_labels.insert(key.clone());
                    }
                    all_data.insert(ticker.clone(), combined_data);
                },
                Err(e) => {
                    eprintln!("\nfetch_finviz_info() :: ERROR -> Failed to parse finviz.com HTML table for {}: {}", ticker, e);
                    return Ok(());
                },
            }
        },
        Err(e) => {
            eprintln!("\nfetch_finviz_info() :: ERROR -> Failed to fetch finviz.com HTML for {}:\n\n{}\n", ticker, e);
            return Ok(());
        },
    }
    let mut writer = Writer::from_writer(File::create(csv_name)?);
    let mut headers = vec!["Label".to_string()];
    headers.push(ticker.clone());
    writer.write_record(&headers)?;
    for label in &all_labels {
        let mut record = vec![label.clone()];
        let value = all_data
            .get(&ticker)
            .and_then(|data| data.get(label))
            .cloned()
            .unwrap_or_else(|| "N/A".to_string());
        record.push(value);
        writer.write_record(&record)?;
    }
    writer.flush()?;
    println!("\nfetch_finviz_info() :: Successfully created {} with financial filing data for {}", csv_name, ticker);
    Ok(())
}