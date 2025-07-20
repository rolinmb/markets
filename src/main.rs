use chrono::Local;
mod finviz;
use finviz::{fetch_finviz_info};
mod avantage;
use avantage::{get_underlying_av};
mod finmath;
mod options;
use options::{fetch_option_chain, chain_from_csv/*, get_atm_straddle, get_atm_credit_spread, get_atm_debit_spread*/};
mod graphing;
use graphing::{generate_tseries_plot, generate_surface_plot/*, plot_volatility_smiles*/};
mod utils;
use utils::{clear_directory_or_create, create_directory_if_dne};
use std::process::{Command, exit};
use std::env;
use std::str;

const CSVDIR: &str = "csv_out/";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("\nmain() :: ERROR -> Please enter only one financial ticker/symbol (4 alphabetical characters) as a command line input argument after 'cargo run'\n\tExample: 'cargo run AAPL'\n");
        exit(1);
    }
    let ticker = &args[1];
    if ticker.chars().all(|c| c.is_alphabetic()) && ticker.len() <= 4 {
        let _ = create_directory_if_dne("csv_out");
        let _ = create_directory_if_dne("pdf_out");
        let _ = create_directory_if_dne("img_out");
        let _ = create_directory_if_dne("dat_out");
        let _ = create_directory_if_dne("html_out");
        let uticker = ticker.to_uppercase();
        let now = Local::now();
        let datetime_str = now.format("%Y-%m-%d_%H-%M-%S").to_string();
        let fv_csv = format!("{}{}_fv_{}.csv", CSVDIR, uticker, datetime_str);
        let av_csv = format!("{}{}_av_{}.csv", CSVDIR, uticker, datetime_str);
        let oc_csv = format!("{}{}_oc_{}.csv", CSVDIR, uticker, datetime_str);
        let _ = fetch_finviz_info(&uticker, &fv_csv);
        let _ = get_underlying_av(&uticker, &av_csv);
        let _ = fetch_option_chain(&uticker, &oc_csv);
        for series_field in 0..11 {
            let _ = generate_tseries_plot(&av_csv, series_field);
        }
        for plot_field in 0..24 {
            let _ = generate_surface_plot(&oc_csv, plot_field);
        }
        let chain = chain_from_csv(&oc_csv).expect("\nmain() :: Failed to load option chain from csv file");
        let contract_volume = chain.total_contract_volume();
        let open_interest = chain.total_open_interest();
        print!("\nmain() :: {} Total Option Contract Volume = {} and Total Open Interest = {}", uticker, contract_volume, open_interest);
        /*let _ = plot_volatility_smiles(&oc_csv);
        let (straddle_price, atm_call, atm_put) = get_atm_straddle(&oc_csv);
        println!("\nmain() :: {} ATM Straddle: ${} cost (implying underlying moves {}% by nearest expiry date)\nATM Call: {:?}\nATM Put: {:?}\n", uticker, straddle_price, straddle_price*0.85, atm_call, atm_put);
        let (ccredit, itm_call, otm_call) = get_atm_credit_spread(&oc_csv, true);
        println!("\nmain() :: Call Credit Spread:\nCredit: ${}\nITM Call: {:?}\nOTM Call: {:?}\n", ccredit, itm_call, otm_call);
        let (pcredit, itm_put, otm_put) = get_atm_credit_spread(&oc_csv, false);
        println!("\nmain() :: Put Credit Spread:\nCredit: ${}\nITM Put: {:?}\nOTM Put: {:?}\n", pcredit, itm_put, otm_put);
        let (cdebit, otm_call, itm_call) = get_atm_debit_spread(&oc_csv, true);
        println!("\nmain() :: Call Debit Spread:\nDebit: ${}\nOTM Call: {:?}\nITM Call: {:?}\n", cdebit, otm_call, itm_call);
        let (pdebit, otm_put, itm_put) = get_atm_debit_spread(&oc_csv, false);
        println!("\nmain() :: Put Debit Spread:\nDebit: ${}\nOTM Put: {:?}\nITM Put: {:?}\n", pdebit, otm_put, itm_put);*/
        let pdf_cmd = Command::new("cmd")
            .args(["/C", "python", "scripts/main.py", &uticker, &datetime_str])
            .output()
            .expect("\nmain() :: ERROR -> Failed to execute pdf_cmd");
        let stdout = str::from_utf8(&pdf_cmd.stdout).expect("\nmain() :: ERROR -> Invalif UTF-8 sequence in string");
        let stderr = str::from_utf8(&pdf_cmd.stderr).expect("\nmain() :: ERROR -> Invalif UTF-8 sequence in string");
        if pdf_cmd.status.success() {
            println!("\nmain() :: Successfully executed pdf_cmd / called scripts/main.py to generate PDF:\n\n{}\n", stdout);
            let _ = clear_directory_or_create("img_out");
            let _ = clear_directory_or_create("dat_out");
            let _ = clear_directory_or_create("html_out");
        } else {
            eprintln!("\nmain() :: ERROR -> PDF Generation failed with status: {:?}\n\n{}\n", pdf_cmd.status, stderr);
            exit(1);
        }
    } else {
        eprintln!("\nmain() :: ERROR -> Please enter a financial ticker/symbol that is at most 4 alphabetical characters; you entered '{}'", ticker);
        exit(1);
    }
}
