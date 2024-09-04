use chrono::Local;
mod finviz;
use finviz::{fetch_finviz_info};
mod avantage;
use avantage::{get_underlying_av};
mod options;
use options::{fetch_option_chain};
mod graphing;
use graphing::{generate_tseries_plot, generate_surface_plot};
use std::env;
use std::process::exit;

const CSVDIR: &str = "csv_out/";
const IMGDIR: &str = "img_out/";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("\nmain() :: ERROR -> Please enter only one financial ticker/symbol (4 alphabetical characters) as a command line input argument after 'cargo run'\n\tExample: 'cargo run AAPL'\n");
        exit(1);
    }
    let ticker = &args[1];
    if ticker.chars().all(|c| c.is_alphabetic()) && ticker.len() <= 4 {
        let uticker = ticker.to_uppercase();
        let now = Local::now();
        let datetime_str = now.format("%Y-%m-%d_%H-%M-%S").to_string();
        let fv_csv = format!("{}{}_fv_{}.csv", CSVDIR, uticker, datetime_str);
        let av_csv = format!("{}{}_av_{}.csv", CSVDIR, uticker, datetime_str);
        let oc_csv = format!("{}{}_oc_{}.csv", CSVDIR, uticker, datetime_str);
        //let hist_png = format!("{}{}_history_{}.png", IMGDIR, uticker, datetime_str);
        let call_png = format!("{}{}_cprice_{}.png", IMGDIR, uticker, datetime_str);
        let put_png = format!("{}{}_pprice_{}.png", IMGDIR, uticker, datetime_str);
        let _ = fetch_finviz_info(&uticker, &fv_csv);
        let _ = get_underlying_av(&uticker, &av_csv);
        let _ = fetch_option_chain(&uticker, &oc_csv);
        let _ = generate_tseries_plot(&av_csv/*, &hist_png*/);
        let _ = generate_surface_plot(&oc_csv, &call_png, &put_png);
    } else {
        eprintln!("\nmain() :: ERROR -> Please enter a financial ticker/symbol that is at most 4 alphabetical characters; you entered '{}'", ticker);
        exit(1);
    }
}