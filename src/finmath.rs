const A1: f64 = 0.254829592;
const A2: f64 = -0.284496736;
const A3: f64 = 1.421413741;
const A4: f64 = -1.453152027;
const A5: f64 = 1.061405429;
const P: f64 = 0.3275911;
const FEDFUNDS: f64 = 0.0533;
// Cumulative Normal Distribution
pub fn cnd(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + P * x);
    let y = (((((A5 * t + A4) * t) + A3) * t + A2) * t + A1) * t;
    0.5 * (1.0 + sign * y)
}
// Normal Probability Density Function
pub fn npd(x: f64) -> f64 {
    let exp_arg = -x * x / 2.0;
    let exponential_term = exp(exp_arg);
    let sqrt_two_pi = (2.0 * std::f64::consts::PI).sqrt();
    exponential_term / sqrt_two_pi * (A1 * exp_arg + A2 * exp_arg.powi(2) + A3 * exp_arg.powi(3) + A4 * exp_arg.powi(4) + A5 * exp_arg.powi(5)) / (1.0 + P * x * x)
}
// Brent's Root Finding Method
pub fn brentq<F>(f: F, mut a: f64, mut b: f64, tol: f64) -> Result<f64, String>
where
    F: Fn(f64) -> f64,
{
    if f(a) * f(b) > 0.0 {
        return Err(format!("\nbrentq(): root is not bracketed in the interval [{}, {}]", a, b));
    }
    while (b - a).abs() > tol {
        let c = (a + b) / 2.0;
        if f(c) == 0.0 {
            return Ok(c);
        } else if f(c) * f(a) < 0.0 {
            b = c;
        } else {
            a = c;
        }
    }
    Ok((a + b) / 2.0)
}
// Black-Scholes Helper for d1
pub fn d_one(iv: f64, s: f64, k: f64, t: f64, q: f64) -> f64 {
    (s.ln() / k + (FEDFUNDS - q + 0.5 * iv * iv) * t) / (iv * t.sqrt())
}
/* Black-Scholes Formula For US Equity Options
iv = the implied volatility of the underlying
s = the price of the underlying equity
k = the contract strike price
t = time to expiration (in years)
q = the underlying equity's dividend yield
r = the effective federal funds rate
*/
pub fn black_scholes(iv: f64, s: f64, k: f64, t: f64, q: f64, is_call: bool) -> f64 {
    let d1 = d_one(iv, s, k, t, q);
    let d2 = d1 - iv * t.sqrt();
    if is_call {
        (s * (-q * t).exp() * cnd(d1)) - (k * (-FEDFUNDS * t).exp() * cnd(d2))
    } else {
        (k * (-FEDFUNDS * t).exp() * cnd(-d2)) - (s * (-q * t).exp() * cnd(-d1))
    }
}