package main

import (
    "os"
    "log"
    "strings"
)

func main() {
    argLen := len(os.Args)
    if argLen != 3 {
        log.Fatalf("\nmain(): Only two command line arguments allowed (ticker/symbol and csv file name); there were %d arguments passed into main.go executable\n\nProper usage = ./src/gomain TICKER CSVFILENAME.csv\n", argLen)
    }
    if !isAlphabetical(os.Args[1]) {
        log.Fatalf("\nmain(): Command line argument (os.Args[1]) is not purely alphabetical as a ticker/symbol should be; the value '%d' is invalid", os.Args[1])
    }
    if  !isValidCsv(os.Args[2]) {
        log.Fatalf("\nmain(): Command line argument (os.Args[2]) is not a valid csv file name; '%s' is invalid", os.Args[2])
    }
    scrapeChain(strings.ToUpper(os.Args[1]), os.Args[2])
}
