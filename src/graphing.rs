use anyhow::{Context, Result};
use super::avantage::tseries_from_csv;
use super::options::chain_from_csv;
use std::process::{Command, Stdio};
use std::io::{Write, BufWriter};
use std::fs::File;

const CDATNAME: &str = "dat_out/ctemp.dat";
const PDATNAME: &str = "dat_out/ptemp.dat";

pub fn generate_tseries_plot(ts_csv_name: &str, png_name: &str) -> Result<()> {
    let data_label: &str = "Close Price";
    let name_parts: Vec<&str> = ts_csv_name.split('/').collect();
    let ticker = name_parts[1].split('_').collect::<Vec<&str>>()[0];
    let gnuplot_script = format!(
        r#"
        set terminal png
        set output '{}'
        set datafile separator ','
        set xdata time
        set timefmt '%Y-%m-%d'
        set format x "%m/%d"
        set xlabel "Date"
        set ylabel "{}"
        set title "{} {}"
        set grid
        set key autotitle columnheader
        plot '{}' using "Date":"Close" with lines title '{}'
        "#, png_name, data_label, ticker, data_label, ts_csv_name, data_label);
    let mut cmd_gnuplot = Command::new("gnuplot")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("\ngenerate_tseries_plot() :: ERROR -> Failed to execute cmd_gnuplot chart generation command")?;
    let stdin = cmd_gnuplot.stdin.as_mut().context("\ngenerate_tseries_plot() :: ERROR -> Failed to open stdin for gnuplot_script")?;
    writeln!(stdin, "{}", gnuplot_script).context("\ngenerate_tseries_plot() :: ERROR -> Failed to write gnuplot_script to stdin for chart")?;
    cmd_gnuplot.wait().context("\ngenerate_tseries_plot() :: ERROR -> Failed to wait for gnuplot chart generation process")?;
    println!("\ngenerate_surface_plot() :: Successfully generated {}", png_name);
    Ok(())
}

pub fn generate_surface_plot(chain_csv_name: &str, call_png_name: &str, put_png_name: &str) -> Result<()> {
    let chain = chain_from_csv(chain_csv_name)
        .map_err(|e| anyhow::anyhow!("Failed to load option chain with chain_from_csv: {}", e))?;
    let tkr = &chain.ticker;
    let cdatfile = File::create(CDATNAME).context("\ngenerate_surface_plot() :: ERROR -> Failed to create cdatfile")?;
    let pdatfile = File::create(PDATNAME).context("\ngenerate_surface_plot() :: ERROR -> Failed to create pdatfile")?;
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
        r#"
        set terminal png
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
    let mut cmd_call = Command::new("gnuplot")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("\ngenerate_surface_plot() :: ERROR -> Failed to execute cmd_call gnuplot surface generation command")?;
    let stdin = cmd_call.stdin.as_mut().context("\ngenerate_surface_plot() :: ERROR -> Failed to open stdin for gnuplot_cscript")?;
    writeln!(stdin, "{}", gnuplot_cscript).context("\ngenerate_surface_plot() :: ERROR -> Failed to write gnuplot_cscript to stdin for call surface")?;
    cmd_call.wait().context("\ngenerate_surface_plot() :: ERROR -> Failed to wait for gnuplot call surface generation process")?;    
    println!("\ngenerate_surface_plot() :: Successfully generated {}", call_png_name);
    let gnuplot_pscript = format!(
        r#"
        set terminal png
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
    let mut cmd_put = Command::new("gnuplot")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("\ngenerate_surface_plot() :: ERROR -> Failed to execute cmd_put gnuplot surface generation command")?;
    let stdin = cmd_put.stdin.as_mut().context("\ngenerate_surface_plot() :: ERROR -> Failed to open stdin for gnuplot_pscript")?;
    writeln!(stdin, "{}", gnuplot_pscript).context("\ngenerate_surface_plot() :: ERROR -> Failed to write gnuplot_pscript to stdin for put surface")?;
    cmd_put.wait().context("\ngenerate_surface_plot() :: ERROR -> Failed to wait for gnuplot put surface generation process")?;    
    println!("\ngenerate_surface_plot() :: Successfully generated {}", put_png_name);
    Ok(())
}