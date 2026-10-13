package main

import (
	"fmt"
	"math"

	"github.com/coeasy/finkit"
)

func main() {
	result, err := finkit.Sma([]float64{1, 2, 3, 4}, 3)
	if err != nil {
		panic(err)
	}
	if len(result) != 4 || !math.IsNaN(result[0]) || !math.IsNaN(result[1]) || result[2] != 2 || result[3] != 3 {
		panic(fmt.Sprintf("unexpected SMA result: %v", result))
	}
	fmt.Println("installed Go module SMA smoke passed")
}
