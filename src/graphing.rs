use anyhow::{Context, Result};
//use super::options::chain_from_csv;
//use super::finmath::{d_one, FEDFUNDS};
use std::process::{Command, Stdio};
use std::io::{Write, BufWriter};
use std::fs::File;

const CDATNAME: &str = "dat_out/ctemp.dat";
const PDATNAME: &str = "dat_out/ptemp.dat";
pub const IMGDIR: &str = "img_out/";

pub fn generate_tseries_plot(ts_csv_name: &str, field: usize) -> Result<()> {
    let data_label = match field {
        0 => "Close",
        1 => "Open",
        2 => "Low",
        3 => "High",
        4 => "Volume",
        5 => "Change",
        6 => "%Change",
        7 => "Range",
        8 => "AvgTrueRange",
        9 => "RealizedVol",
        10 => "FiniteDiff",
        _ => "Close",
    };
    let name_parts: Vec<&str> = ts_csv_name.split('/').collect();
    let info_parts = name_parts[1].split('_').collect::<Vec<&str>>();
    let ticker = info_parts[0];
    let binding = data_label.to_lowercase();
    let png_name_label = match data_label {
        "%Change" => "perchange",
        "AvgTrueRange" => "atr",
        "RealizedVol" => "rvol",
        "FiniteDiff" => "bfd",
        _ => binding.as_str(),
    };
    let png_name = format!("{}{}_{}_{}_{}.png", IMGDIR, ticker, png_name_label, info_parts[2], info_parts[3].replace(".csv", ""));
    let mut gnuplot_script = format!(
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
        set logscale y
        set key autotitle columnheader
        plot '{}' using "Date":"{}" with lines title '{}'"#,
        png_name, data_label, ticker, data_label, ts_csv_name, data_label, data_label
    );
    if field == 0 || field == 1 || field == 2 || field == 3 {
        gnuplot_script.push_str(&format!(
            ", '{}' using \"Date\":\"LinearReg\" with lines title 'Linear Regression'",
            ts_csv_name
        ));
    }    
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

/*pub fn generate_surface_plot(chain_csv_name: &str, field: usize) -> Result<()> {
    let chain = chain_from_csv(chain_csv_name)
        .map_err(|e| anyhow::anyhow!("\ngenerate_surface_plot() :: ERROR -> Failed to load option chain with chain_from_csv: {}", e))?;
    let name_parts: Vec<&str> = chain_csv_name.split('/').collect();
    let info_parts: Vec<&str> = name_parts[1].split('_').collect();
    let cdatfile = File::create(CDATNAME).context("\ngenerate_surface_plot() :: ERROR -> Failed to create cdatfile")?;
    let pdatfile = File::create(PDATNAME).context("\ngenerate_surface_plot() :: ERROR -> Failed to create pdatfile")?;
    let mut cwriter = BufWriter::new(cdatfile);
    let mut pwriter = BufWriter::new(pdatfile);
    let data_label = match field {
        0 => "last",
        1 => "change",
        2 => "volume",
        3 => "bid",
        4 => "ask",
        5 => "oi",
        6 => "strike",
        7 => "yte",
        8 => "iv",
        9 => "delta",
        10 => "elasticity",
        11 => "vega",
        12 => "theta",
        13 => "rho",
        14 => "epsilon",
        15 => "gamma",
        16 => "vanna",
        17 => "charm",
        18 => "vomma",
        19 => "veta",
        20 => "speed",
        21 => "zomma",
        22 => "color",
        23 => "ultima",
        _ => "last",
    };
    let call_png_name = format!("{}{}_c{}_{}_{}.png", IMGDIR, &chain.ticker, data_label, info_parts[2], info_parts[3].replace(".csv", ""));
    let put_png_name = format!("{}{}_p{}_{}_{}.png", IMGDIR, &chain.ticker, data_label, info_parts[2], info_parts[3].replace(".csv", ""));
    for expiry in &chain.expiries {
        for call in expiry.calls.iter() {
            let civ = call.get_imp_vol(chain.current_price, chain.div_yield);
            let cd1 = d_one(civ, chain.current_price, call.strike, call.yte, chain.div_yield);
            let cd2 = cd1 - (civ * call.yte.sqrt());
            let cdata = match field {
                0 => call.last,
                1 => call.change,
                2 => call.vol,
                3 => call.bid,
                4 => call.ask,
                5 => call.open_int,
                6 => call.strike,
                7 => call.yte,
                8 => civ,
                9 => call.get_delta(chain.div_yield, cd1),
                10 => {
                    let cdelta = call.get_delta(chain.div_yield, cd1);
                    call.get_elasticity(chain.current_price, cdelta)
                },
                11 => call.get_vega(cd2),
                12 => call.get_theta(civ, chain.current_price, chain.div_yield, cd1, cd2, FEDFUNDS),
                13 => call.get_rho(cd2, FEDFUNDS),
                14 => call.get_epsilon(chain.current_price, chain.div_yield, cd1),
                15 => call.get_gamma(civ, chain.current_price, cd2, FEDFUNDS),
                16 => {
                    let cvega: f64 = call.get_vega(cd2);
                    call.get_vanna(civ, cvega, chain.current_price, cd1)
                },
                17 => call.get_charm(civ, chain.div_yield, cd1, cd2, FEDFUNDS),
                18 => {
                    let cvega: f64 = call.get_vega(cd2);
                    call.get_vomma(civ, cvega, cd1, cd2)
                },
                19 => call.get_veta(civ, chain.current_price, chain.div_yield, cd1, cd2, FEDFUNDS),
                20 => {
                    let cgamma: f64 = call.get_gamma(civ, chain.current_price, cd2, FEDFUNDS);
                    call.get_speed(civ, cgamma, cd1, cd2)
                },
                21 => {
                    let cgamma: f64 = call.get_gamma(civ, chain.current_price, cd2, FEDFUNDS); 
                    call.get_zomma(civ, cgamma, cd1, cd2)
                },
                22 => call.get_color(civ, chain.current_price, chain.div_yield, cd1, cd2, FEDFUNDS),
                23 => {
                    let cvega: f64 = call.get_vega(cd2);
                    call.get_ultima(civ, cvega, cd1, cd2)
                },
                _ => call.last,
            };
            writeln!(cwriter, "{} {} {}", call.strike, call.yte, cdata)?;
        }
        writeln!(cwriter, "")?;
        for put in expiry.puts.iter() {
            let piv = put.get_imp_vol(chain.current_price, chain.div_yield);
            let pd1 = d_one(piv, chain.current_price, put.strike, put.yte, chain.div_yield);
            let pd2 = pd1 - (piv * put.yte.sqrt());
            let pdata = match field {
                0 => put.last,
                1 => put.change,
                2 => put.vol,
                3 => put.bid,
                4 => put.ask,
                5 => put.open_int,
                6 => put.strike,
                7 => put.yte,
                8 => piv,
                9 => put.get_delta(chain.div_yield, pd1),
                10 => {
                    let pdelta = put.get_delta(chain.div_yield, pd1);
                    put.get_elasticity(chain.current_price, pdelta)
                },
                11 => put.get_vega(pd2),
                12 => put.get_theta(piv, chain.current_price, chain.div_yield, pd1, pd2, FEDFUNDS),
                13 => put.get_rho(pd2, FEDFUNDS),
                14 => put.get_epsilon(chain.current_price, chain.div_yield, pd1),
                15 => put.get_gamma(piv, chain.current_price, pd2, FEDFUNDS),
                16 => {
                    let pvega: f64 = put.get_vega(pd2);
                    put.get_vanna(piv, pvega, chain.current_price, pd1)
                },
                17 => put.get_charm(piv, chain.div_yield, pd1, pd2, FEDFUNDS),
                18 => {
                    let pvega: f64 = put.get_vega(pd2);
                    put.get_vomma(piv, pvega, pd1, pd2)
                },
                19 => put.get_veta(piv, chain.current_price, chain.div_yield, pd1, pd2, FEDFUNDS),
                20 => {
                    let pgamma: f64 = put.get_gamma(piv, chain.current_price, pd2, FEDFUNDS);
                    put.get_speed(piv, pgamma, pd1, pd2)
                },
                21 => {
                    let pgamma: f64 = put.get_gamma(piv, chain.current_price, pd2, FEDFUNDS); 
                    put.get_zomma(piv, pgamma, pd1, pd2)
                },
                22 => put.get_color(piv, chain.current_price, chain.div_yield, pd1, pd2, FEDFUNDS),
                23 => {
                    let pvega: f64 = put.get_vega(pd2);
                    put.get_ultima(piv, pvega, pd1, pd2)
                },
                _ => put.last,
            };
            writeln!(pwriter, "{} {} {}", put.strike, put.yte, pdata)?;
        }
        writeln!(pwriter, "")?;
    }
    cwriter.flush()?;
    pwriter.flush()?;
    let gnuplot_cscript = format!(
        r#"
        set terminal png
        set output '{}'
        set xlabel "Contract Strike Price ($)"
        set ylabel "Years To Expiration"
        set zlabel "{}"
        set title "{} Call Options {} Surface"
        set view 50.0,0.0,1.0
        set palette rgb 7,5,15
        splot '{}' using 1:2:3 with points palette title "Calls"
    "#, call_png_name, data_label, &chain.ticker, data_label, CDATNAME
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
        set xlabel "Contract Strike Price ($)"
        set ylabel "Years To Expiration"
        set zlabel "{}"
        set title "{} Put Options {} Surface"
        set view 50.0,0.0,1.0
        set palette rgb 7,5,15
        splot '{}' using 1:2:3 with points palette title "Puts"
    "#, put_png_name, data_label, &chain.ticker, data_label, PDATNAME
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

pub fn plot_volatility_smiles(chain_csv_name: &str) -> Result<()> {
    let chain = chain_from_csv(chain_csv_name)
        .map_err(|e| anyhow::anyhow!("\nplot_volatility_smiles() :: ERROR -> Failed to load option chain with chain_from_csv: {}", e))?;
    let name_parts: Vec<&str> = chain_csv_name.split('/').collect();
    let info_parts: Vec<&str> = name_parts[1].split('_').collect();
    let cdatfile = File::create(CDATNAME).context("\nplot_volatility_smiles() :: ERROR -> Failed to create cdatfile")?;
    let pdatfile = File::create(PDATNAME).context("\nplot_volatility_smiles() :: ERROR -> Failed to create pdatfile")?;
    let mut cwriter = BufWriter::new(cdatfile);
    let mut pwriter = BufWriter::new(pdatfile);
    let exp_date = &chain.expiries[0].date;
    for call in &chain.expiries[0].calls {
        let civ = call.get_imp_vol(chain.current_price, chain.div_yield);
        writeln!(cwriter, "{} {}", call.strike, civ)?;
    }
    cwriter.flush()?;
    for put in &chain.expiries[0].puts {
        let piv = put.get_imp_vol(chain.current_price, chain.div_yield);
        writeln!(pwriter, "{} {}", put.strike, piv)?;
    }
    pwriter.flush()?;
    let call_png_name = format!("{}{}_volsmile_{}_{}.png", IMGDIR, &chain.ticker, info_parts[2], info_parts[3].replace(".csv", ""));
    let put_png_name = format!("{}{}_volsmile_{}_{}.png", IMGDIR, &chain.ticker, info_parts[2], info_parts[3].replace(".csv", ""));
    let gnuplot_cscript = format!(
        r#"
        set terminal png
        set output '{}'
        set xlabel "Strike Price ($)"
        set ylabel "Implied Volatility"
        set title "{} Calls Volatility Smile (Expiring {})"
        set grid
        plot '{}' using 1:2 with lines title 'Implied Volatility'
        "#, call_png_name, &chain.ticker, exp_date, CDATNAME
    );
    let mut cmd_call = Command::new("gnuplot")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("\nplot_volatility_smiles() :: ERROR -> Failed to execute gnuplot call volatility smile command")?;
    let stdin = cmd_call.stdin.as_mut().context("\nplot_volatility_smiles() :: ERROR -> Failed to open stdin for gnuplot call volatility smile script")?;
    writeln!(stdin, "{}", gnuplot_cscript).context("\nplot_volatility_smiles() :: ERROR -> Failed to write gnuplot call volatility smile script to stdin")?;
    cmd_call.wait().context("\nplot_volatility_smiles() :: ERROR -> Failed to wait for gnuplot call volatility smile process")?;
    println!("\nplot_volatility_simles() :: Successfully generated {}", call_png_name);
    let gnuplot_pscript = format!(
        r#"
        set terminal png
        set output '{}'
        set xlabel "Strike Price ($)"
        set ylabel "Implied Volatility"
        set title "{} Puts Voliatility Smile (Expiring {})"
        set grid
        plot '{}' using 1:2 with lines title 'Implied Volatilty'
        "#, put_png_name, &chain.ticker, exp_date, PDATNAME
    );
    let mut cmd_put = Command::new("gnuplot")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("\nplot_volatility_smiles() :: ERROR -> Failed to execute gnuplot put volatility smile command")?;
    let stdin = cmd_put.stdin.as_mut().context("\nplot_volatility_smiles() :: ERROR -> Failed to open stdin for gnuplot put volatility smile script")?;
    writeln!(stdin, "{}", gnuplot_pscript).context("\nplot_volatility_smiles() :: ERROR -> Failed to write gnuplot put volatility smile script to stdin")?;
    cmd_call.wait().context("\nplot_volatility_smiles() :: ERROR -> Failed to wait for gnuplot put volatility smile process")?;
    println!("\nplot_volatility_simles() :: Successfully generated {}", put_png_name);
    Ok(())
}*/