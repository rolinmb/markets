mod avantage;
use avantage::{tseries_from_csv};
mod options;
use options::{chain_from_csv};
// TODO: create function for generating 2d plots via gnuplot using time series data then save as png images (not just close price and volume, indicators too)

// TODO: create function for generating 3d plots via gnuplot using option chain derivative surfaces like in go-vol