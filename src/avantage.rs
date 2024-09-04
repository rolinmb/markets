use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use std::collections::BTreeMap;
use std::fs::File;
use std::f64;

const AVPATH: &str = "av_key.txt";
const AVBASEURL: &str = "https://www.alphavantage.co/query?function=TIME_SERIES_DAILY&symbol=";
const RVOLWINDOW: usize = 30;
const ATRPERIOD: usize = 14;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeSeries {
    #[serde(rename = "Meta Data")]
    pub meta_data: MetaData,
    #[serde(rename = "Time Series (Daily)")]
    pub ohlcv: BTreeMap<String, DayData>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MetaData {
    #[serde(rename = "1. Information")]
    information: String,
    #[serde(rename = "2. Symbol")]
    symbol: String,
    #[serde(rename = "3. Last Refreshed")]
    last_refreshed: String,
    #[serde(rename = "4. Output Size")]
    output_size: String,
    #[serde(rename = "5. Time Zone")]
    time_zone: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DayData {
    #[serde(rename = "1. open")]
    pub open: String,
    #[serde(rename = "2. high")]
    pub high: String,
    #[serde(rename = "3. low")]
    pub low: String,
    #[serde(rename = "4. close")]
    pub close: String,
    #[serde(rename = "5. volume")]
    pub volume: String,
}

fn daily_returns(t_series: &TimeSeries) -> Vec<f64> {
    let mut returns = Vec::new();
    let mut sorted_dates: Vec<_> = t_series.ohlcv.keys().collect();
    sorted_dates.sort();
    for i in 1..sorted_dates.len() {
        let prev_close = t_series.ohlcv[sorted_dates[i - 1]].close.parse::<f64>().unwrap_or(0.0);
        let cur_close = t_series.ohlcv[sorted_dates[i]].close.parse::<f64>().unwrap_or(0.0);
        returns.push((cur_close - prev_close) / prev_close);
    }
    returns
}

fn get_realized_vol(t_series: &TimeSeries, desired_window: usize) -> f64 {
    let returns = daily_returns(t_series);
    let available_window = returns.len().min(desired_window);
    if available_window < 2 {
        return 0.0;
    }
    let ssr: f64 = returns.iter().rev().take(available_window).map(|r| r.powi(2)).sum();
    let variance = ssr / (available_window as f64 - 1.0);
    variance.sqrt()
}

fn calculate_true_range(t_series: &TimeSeries) -> Vec<f64> {
    let mut true_ranges = Vec::new();
    let sorted_dates: Vec<_> = t_series.ohlcv.keys().collect();
    for i in 0..sorted_dates.len() {
        let day_data = &t_series.ohlcv[sorted_dates[i]];
        let high = day_data.high.parse::<f64>().unwrap_or(0.0);
        let low = day_data.low.parse::<f64>().unwrap_or(0.0);
        let close = day_data.close.parse::<f64>().unwrap_or(0.0);
        let prev_close = if i > 0 {
            t_series.ohlcv[sorted_dates[i - 1]].close.parse::<f64>().unwrap_or(0.0)
        } else {
            close
        };
        let tr = (high - low)
            .max((high - prev_close).abs())
            .max((low - prev_close).abs());

        true_ranges.push(tr);
    }
    true_ranges
}

fn get_avg_true_range(t_series: &TimeSeries, period: usize) -> f64 {
    let true_ranges = calculate_true_range(t_series);
    if true_ranges.len() < period {
        return 0.0;
    }
    let mut sum = 0.0;
    for i in 0..period {
        sum += true_ranges[i];
    }
    let mut atr = sum / period as f64;
    for i in period..true_ranges.len() {
        atr = (atr * (period as f64 - 1.0) + true_ranges[i]) / period as f64;
    }
    atr
}

/*fn get_stdev(t_series: &TimeSeries) -> f64 {
    let n = t_series.ohlcv.len();
    if n <= 1 {
        return 0.0;
    }
    let mut mean = 0.0;
    let mut m2 = 0.0;
    let mut count = 0;
    for day_data in t_series.ohlcv.values() {
        let close_price = day_data.close.parse::<f64>().unwrap_or(0.0);
        count += 1;
        let delta = close_price - mean;
        mean += delta / count as f64;
        m2 += delta * (close_price - mean);
    }
    (m2 / (n - 1) as f64).sqrt()
}*/

/*fn get_stdev(t_series: &TimeSeries) -> f64 {
    let n = t_series.ohlcv.len();
    if n <= 1 {
        return 0.0;
    }
    let sum: f64 = t_series.ohlcv.values().map(|day| day.close.parse::<f64>().unwrap_or(0.0)).sum();
    let mean = sum / n as f64;
    let sds: f64 = t_series.ohlcv.values().map(|day| {
        let diff = day.close.parse::<f64>().unwrap_or(0.0) - mean;
        diff * diff
    }).sum();
    let variance = sds / (n - 1) as f64;
    variance.sqrt()
}*/

fn ln_factorial(n: usize) -> f64 {
    (1..=n).map(|i| (i as f64).ln()).sum()
}

fn binomial_coefficient(n: usize, k: usize) -> f64 {
    (ln_factorial(n) - ln_factorial(k) - ln_factorial(n - k)).exp()
}

fn back_finite_diff(t_series: &TimeSeries, order: usize) -> f64 {
    let n = order.min(t_series.ohlcv.len() - 1);
    let dates: Vec<_> = t_series.ohlcv.keys().collect();
    let mut est = 0.0;
    for i in 0..=n {
        let coef = (-1.0_f64).powi(i as i32) * binomial_coefficient(n, i);
        let value = t_series.ohlcv[dates[dates.len() - (i + 1)]].close.parse::<f64>().unwrap_or(0.0);
        est += coef * value;
    }
    est
}

fn mean(data: &[f64]) -> f64 {
    let sum: f64 = data.iter().sum();
    sum / data.len() as f64
}

fn linear_regression(t_series: &TimeSeries, idx: usize) -> f64 {
    let data_len = t_series.ohlcv.len();
    if data_len == 0 {
        return 0.0;
    }
    let mut sorted_dates: Vec<_> = t_series.ohlcv.keys().collect();
    sorted_dates.sort();
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut price = 0.0;
    for i in (0..data_len).rev() {
        x.push((data_len - i) as f64);
        let value = t_series.ohlcv[sorted_dates[i]].close.parse::<f64>().unwrap_or(0.0);
        if i == idx {
            price = value;
        }
        y.push(value);
    }
    let mean_x = mean(&x);
    let mean_y = mean(&y);
    let m: f64 = x.iter().zip(&y).map(|(xi, yi)| (xi - mean_x) * (yi - mean_y)).sum();
    let b: f64 = x.iter().map(|xi| (xi - mean_x).powi(2)).sum();
    (m / b) * price + (mean_y - (m / b) * mean_x)
}

fn tseries_to_csv(t_series: &TimeSeries, filename: &str) -> Result<(), Box<dyn StdError>> {
    let file = File::create(filename)?;
    let mut writer = csv::WriterBuilder::new().from_writer(file);
    let _ = writer.write_record(&["LastRefreshed", "Date", "Open", "High", "Low", "Close", "Volume", "Change", "%Change", "Range", "AvgTrueRange", "RealizedVol", "FiniteDiff", "LinearReg"])?;
    let mut data_up_to_date = TimeSeries {
        meta_data: t_series.meta_data.clone(),
        ohlcv: BTreeMap::new(),
    };
    let sorted_dates: Vec<_> = t_series.ohlcv.keys().cloned().collect();
    for date in sorted_dates {
        if let Some(day_data) = t_series.ohlcv.get(&date) {
            data_up_to_date.ohlcv.insert(date.clone(), day_data.clone());
            let rvol = get_realized_vol(&data_up_to_date, RVOLWINDOW);
            let atr = get_avg_true_range(&data_up_to_date, ATRPERIOD);
            let finite_diff = back_finite_diff(&data_up_to_date, 1);
            let linear_reg = linear_regression(&data_up_to_date, data_up_to_date.ohlcv.len() - 1);
            let fclose: &f64 = &day_data.close.parse::<f64>().unwrap_or(0.0);
            let fopen: &f64 = &day_data.open.parse::<f64>().unwrap_or(0.0);
            let fhigh: &f64 = &day_data.high.parse::<f64>().unwrap_or(0.0);
            let flow: &f64 = &day_data.low.parse::<f64>().unwrap_or(0.0);
            writer.write_record(&[
                &t_series.meta_data.last_refreshed,
                &date,
                &day_data.open,
                &day_data.high,
                &day_data.low,
                &day_data.close,
                &day_data.volume,
                &(fclose - fopen).to_string(),
                &((fclose - fopen) / (fclose)).to_string(),
                &(fhigh - flow).to_string(),
                &atr.to_string(),
                &rvol.to_string(),
                &finite_diff.to_string(),
                &linear_reg.to_string(),
            ])?;
        }
    }
    writer.flush()?;
    Ok(())
}

pub fn get_underlying_av(ticker: &str, csv_name: &str) -> Result<(), Box<dyn StdError>> {
    let content = std::fs::read_to_string(AVPATH)?;
    let avkey = content.trim();
    if avkey.is_empty() {
        eprintln!("\nget_underlying_av() :: ERROR -> Alpha Vantage API Key not found; either {} is empty or missing", AVPATH);
        return Ok(());
    }
    let api_url = format!("{}{}&outputsize=full&apikey={}", AVBASEURL, ticker, avkey);
    println!("\nget_underlying_av() :: Fetching JSON from Alpha Vantage API for {}", ticker);
    let response = reqwest::blocking::get(&api_url)?;
    let t_series: TimeSeries = response.json()?;
    if t_series.meta_data.last_refreshed.is_empty() {
        eprintln!("\nget_underlying_av(): ERROR -> Alpha Vantage API request limit exceeded; either ticker/symbol {} does not exist, or some other error occurred", ticker);
        return Ok(());
    }
    println!("\nget_underlying_av() :: Successfully fetched JSON OHLCV data from Alpha Vantage for {}", ticker);
    let _ = tseries_to_csv(&t_series, csv_name);
    println!("\nget_underlying_av() :: Successfully created {} with time series data for {}", csv_name, ticker);
    Ok(())
}

pub fn tseries_from_csv(filename: &str) -> Result<TimeSeries, Box<dyn StdError>> {
    let file = File::open(filename)?;
    let mut reader = csv::Reader::from_reader(file);
    let mut ts = TimeSeries {
        meta_data: MetaData {
            information: String::new(),
            symbol: String::new(),
            last_refreshed: String::new(),
            output_size: String::new(),
            time_zone: String::new(),
        },
        ohlcv: BTreeMap::new(),
    };
    for (i, result) in reader.records().enumerate() {
        let record = result?;
        if i == 0 {
            ts.meta_data.last_refreshed = record[0].to_string();
        }
        let date = &record[1];
        ts.ohlcv.insert(
            date.to_string(),
            DayData {
                open: record[2].to_string(),
                high: record[3].to_string(),
                low: record[4].to_string(),
                close: record[5].to_string(),
                volume: record[6].to_string(),
            },
        );
    }
    Ok(ts)
}