package main

import (
	"fmt"
	"math"
	"os"

	"github.com/coeasy/finkit/go/ta"
)

func main() {
	closes := []float64{
		44.34, 44.09, 43.61, 44.33, 44.83,
		45.10, 45.42, 45.84, 46.08, 45.89,
		46.03, 45.61, 46.28, 46.28, 46.00,
		46.03, 46.41, 46.22, 45.64, 46.21,
		46.25, 45.71, 46.45, 45.78, 45.35,
		44.03, 44.18, 44.22, 44.57, 43.42,
	}

	highs := make([]float64, len(closes))
	lows := make([]float64, len(closes))
	volume := make([]float64, len(closes))
	for i, c := range closes {
		highs[i] = c + 0.5
		lows[i] = c - 0.5
		volume[i] = 1000000 + float64(i)*10000
	}

	fmt.Println("=== Finkit Go Binding Example ===")
	fmt.Printf("Version: %s\n", ta.Version())
	fmt.Printf("Data points: %d\n\n", len(closes))

	sma, err := ta.Sma(closes, 14)
	if err != nil {
		fmt.Fprintf(os.Stderr, "SMA error: %v\n", err)
		os.Exit(1)
	}
	printResult("SMA(14)", sma)

	ema, err := ta.Ema(closes, 14)
	if err != nil {
		fmt.Fprintf(os.Stderr, "EMA error: %v\n", err)
		os.Exit(1)
	}
	printResult("EMA(14)", ema)

	rsi, err := ta.Rsi(closes, 14)
	if err != nil {
		fmt.Fprintf(os.Stderr, "RSI error: %v\n", err)
		os.Exit(1)
	}
	printResult("RSI(14)", rsi)

	macd, err := ta.Macd(closes, 12, 26, 9)
	if err != nil {
		fmt.Fprintf(os.Stderr, "MACD error: %v\n", err)
		os.Exit(1)
	}
	fmt.Println("MACD(12,26,9):")
	printResult("  MACD Line", macd.Macd)
	printResult("  Signal", macd.Signal)
	printResult("  Histogram", macd.Hist)

	bbands, err := ta.Bbands(closes, 20, 2.0, 2.0)
	if err != nil {
		fmt.Fprintf(os.Stderr, "BBANDS error: %v\n", err)
		os.Exit(1)
	}
	fmt.Println("Bollinger Bands(20,2,2):")
	printResult("  Upper", bbands.Upper)
	printResult("  Middle", bbands.Middle)
	printResult("  Lower", bbands.Lower)

	atr, err := ta.Atr(highs, lows, closes, 14)
	if err != nil {
		fmt.Fprintf(os.Stderr, "ATR error: %v\n", err)
		os.Exit(1)
	}
	printResult("ATR(14)", atr)

	valid := ta.FormulaValidate("SMA(CLOSE, 20)")
	fmt.Printf("\nFormula 'SMA(CLOSE, 20)' valid: %v\n", valid)

	fmt.Println("\n=== Example completed successfully ===")
}

func printResult(name string, data []float64) {
	validCount := 0
	for _, v := range data {
		if !math.IsNaN(v) {
			validCount++
		}
	}
	if len(data) > 0 {
		last := data[len(data)-1]
		fmt.Printf("%s: last=%.4f (valid=%d/%d)\n", name, last, validCount, len(data))
	} else {
		fmt.Printf("%s: no data\n", name)
	}
}
