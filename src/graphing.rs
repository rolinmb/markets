use anyhow::{Context, Result};
//use super::avantage::tseries_from_csv;
use super::options::chain_from_csv;
use std::process::Command;
use std::fs::File;
use std::io::{Write, BufWriter};

const CDATNAME: &str = "dat_out/ctemp.dat";
const PDATNAME: &str = "dat_out/ptemp.dat";

pub fn generate_surface_plot(chain_csv_name: &str, call_png_name: &str, put_png_name: &str) -> Result<()> {
    let chain = chain_from_csv(chain_csv_name).unwrap();
    let tkr = &chain.ticker;
    let cdatfile = File::create(CDATNAME)?;
    let pdatfile = File::create(PDATNAME)?;
    let mut cwriter = BufWriter::new(cdatfile);
    let mut pwriter = BufWriter::new(pdatfile);
    let data_label: &str = "Last Trade Price";
    for expiry in &chain.expiries {
        for call in expiry.calls.iter() {
            writeln!(cwriter, "{} {} {}", call.strike, call.yte, call.last)?;
        }
        writeln!(cwriter, "")?;
        for put in expiry.puts.iter() {
            writeln!(pwriter, "{} {} {}", put.strike, put.yte, put.last)?;
        }
        writeln!(pwriter, "")?;
    }
    cwriter.flush()?;
    pwriter.flush()?;
    let gnuplot_cscript = format!(
        r#"set terminal png
        set output '{}'
        set xlabel "Strike Price ($)"
        set ylabel "YTE"
        set zlabel "{}"
        set title "{} Call Options {} Surface"
        set view 25.0,275.0,1.0
        set palette rgb 7,5,15
        splot '{}' using 1:2:3 with points palette title "Calls"
    "#, call_png_name, data_label, tkr, data_label, CDATNAME
    );
    Command::new("cmd")
        .args(["/C", "gnuplot", &gnuplot_cscript])
        .output()
        .expect("\ngenerate_surface_plot() :: ERROR -> Failed to execute gnuplot call surface generation command");
    println!("\ngenerate_surface_plot() :: Successfully generated {}",put_png_name);
    let gnuplot_pscript = format!(
        r#"set terminal png
        set output '{}'
        set xlabel "Strike Price"
        set ylabel "YTE"
        set zlabel "{}"
        set title "{} Put Options {} Surface"
        set view 25.0,275.0,1.0
        set palette rgb 7,5,15
        splot '{}' using 1:2:3 with points palette title "Puts"
    "#, put_png_name, data_label, tkr, data_label, PDATNAME
    );
    Command::new("cmd")
        .args(["/C", "gnuplot", &gnuplot_pscript])
        .output()
        .expect("\ngenerate_surface_plot() :: ERROR -> Failed to execute gnuplot put surface generation command");
    println!("\ngenerate_surface_plot() :: Successfully generated {}",put_png_name);
    Ok(())
}