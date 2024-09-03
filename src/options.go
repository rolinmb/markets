package main

import (
    "os"
    "fmt"
    "log"
    "math"
    "time"
    "regexp"
    "unicode"
    "os/exec"
    "strings"
    "strconv"
    "math/rand"
    "encoding/csv"
    "github.com/playwright-community/playwright-go"
)

const (
    TIMETEMPLATE = "Jan 2 2006"
    SLEEPSHORT = 1*time.Second
    SLEEPLONG = 100*time.Second
    FEDFUNDS = 0.0533
    A1 = 0.254829592
    A2 = -0.284496736
    A3 = 1.421413741
    A4 = -1.453152027
    A5 = 1.061405429
    P = 0.3275911
)

func randIntRange(min,max int) int {
    rand.Seed(time.Now().Unix())
    return rand.Intn(max-min+1) + min
}

func removeOrdinalSuffix(input string) string {
    rx := regexp.MustCompile(`(?i)(\d+)(st|nd|rd|th)`)
    return rx.ReplaceAllString(input, "$1")
} 

func isAlphabetical(s string) bool {
    for _, r := range s {
        if !unicode.IsLetter(r) {
            return false
        }
    }
    return true
}

func strToFloat(s string) float64 {
	num, err := strconv.ParseFloat(strings.Replace(s, ",", "", -1), 64)
	if err != nil {
		//fmt.Printf("\nstrToFloat(): could not parse '%s' to float64: %v\n > defaulting to (float64) 0.0\n", s, err)
		num = 0.0
	}
	return num
}

func cnd(x float64) float64 { // Cumulative Normal Distribution Function
    sign := 1.0
	if x < 0 {
		sign = -1.0
	}
	x = math.Abs(x)
	t := 1.0 / (1.0 + P*x)
	y := ((((A5*t+A4)*t)+A3)*t + A2) * t + A1
	return 0.5 * (1.0 + sign*y)
}

func npd(x float64) float64 { // Normal Probability Density Function
    expArg := -x * x / 2.0
	exponentialTerm := math.Exp(expArg)
	sqrtTwoPi := math.Sqrt(2 * math.Pi)
	return exponentialTerm / sqrtTwoPi * (A1*expArg + A2*math.Pow(expArg, 2) + A3*math.Pow(expArg, 3) + A4*math.Pow(expArg, 4) + A5*math.Pow(expArg, 5)) / (1.0 + P*x*x)
}

func brentq(f func(float64) float64, a,b,tol float64) (float64, error) {
    if f(a)*f(b) > 0 {
		return 0, fmt.Errorf("\nbrentq(): root is not bracketed in the interval [%f, %f]\n", a, b)
	}
	for math.Abs(b-a) > tol {
		c := (a + b) / 2
		if f(c) == 0 {
			return c, nil
		} else if f(c)*f(a) < 0 {
			b = c
		} else {
			a = c
		}
	}
	return (a + b) / 2, nil
}
/*
Solution to Black-Scholes Option Pricing Model (from Wikipedia)
V = Option Contract Price ($) (may need to set 0.0 value options to 0.01 for better .pngs)
S = Underlying Asset Price ($)
K = Contract Strike Price ($)
T = Time to Expiration (years)
Q = Underlying Dividend Yield (%)
R = Effective Federal Funds Rate (%) (use const named FEDFUNDS)
*/
func dOne(IV,S,K,T,Q float64) float64 {
    return (math.Log(S/K)+((FEDFUNDS-Q+(0.5*IV*IV))*T))/(IV*math.Sqrt(T))
}

func dTwo(d1,IV,T float64) float64 {
    return d1-(IV*math.Sqrt(T))
}

func blackScholes(IV,S,K,T,Q float64, ISCALL bool) float64 {
    d1 := dOne(IV, S, K, T, Q)
    d2 := dTwo(d1, IV, T)
    if ISCALL {
		return (S*math.Exp(-1.0*Q*T)*cnd(d1)) - (K*math.Exp(-1.0*FEDFUNDS*T)*cnd(d2))
	} else {
		return (K*math.Exp(-1.0*FEDFUNDS*T)*cnd(-d2)) - (S*math.Exp(-1.0*Q*T)*cnd(-d1))
	}
}

type Option struct {
    Last float64
    Change float64
    Vol float64
    Bid float64
    Ask float64
    OpenInt float64
    Strike float64
    Yte float64
    IsCall bool
}

func (o *Option) getImpVol(S,Q float64) float64 {
    f := func(x float64) float64 {
        return blackScholes(x, S, o.Strike, o.Yte, Q, o.IsCall) - o.Last
    }
    IV, err := brentq(f, 0.0, 15.0, 1e-6)
    if err != nil {
		//fmt.Printf("\ngetImpVol(): couldnlt find IV; defaulting to 0.0 %v\n", err)
		return 0.0
	}
	return IV
}

func (o *Option) getDelta(IV,S,Q,d1 float64) float64 {
    if o.IsCall {
        return math.Exp(-1.0*Q*o.Yte)*cnd(d1)
    }
    return -1.0*math.Exp(-1.0*Q*o.Yte)*cnd(-1.0*d1)
}

func (o *Option) getElasticity(DELTA,S float64) float64 {
    return DELTA * (S / o.Last)
}

func (o *Option) getVega(IV,S,Q,d1,d2 float64) float64 {
    VEGA := o.Strike * npd(d2) * math.Sqrt(o.Yte)
    if math.IsNaN(VEGA) {
        //fmt.Printf("\ngetVega(): couldn't calculate vega; defaulting to 0.0\n")
        return 0.0
    }
    return VEGA
}

func (o *Option) getTheta(IV,S,Q,d1,d2 float64) float64 {
    if o.IsCall {
        cTHETA := (((-1.0*math.Exp(-1.0*Q*o.Yte))*((S*npd(d1)*IV)/(2*math.Sqrt(o.Yte)))) - (FEDFUNDS*o.Strike*math.Exp(-1.0*FEDFUNDS*o.Yte)*cnd(d2))) + (Q*S*math.Exp(-1.0*Q*o.Yte)*cnd(d1))
        if math.IsNaN(cTHETA) {
            //fmt.Printf("\ngetTheta(): couldn't calculate theta; defaulting to 0.0\n")
            return 0.0
        }
        return cTHETA
    }
    pTHETA := (((-1.0*math.Exp(-1.0*Q*o.Yte))*((S*npd(d1)*IV)/(2*math.Sqrt(o.Yte)))) + (FEDFUNDS*o.Strike*math.Exp(-1.0*FEDFUNDS*o.Yte)*cnd(-1.0*d2))) - (Q*S*math.Exp(-1.0*Q*o.Yte)*cnd(-1.0*d1))
    if math.IsNaN(pTHETA) {
        //fmt.Printf("\ngetTheta(): couldn't calculate theta; defaulting to 0.0\n")
        return 0.0
    }
    return pTHETA
}

func (o *Option) getRho(d2 float64) float64 {
    if o.IsCall {
        cRHO := o.Strike*o.Yte*math.Exp(-1.0*FEDFUNDS*o.Yte)*cnd(d2)
        if math.IsNaN(cRHO) {
            //fmt.Printf("\ngetRho(): couldn't calculate rho; defaulting to 0.0\n")
            return 0.0
        }
        return cRHO
    }
    pRHO := -1.0*o.Strike*o.Yte*math.Exp(-1.0*FEDFUNDS*o.Yte)*cnd(-1.0*d2)
    if math.IsNaN(pRHO) {
        //fmt.Printf("\ngetRho(): couldn't calculate rho; defaulting to 0.0\n")
        return 0.0
    }
    return pRHO
}

func (o *Option) getEpsilon(IV,S,Q,d1 float64) float64 {
    if o.IsCall {
        cEPS := -1.0*S*o.Strike*o.Yte*math.Exp(-1.0*Q*o.Yte)*cnd(d1)
        if math.IsNaN(cEPS) {
            //fmt.Printf("\ngetEpsilon(): couldn't calculate epsilon; defaulting to 0.0\n")
            return 0.0
        }
        return cEPS
    }
    pEPS := S*o.Yte*math.Exp(-1.0*FEDFUNDS*o.Yte)*cnd(-1.0*d1)
    if math.IsNaN(pEPS) {
        //fmt.Printf("\ngetEpsilon(): couldn't calculate epsilon; defaulting to 0.0\n")
        return 0.0
    }
    return pEPS
}

func (o *Option) getGamma(IV,S,d2 float64) float64 {
    GAMMA := o.Strike*math.Exp(-1.0*FEDFUNDS*o.Yte)*(npd(d2)/(S*S*IV*math.Sqrt(o.Yte)))
    if math.IsNaN(GAMMA) {
        //fmt.Printf("\ngetGamma(): couldn't calculate gamma; defaulting to 0.0\n")
        return 0.0 
    }
    return GAMMA
}

func (o *Option) getVanna(IV,VEGA,S,d1 float64) float64 {
    VANNA := (VEGA/S)*(1.0 - (d1/(IV*math.Sqrt(o.Yte))))
    if math.IsNaN(VANNA) {
        //fmt.Printf("\ngetVanna(): couldn't calculate vanna; defaulting to 0.0\n")
        return 0.0
    }
    return VANNA
}

func (o *Option) getCharm(IV,Q,d1,d2 float64) float64 {
    if o.IsCall {
        cCHARM := (Q*math.Exp(-1.0*Q*o.Yte)*cnd(d1)) - ((math.Exp(-1.0*Q*o.Yte)*npd(d1))*(((2.0*(FEDFUNDS-Q)*o.Yte)-(d2*IV*math.Sqrt(o.Yte)))/(2.0*o.Yte*IV*math.Sqrt(o.Yte))))
        if math.IsNaN(cCHARM) {
        //fmt.Printf("\ngetCharm(): couldn't calculate charm; defaulting to 0.0\n")
        return 0.0
        }
        return cCHARM
    }
    pCHARM := (-1.0*Q*math.Exp(-1.0*Q*o.Yte)*cnd(-1.0*d1)) - ((math.Exp(-1.0*Q*o.Yte)*npd(d1))*(((2.0*(FEDFUNDS-Q)*o.Yte)-(d2*IV*math.Sqrt(o.Yte)))/(2.0*o.Yte*IV*math.Sqrt(o.Yte))))
    if math.IsNaN(pCHARM) {
        //fmt.Printf("\ngetCharm(): couldn't calculate charm; defaulting to 0.0\n")
        return 0.0
    }
    return pCHARM
}

func (o *Option) getVomma(IV,VEGA,d1,d2 float64) float64 {
    VOMMA := (VEGA*d1*d2)/IV
    if math.IsNaN(VOMMA) {
        //fmt.Printf("\ngetVomma(): couldn't calculate vomma; defaulting to 0.0\n")
        return 0.0
    }
    return VOMMA
}

func (o *Option) getVeta(IV,S,Q,d1,d2 float64) float64 {
    factor := -1.0*S*math.Exp(-1.0*Q*o.Yte)*npd(d1)*math.Sqrt(o.Yte)
    VETA := factor*(Q+(((FEDFUNDS-Q)*d1)/(IV*math.Sqrt(o.Yte)))-((1.0+(d1*d2))/(2.0*o.Yte)))
    if math.IsNaN(VETA) {
        //fmt.Printf("\ngetVeta(): couldn't calculate veta; defaulting to 0.0\n")
        return 0.0
    }
    return VETA
}

func (o *Option) getSpeed(IV,GAMMA,S,d1 float64) float64 {
    SPEED := (-1.0*GAMMA/S)*((d1/(IV*math.Sqrt(o.Yte)))+1.0)
    if math.IsNaN(SPEED) {
        //fmt.Printf("\ngetSpeed(): couldn't calculate speed; defaulting to 0.0\n")
        return 0.0
    }
    return SPEED
}

func (o *Option) getZomma(IV,GAMMA,d1,d2 float64) float64 {
    ZOMMA := GAMMA*(((d1*d2)-1.0)/IV)
    if math.IsNaN(ZOMMA) {
        //fmt.Printf("\ngetZomma(): couldn't calculate zomma; defaulting to 0.0\n")
        return 0.0
    }
    return ZOMMA
}

func (o *Option) getColor(IV,S,Q,d1,d2 float64) float64 {
    factor1 := -1.0*math.Exp(-1.0*Q*o.Yte)*npd(d1)/(2.0*S*o.Yte*IV*math.Sqrt(o.Yte))
    factor2 := (((2.0*(FEDFUNDS-Q)*o.Yte)-(d2*IV*math.Sqrt(o.Yte)))/(IV*math.Sqrt(o.Yte)))*d1
    COLOR := factor1*((2.0*Q*o.Yte)+1.0+factor2)
    if math.IsNaN(COLOR) {
        //fmt.Printf("\ngetColor(): couldn't calculate color; defaulting to 0.0\n")
        return 0.0
    }
    return COLOR  
}

func (o *Option) getUltima(IV,VEGA,d1,d2 float64) float64 {
    factor := (-1.0*VEGA)/(IV*IV)
    ULTIMA := factor*(((d1*d2)*(1.0-(d1*d2)))+(d1*d1)+(d2*d2))
    if math.IsNaN(ULTIMA) {
        //fmt.Printf("\ngetUltima(): couldn't calculate ultima; defaulting to 0.0\n")
        return 0.0
    }
    return ULTIMA
}

type OptionExpiry struct {
    Date string
    Yte float64
    Calls []Option
    Puts []Option
}

type OptionChain struct {
    Expiries []OptionExpiry
    Ticker string
    CurrentPrice float64
    DivYield float64
}

func (chain *OptionChain) chainToCSV(csvName string) {
    csvFile, err := os.Create(csvName)
    if err != nil {
        log.Fatalf("\nchainToCSV(): Error creating %s: %v", csvName, err)
    }
    defer csvFile.Close()
    csvWriter := csv.NewWriter(csvFile)
    csvWriter.UseCRLF = true
    defer csvWriter.Flush()
    headers := []string{
        "Ticker", "Expiration Date", "Yte",
        "Call Last", "Call Change", "Call Vol", "Call Bid", "Call Ask", "Call OpenInt",
        "Put Last", "Put Change", "Put Vol", "Put Bid", "Put Ask", "Put OpenInt",
        "Strike",
    }
    if err := csvWriter.Write(headers); err != nil {
        log.Fatalf("chainToCSV(): Error writing headers to %s: %v", csvName, err)
    }
    for _, expiry := range chain.Expiries {
        for i := 0; i < len(expiry.Calls); i++ {
            csvRow := []string{
                chain.Ticker,
                expiry.Date,
                fmt.Sprintf("%.6f", expiry.Yte),
                fmt.Sprintf("%.2f", expiry.Calls[i].Last),
                fmt.Sprintf("%.2f", expiry.Calls[i].Change),
                fmt.Sprintf("%.0f", expiry.Calls[i].Vol),
                fmt.Sprintf("%.2f", expiry.Calls[i].Bid),
                fmt.Sprintf("%.2f", expiry.Calls[i].Ask),
                fmt.Sprintf("%.0f", expiry.Calls[i].OpenInt),
                fmt.Sprintf("%.2f", expiry.Puts[i].Last),
                fmt.Sprintf("%.2f", expiry.Puts[i].Change),
                fmt.Sprintf("%.0f", expiry.Puts[i].Vol),
                fmt.Sprintf("%.2f", expiry.Puts[i].Bid),
                fmt.Sprintf("%.2f", expiry.Puts[i].Ask),
                fmt.Sprintf("%.0f", expiry.Puts[i].OpenInt),
                fmt.Sprintf("%.2f", expiry.Calls[i].Strike),
            }
            if err := csvWriter.Write(csvRow); err != nil {
                log.Fatalf("chainToCSV(): Error writing csv row %v to %s: %v", i, csvName, err)
            }
        }
    }
}

func getPriceAndYield(ticker string) (float64, float64) {
    pw, err := playwright.Run()
    if err != nil {
        log.Fatalf("\ngetPriceAndYield(): could not start playwright: %v\n", err)
    }
    browser, err := pw.Chromium.Launch(playwright.BrowserTypeLaunchOptions{
        Headless: playwright.Bool(true),
    })
    if err != nil {
        log.Fatalf("\ngetPriceAndYield(): could not launch chromium: %v\n", err)
    }
    page, err := browser.NewPage()
    if err != nil {
        log.Fatalf("\nscapeChain(): could not create page: %v\n", err)
    }
    url := fmt.Sprintf("https://bigcharts.marketwatch.com/quickchart/options.asp?symb=%s", ticker)
    if _, err = page.Goto(url); err != nil {
        log.Fatalf("\ngetPriceAndYield(): could not goto: %v\n", err)
    }
    time.Sleep(time.Duration(randIntRange(500, 1000))*time.Millisecond)
    fmt.Printf("\ngetPriceAndYield(): [playwright-go Chromium driver navigated to\n > %s]\n", url)
    priceStr, err := page.Locator(".fright .price").InnerText()
    if err != nil {
        log.Fatalf("\ngetPriceAndYield(): could not get current price: %v\n", err)
    }
    currentPrice := strToFloat(priceStr)
    time.Sleep(time.Duration(randIntRange(500, 1000))*time.Millisecond)
    yieldStr, err := page.Locator("td.label:has-text('Yield:') + td.aright").InnerText()
    if err != nil {
        log.Fatalf("\ngetPriceAndYield(): could not get dividend yield: %v\n", err)
    }
    yield := 0.0
    if yieldStr != "n/a" {
        yieldStr = strings.Replace(yieldStr, "%", "", -1)
        yield, err = strconv.ParseFloat(yieldStr, 64)
        if err != nil {
            log.Fatalf("\ngetPriceAndYield(): could not parse dividend yieldStr '%s' into float64: %v\n", err)
        }
        yield /= 100
    }
    if err = browser.Close(); err != nil {
        log.Fatalf("scapeChain(): could not close browser: %v", err)
    }
    if err = pw.Stop(); err != nil {
        log.Fatalf("scapeChain(): could not stop Playwright: %v", err)
    }
    return currentPrice, yield
}

/*func chainFromCSV(csvName string) OptionChain {
	csvFile, err := os.Open(csvName)
	if err != nil {
		log.Fatalf("\nchainFromCSV(): Error opening %s: %v", csvName, err)
	}
	defer csvFile.Close()
	reader := csv.NewReader(csvFile)
	lines, err := reader.ReadAll()
	if err != nil {
		log.Fatalf("chainFromCSV(): Error reading lines from %s: %v", csvName, err)
	}
	chain := OptionChain{ Ticker: "" }
    currentExpiry := OptionExpiry{ Date: "" }
	for i, line := range lines {
		if i == 0 {
			continue
		}
		if len(line) < 16 {
			log.Fatalf("chainFromCSV(): Unexpected number of columns in %s on line %v (found %v tokens, expected 16)", csvName, i, len(line))
		}
        if chain.Ticker == "" {
            chain.Ticker = line[0]
        }
        if currentExpiry.Date != line[1] {
            if len(currentExpiry.Calls) > 0 {
                chain.Expiries = append(chain.Expiries, currentExpiry)
                currentExpiry = OptionExpiry{}
            }
            currentExpiry.Date = line[1]
        }
        yte, _ := strconv.ParseFloat(line[2], 64)
        currentExpiry.Yte = yte
        call := Option{
            Last: strToFloat(line[3]),
            Change: strToFloat(line[4]),
            Vol: strToFloat(line[5]),
            Bid: strToFloat(line[6]), 
            Ask: strToFloat(line[7]),
            OpenInt: strToFloat(line[8]),
            Strike: strToFloat(line[15]),
            Yte: yte,
            IsCall: true,
        }
        put := Option{
            Last: strToFloat(line[9]),
            Change: strToFloat(line[10]),
            Vol: strToFloat(line[11]),
            Bid: strToFloat(line[12]),
            Ask: strToFloat(line[13]),
            OpenInt: strToFloat(line[14]),
            Strike: strToFloat(line[15]),
            Yte: yte,
            IsCall: false,
        }
        currentExpiry.Calls = append(currentExpiry.Calls, call)
        currentExpiry.Puts = append(currentExpiry.Puts, put)
    }
    if currentExpiry.Date != "" && len(currentExpiry.Calls) > 0 {
		chain.Expiries = append(chain.Expiries, currentExpiry)
    }
    currentPrice, yield := getPriceAndYield(chain.Ticker)
    chain.CurrentPrice = currentPrice
    chain.DivYield = yield
    return chain
}*/

func scrapeChain(ticker string) OptionChain {
    pw, err := playwright.Run()
    if err != nil {
        log.Fatalf("scapeChain(): could not start playwright: %v", err)
    }
    browser, err := pw.Chromium.Launch(playwright.BrowserTypeLaunchOptions{
        Headless: playwright.Bool(true),
    })
    if err != nil {
        log.Fatalf("scapeChain(): could not launch chromium: %v", err)
    }
    page, err := browser.NewPage()
    if err != nil {
        log.Fatalf("scapeChain(): could not create page: %v", err)
    }
    url := fmt.Sprintf("https://bigcharts.marketwatch.com/quickchart/options.asp?symb=%s", ticker)
    if _, err = page.Goto(url); err != nil {
        log.Fatalf("scapeChain(): could not goto: %v", err)
    }
    fmt.Printf("\nscapeChain(): [playwright-go Chromium driver navigated to\n > %s]\n", url)
    /*initHTML, err := page.Content()
    if err != nil {
        log.Fatalf("scapeChain(): could not get HTML content: %v", err)
    }
    fmt.Printf("scapeChain(): HTML content before toggling:\n%s\n", initHTML)*/
    priceStr, err := page.Locator(".fright .price").InnerText()
    if err != nil {
        log.Fatalf("\nscrapeChain(): could not get current price: %v\n", err)
    }
    currentPrice := strToFloat(priceStr)
    yieldStr, err := page.Locator("td.label:has-text('Yield:') + td.aright").InnerText()
    if err != nil {
        log.Fatalf("\nscrapeChain(): could not get dividend yield: %v\n", err)
    }
    yield := 0.0
    if yieldStr != "n/a" {
        yieldStr = strings.Replace(yieldStr, "%", "", -1)
        yield, err = strconv.ParseFloat(yieldStr, 64)
        if err != nil {
            log.Fatalf("\nscrapeChain(): could not parse dividend yieldStr '%s' into float64: %v\n", err)
        }
        yield /= 100
    } else {
        yieldStr = "0"
    }
    fmt.Printf("\nscrapeChain(): Current Price: $%.2f; Dividend Yield: %.4f (%s%%)\n", currentPrice, yield, yieldStr)
    time.Sleep(time.Duration(randIntRange(2000, 3000))*time.Millisecond)
    toggles, err := page.Locator("table.optionchain").Locator("tr.optiontoggle").Locator("td.caption").Locator("form.ajaxpartial").All()
    if err != nil {
        log.Fatalf("scapeChain(): could not get all toggled: %v", err)
    }
    fmt.Printf("\nscapeChain(): [len(toggles) = %d]\n", len(toggles))
    if len(toggles) > 1 {
        for i, tgl := range toggles[1:] {
        tgl.Click()
        sleepDur := time.Duration(randIntRange(1000, 2000))*time.Millisecond
        fmt.Printf("\nscapeChain(): [Toggle %d completed\n > sleeping %v]\n", i+1, sleepDur)
        time.Sleep(sleepDur)
        }
    }
    sleepDur := time.Duration(randIntRange(100000, 110000))*time.Millisecond
    fmt.Printf("\nscapeChain(): [All toggles completed (toggle 0 (1st) skipped because it's visible by default)\n > sleeping %v]\n", sleepDur)
    time.Sleep(sleepDur)
    /*fullHTML, err := page.Content()
    if err != nil {
        log.Fatalf("scapeChain(): could not get HTML content: %v", err)
    }
    fmt.Printf("scapeChain(): Full HTML content after toggling:\n%s\n", fullHTML)*/
    rows, err := page.Locator("table.optionchain tr.chainrow").All()
        if err != nil {
            log.Fatalf("\nscapeChain(): could not get opion chain HTML rows: %v\n", err)
        }
    chain := OptionChain{ Ticker: ticker, CurrentPrice: currentPrice, DivYield: yield }
    expiry := OptionExpiry{ Date: "", Yte: 0.0 }
    currentTime := time.Now()
    currentExpDate := ""
    currentYte := 0.0
	for _, tr := range rows {
        trText, _ := tr.TextContent()
        //fmt.Printf("\nscapeChain(): [Expiration: %s] trText (len(trText) = %d): %v\n", currentExpDate, len(trText), trText)
        if strings.TrimSpace(trText) == "" || strings.Contains(trText, "Stock Price »") ||
            strings.Contains(trText, "CALLS") || strings.Contains(trText, "Last") ||
            strings.Contains(trText, "Show") {
            continue
        }
        if strings.Contains(trText, "Expires") {
            dateFields := strings.Fields(trText)
            if len(dateFields[1]) >= 3 {
                dateFields[1] = removeOrdinalSuffix(dateFields[1])
            }
            dateFields[2] = strings.Replace(dateFields[2], ",", "", -1)
            newExpDate := fmt.Sprintf("%s %s %s", dateFields[1][:3], dateFields[2], dateFields[3])
            parsedTime, err := time.Parse(TIMETEMPLATE, newExpDate)
            if err != nil {
                log.Fatalf("\nscapeChain(): Error parsing newExpDate '%s' into time.Time(): %v\n", newExpDate, err)
            }
            newYte := math.Abs(currentTime.Sub(parsedTime).Hours()) / 24 / 252  
            if currentExpDate != newExpDate && currentExpDate != "" {
                expiry.Date = currentExpDate
                expiry.Yte = currentYte 
                chain.Expiries = append(chain.Expiries, expiry)
                expiry = OptionExpiry{}
                fmt.Printf("\nscapeChain(): [FINISHED PARSING EXPIRATION DATE: %s (%.3f years to expiration)]\n", currentExpDate, currentYte)
            }
            currentExpDate = newExpDate
            currentYte = newYte
            continue
        }
        tdCells, err := tr.Locator("td").All()
        if err != nil {
            log.Fatalf("scapeChain(): could not get tdCells: %v", err)
        }
        var trData []float64
        for _, td := range tdCells {
            tdText, err := td.TextContent()
            if err != nil {
                log.Fatalf("scapeChain(): could not get tdText: %v", err)
            }
            if strings.TrimSpace(tdText) == "" {
                trData = append(trData, 0.0)
                continue
            }
            //fmt.Printf("\nscapeChain(): [Expiration: %s] tdText (len(tdTtext) = %d): '%v'\n", currentExpDate, len(tdText), tdText)
            tdFields := strings.Fields(tdText)
            //fmt.Printf("\nscapeChain(): [Expiration: %s] tdFields (len(tdFields) = %d): %v\n", currentExpDate, len(tdFields), tdFields)
            num, err := strconv.ParseFloat(strings.Replace(tdFields[0], ",", "", -1), 64)
            if err != nil {
                //fmt.Printf("scapeChain(): could not parse '%s' to float64: %v\n > defaulting to (float64) 0.0\n", tdFields[0], err)
                num = 0.0
            }
            trData = append(trData, num)
        }
        //fmt.Printf("\nscapeChain(): [Expiration: %s] trData (len(trData) = %d): %v\n", currentExpDate, len(trData), trData)
        if len(trData) < 13 {
            continue
        }
        call := Option{
            Last: trData[0],
            Change: trData[1],
            Vol: trData[2],
            Bid: trData[3],
            Ask: trData[4],
            OpenInt: trData[5],
            Strike: trData[6],
            Yte: currentYte,
            IsCall: true,
        }
        put := Option{
            Last: trData[7],
            Change: trData[8],
            Vol: trData[9],
            Bid: trData[10],
            Ask: trData[11],
            OpenInt: trData[12],
            Strike: trData[6],
            Yte: currentYte,
            IsCall: false,
        }
        expiry.Calls = append(expiry.Calls, call)
        expiry.Puts = append(expiry.Puts, put) 
        //fmt.Printf("\nscapeChain(): [Expiration: %s] $%f CALL > %v\n", currentExpDate, call.Strike, call)
        //fmt.Printf("\nscapeChain(): [Expiration: %s] $%f PUT > %v\n", currentExpDate, put.Strike, put)
    }
    if currentExpDate != "" && len(expiry.Calls) > 0 {
        expiry.Date = currentExpDate
        expiry.Yte = currentYte
        chain.Expiries = append(chain.Expiries, expiry)
    }
    if err = browser.Close(); err != nil {
        log.Fatalf("scapeChain(): could not close browser: %v", err)
    }
    if err = pw.Stop(); err != nil {
        log.Fatalf("scapeChain(): could not stop Playwright: %v", err)
    }
    return chain
}

/*func (chain *OptionChain) plotSurfaces(surfType int, tnowStr string) {
    if surfType > 19 || surfType < 0 {
        log.Fatalf("\nplotSurfaces(): surfType > 19 or < 0 (surfType = %d)\n", surfType)
    }
	fileCall, err := os.Create("src/dat_out/tempcall.dat")
	if err != nil {
		log.Fatalf("\nplotSurfaces(): Couldn't create src/dat_out/tempcall.dat: %v", err)
	}
	defer fileCall.Close()
    filePut, err := os.Create("src/dat_out/tempput.dat")
    if err != nil {
        log.Fatalf("\nplotSurfaces(): Couldn't create src/dat_out/tempput.dat: %v", err)
    }
    defer filePut.Close()
    var chartTitle string
    switch surfType {
    case 0:
        chartTitle = "Price"
    case 1:
        chartTitle = "IV"
    case 2:
        chartTitle = "Delta"
    case 3:
        chartTitle = "Elasticity"
    case 4:
        chartTitle = "Vega"
    case 5:
        chartTitle = "Theta"
    case 6:
        chartTitle = "Rho"
    case 7:
        chartTitle = "Epsilon"
    case 8:
        chartTitle = "Gamma"
    case 9:
        chartTitle = "Vanna"
    case 10:
        chartTitle = "Charm"
    case 11:
        chartTitle = "Vomma"
    case 12:
        chartTitle = "Veta"
    case 13:
        chartTitle = "Speed"
    case 14:
        chartTitle = "Zomma"
    case 15:
        chartTitle = "Color"
    case 16:
        chartTitle = "Ultima"
    case 17:
        chartTitle = "Change"
    case 18:
        chartTitle = "Volume"
    case 19:
        chartTitle = "OpenInt"
    default:
        chartTitle = "Price"
    }
    var civ,piv,cd1,cd2,pd1,pd2,cVega,pVega,cDelta,pDelta,cGamma,pGamma,callVal,putVal float64
	for i, expiry := range chain.Expiries {
        for _, call := range expiry.Calls {
            if surfType > 0 {
                civ = call.getImpVol(chain.CurrentPrice, chain.DivYield)
                if surfType >= 2 {
                    cd1 = dOne(civ, chain.CurrentPrice, call.Strike, call.Yte, chain.DivYield)
                    if surfType == 2 || surfType == 3 {
                        cDelta = call.getDelta(civ, chain.CurrentPrice, chain.DivYield, cd1)
                    }
                    if (surfType >= 4 && surfType <= 6) || (surfType >= 8 && surfType <= 16){
                        cd2 = dTwo(cd1, civ, call.Yte)
                        if surfType == 4 || surfType == 9 || surfType == 11 || surfType == 16 {
                            cVega = call.getVega(civ, chain.CurrentPrice, chain.DivYield, cd1, cd2)
                        }
                        if surfType == 13 || surfType == 14 {
                            cGamma = call.getGamma(civ, chain.CurrentPrice, cd2)
                        }
                    }
                }
            }
            switch surfType {
            case 0:
                callVal = call.Last
            case 1:
                callVal = civ
            case 2:
                callVal = cDelta
            case 3:
                callVal = call.getElasticity(cDelta, chain.CurrentPrice)
            case 4:
                callVal = cVega
            case 5:
                callVal = call.getTheta(civ, chain.CurrentPrice, chain.DivYield, cd1, cd2)
            case 6:
                callVal = call.getRho(cd2)
            case 7:
                callVal = call.getEpsilon(civ, chain.CurrentPrice, chain.DivYield, cd1)
            case 8:
                callVal = call.getGamma(civ, chain.CurrentPrice, cd2)
            case 9:
                callVal = call.getVanna(civ, cVega, chain.CurrentPrice, cd1)
            case 10:
                callVal = call.getCharm(civ, chain.DivYield, cd1, cd2)
            case 11:
                callVal = call.getVomma(civ, cVega, cd1, cd2)
            case 12:
                callVal = call.getVeta(civ, chain.CurrentPrice, chain.DivYield, cd1, cd2)
            case 13:
                callVal = call.getSpeed(civ, cGamma, chain.CurrentPrice, cd1)
            case 14:
                callVal = call.getZomma(civ, cGamma, cd1, cd2)
            case 15:
                callVal = call.getColor(civ, chain.CurrentPrice, chain.DivYield, cd1, cd2)
            case 16:
                callVal = call.getUltima(civ, cVega, cd1, cd2)
            case 17:
                callVal = call.Change
            case 18:
                callVal = call.Vol
            case 19:
                callVal = call.OpenInt
            default:
                callVal = call.Last
            }
            _, err := fileCall.WriteString(fmt.Sprintf("%d %f %f\n", i, call.Strike, callVal))
            if err != nil {
                log.Fatalf("\nplotSurfaces(): Couldn't write line %d to src/dat_out/tempcall.dat: %v", i, err)
            }
        }
        for _, put := range expiry.Puts {
            if surfType > 0 {
                piv = put.getImpVol(chain.CurrentPrice, chain.DivYield)
                if surfType >= 2 {
                pd1 = dOne(piv, chain.CurrentPrice, put.Strike, put.Yte, chain.DivYield)
                if surfType == 2 || surfType == 3 {
                    pDelta = put.getDelta(piv, chain.CurrentPrice, chain.DivYield, pd1)
                }
                if (surfType >= 4 && surfType <= 6) || (surfType >= 8 && surfType <= 16){
                    pd2 = dTwo(pd1, piv, put.Yte)
                    if surfType == 4 || surfType == 9 || surfType == 11 || surfType == 16 {
                    pVega = put.getVega(piv, chain.CurrentPrice, chain.DivYield, pd1, pd2)
                    }
                    if surfType == 13 || surfType == 14 {
                    pGamma = put.getGamma(piv, chain.CurrentPrice, pd2)
                    }
                }
                }
            }
            switch surfType {
            case 0:
                putVal = put.Last
            case 1:
                putVal = piv
            case 2:
                putVal = pDelta
            case 3:
                putVal = put.getElasticity(pDelta, chain.CurrentPrice)
            case 4:
                putVal = pVega
            case 5:
                putVal = put.getTheta(piv, chain.CurrentPrice, chain.DivYield, pd1, pd2)
            case 6:
                putVal = put.getRho(pd2)
            case 7:
                putVal = put.getEpsilon(piv, chain.CurrentPrice, chain.DivYield, pd1)
            case 8:
                putVal = put.getGamma(piv, chain.CurrentPrice, pd2)
            case 9:
                putVal = put.getVanna(piv, pVega, chain.CurrentPrice, pd1)
            case 10:
                putVal = put.getCharm(piv, chain.DivYield, pd1, pd2)
            case 11:
                putVal = put.getVomma(piv, pVega, pd1, pd2)
            case 12:
                putVal = put.getVeta(piv, chain.CurrentPrice, chain.DivYield, pd1, pd2)
            case 13:
                putVal = put.getSpeed(piv, pGamma, chain.CurrentPrice, pd1)
            case 14:
                putVal = put.getZomma(piv, pGamma, pd1, pd2)
            case 15:
                putVal = put.getColor(piv, chain.CurrentPrice, chain.DivYield, pd1, pd2)
            case 16:
                putVal = put.getUltima(piv, pVega, pd1, pd2)
            case 17:
                putVal = put.Change
            case 18:
                putVal = put.Vol
            case 19:
                putVal = put.OpenInt
            default:
                putVal = put.Last
            }
            _, err = filePut.WriteString(fmt.Sprintf("%d %f %f\n", i, put.Strike, putVal))
            if err != nil {
                log.Fatalf("\nplotSurfaces(): Couldn't write line %d to src/dat_out/tempput.dat: %v", i, err)
            }
        }
    }
    timestamp := strings.Replace(strings.Replace(strings.Replace(tnowStr, "-", " ", -1), "_", "-", 2), "_", ":", 2)
    scriptCall := fmt.Sprintf(
        `set terminal png
        set output "src/png_out/%s_c%s_%s.png"
        set xlabel "Expiry"
        set ylabel "Strike ($)"
        set zlabel "%s"
        set title "%s Call %s Surface (%s)"
        set view 25.0,275.0,1.0
        set palette rgb 7,5,15
        splot 'src/dat_out/tempcall.dat' using 1:2:3 with points palette title '%s'`,
        chain.Ticker, strings.ToLower(chartTitle), tnowStr, chartTitle, chain.Ticker, chartTitle, timestamp, chartTitle)
    scriptPut := fmt.Sprintf(
        `set terminal png
        set output "src/png_out/%s_p%s_%s.png"
        set xlabel "Expiry"
        set ylabel "Strike ($)"
        set zlabel "%s"
        set title "%s Put %s Surface (%s)"
        set view 25.0,275.0,1.0
        set palette rgb 7,5,15
        splot 'src/dat_out/tempput.dat' using 1:2:3 with points palette title '%s'`,
        chain.Ticker, strings.ToLower(chartTitle), tnowStr, chartTitle, chain.Ticker, chartTitle, timestamp, chartTitle)
    cmdCall := exec.Command("gnuplot")
    cmdCall.Stdin = strings.NewReader(scriptCall)
    cmdCall.Stdout = os.Stdout
    cmdCall.Stderr = os.Stderr
    err = cmdCall.Run()
    if err != nil {
        log.Fatalf("\nplotSurfaces(): Error running gnuplot generation script for %s call option %s surface: %v", chain.Ticker, chartTitle, err)
    }
    fmt.Printf("\nplotSurfaces(): Successfully saved %s Call %s Surface to src/png_out/%s_c%s.png\n", chain.Ticker, chartTitle, chain.Ticker, strings.ToLower(chartTitle))
    cmdPut := exec.Command("gnuplot")
    cmdPut.Stdin = strings.NewReader(scriptPut)
    cmdPut.Stdout = os.Stdout
    cmdPut.Stderr = os.Stderr
    err = cmdPut.Run()
    if err != nil {
        log.Fatalf("\nplotSurfaces() Error running gnuplot generation script for %s put option %s surface: %v", chain.Ticker, chartTitle, err)
    }
    fmt.Printf("\nplotSurfaces(): Successfully saved %s Put %s Surface to src/png_out/%s_p%s.png\n", chain.Ticker, chartTitle, chain.Ticker, strings.ToLower(chartTitle))
}*/