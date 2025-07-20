WARNING: NOT A VALID FINANCIAL PREDICTION INSTRUMENT; USE AT YOUR OWN RISK.

Looking at many markets in many ways.

Requires gnuplot to be installed and available from command line.

TODO:
    - add linear regression calculations and charts of non OHLC data
    - find / calculate other financial metrics using finviz data or other info
    - try to implement simple AI neural net / model to predict next day close price using other time series data after implementing calculations into time series csv generation

src/avantage.rs:
    - daily_returns() :: Finds the daily returns and returns the returns as a vector in the order of the TimeSeries struct data
    - get_realized_vol() :: Calculates the realized volatility of the TimeSeries struct data for an input desired window (typically 30)
    - calculate_true_range() :: Helper function for get_avg_true_range(); finds the 'true range' of the TimeSeries struct data
    - get_avg_true_range() :: Calculates and returns the current average true range
    - ln_factorial() :: Approximation for natural log of n factorial
    - binomial_coefficient() :: Returns the binomial coefficient for parameters n, k using ln_factorial()
    - back_finite_diff() :: Approximates the current derivative of the TimeSeries struct data using bionmial_coefficient()
    - mean() :: Returns the mean of the list of floats
    - linear_regression() :: Returns the current linear regression approximation of the TimeSeries struct data
    - tseries_to_csv() :: Saves a TimeSeries struct as a csv file
    - get_underlying_av() :: Calls the Alpha Vantage API to fetch JSON time series OHLCV data and saves as a csv
    - tseries_from_csv() :: Instantiates a new TimeSeries struct from a csv file name

src/finmath.rs:
    - cnd() :: Cumulative Normal Distribution Function
    - npd() :: Normal Probability Density Function
    - brentq() :: Brent's Root Finding Method (inspired by python numpy/scipy implementation)
    - d_one() :: Helper for calculating d1 variable in Black-Scholes Options Pricing Model
    - black_scholes() :: Returns the price of a US Equity option according to the Black-Scholes option pricing model

src/finviz.src:
    - fetch_html() :: Fetches HTML content from the url parameter
    - parse_fv_html_table() :: Helper function for fetch_finviz_info()
    - parse_finval() :: Parses a string of a financial value into a float
    - compute_additional_financials() :: Returns a HashMap of additional financial data to append to the csv
    - fetch_finviz_info() :: Fetches, parses and saves financial information for an input ticker/symbol and saves it in csv format

src/graphing.rs:
    - generate_tseries_plot() :: Generates a specified time-series data chart using gnuplot
    - generate_surface_plot() :: Generates a specified option chain surface plot using gnuplot

src/options.rs:
    - str_to_float() :: Helper function to remove commas from numbers as strings and return as a float
    - fetch_option_chain() :: Fetches, parses, and saves option chain data from an underlying equity and saves the data in csv format
    - chain_from_csv() :: Instantiates an OptionChain struct from a csv file name