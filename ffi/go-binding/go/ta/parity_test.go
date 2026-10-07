package ta

import (
	"math"
	"testing"
)

// Coverage for the registry-parity wrappers added to close the Go binding's gap
// against the 78-indicator FFI registry.
//
// These run the real cgo path, so they catch ABI drift that the Rust-side tests
// (which call the exported symbols directly) cannot see: a mismatched struct
// layout in the cgo preamble, or a wrong element type in the result copy.

func paritySeries(n int) []float64 {
	out := make([]float64, n)
	for i := range out {
		out[i] = 100.0 + math.Sin(float64(i)*0.11)*5.0 + float64(i)*0.05
	}
	return out
}

func parityHighLow(close []float64) ([]float64, []float64, []float64) {
	high := make([]float64, len(close))
	low := make([]float64, len(close))
	open := make([]float64, len(close))
	for i, c := range close {
		high[i] = c + 1.5
		low[i] = c - 1.5
		open[i] = c - 0.4
	}
	return high, low, open
}

func TestParitySingleOutputLengths(t *testing.T) {
	close := paritySeries(128)
	high, low, open := parityHighLow(close)
	volume := make([]float64, len(close))
	for i := range volume {
		volume[i] = 1000.0 + float64(i)
	}

	cases := []struct {
		name string
		call func() ([]float64, error)
	}{
		{"Apo", func() ([]float64, error) { return Apo(close, 12, 26) }},
		{"Cmo", func() ([]float64, error) { return Cmo(close, 14) }},
		{"Trix", func() ([]float64, error) { return Trix(close, 15) }},
		{"PercentRank", func() ([]float64, error) { return PercentRank(close, 14) }},
		{"ChandeForecast", func() ([]float64, error) { return ChandeForecast(close, 14) }},
		{"MidPoint", func() ([]float64, error) { return MidPoint(close, 14) }},
		{"MidPrice", func() ([]float64, error) { return MidPrice(high, low, 14) }},
		{"Sar", func() ([]float64, error) { return Sar(high, low, 0.02, 0.2) }},
		{"AvgPrice", func() ([]float64, error) { return AvgPrice(open, high, low, close) }},
		{"MedPrice", func() ([]float64, error) { return MedPrice(high, low) }},
		{"TypPrice", func() ([]float64, error) { return TypPrice(high, low, close) }},
		{"WclPrice", func() ([]float64, error) { return WclPrice(high, low, close) }},
		{"Bop", func() ([]float64, error) { return Bop(open, high, low, close) }},
		{"Mfi", func() ([]float64, error) { return Mfi(high, low, close, volume, 14) }},
		{"Vzo", func() ([]float64, error) { return Vzo(close, volume, 14) }},
		{"VolumeMomentum", func() ([]float64, error) { return VolumeMomentum(volume, 14) }},
		{"VolumeRoc", func() ([]float64, error) { return VolumeRoc(volume, 14) }},
		{"TwiggsMoneyFlow", func() ([]float64, error) { return TwiggsMoneyFlow(high, low, close, volume, 21) }},
		{"Inertia", func() ([]float64, error) { return Inertia(open, high, low, close, 14, 20) }},
	}

	for _, tc := range cases {
		got, err := tc.call()
		if err != nil {
			t.Fatalf("%s: unexpected error: %v", tc.name, err)
		}
		if len(got) != len(close) {
			t.Fatalf("%s: got %d values, want %d", tc.name, len(got), len(close))
		}
		finite := false
		for _, v := range got {
			if !math.IsNaN(v) {
				finite = true
				break
			}
		}
		if !finite {
			t.Fatalf("%s: every value is NaN", tc.name)
		}
	}
}

func TestParityTwoOutputLengths(t *testing.T) {
	close := paritySeries(128)
	high, low, _ := parityHighLow(close)

	vortex, err := Vortex(high, low, close, 14)
	if err != nil {
		t.Fatalf("Vortex: %v", err)
	}
	if len(vortex.ViPlus) != len(close) || len(vortex.ViMinus) != len(close) {
		t.Fatalf("Vortex: got VI+=%d VI-=%d, want %d each", len(vortex.ViPlus), len(vortex.ViMinus), len(close))
	}

	mama, err := Mama(close, 0.5, 0.05)
	if err != nil {
		t.Fatalf("Mama: %v", err)
	}
	if len(mama.Mama) != len(close) || len(mama.Fama) != len(close) {
		t.Fatalf("Mama: got Mama=%d Fama=%d, want %d each", len(mama.Mama), len(mama.Fama), len(close))
	}
}

func TestParityCandlestickValues(t *testing.T) {
	close := paritySeries(128)
	high, low, open := parityHighLow(close)

	cases := []struct {
		name string
		call func() ([]int32, error)
	}{
		{"CdlDoji", func() ([]int32, error) { return CdlDoji(open, high, low, close, 0.1) }},
		{"CdlHammer", func() ([]int32, error) { return CdlHammer(open, high, low, close) }},
		{"CdlEngulfing", func() ([]int32, error) { return CdlEngulfing(open, high, low, close) }},
		{"CdlMarubozu", func() ([]int32, error) { return CdlMarubozu(open, high, low, close, 0.1) }},
		{"CdlThreeBlackCrows", func() ([]int32, error) { return CdlThreeBlackCrows(open, high, low, close) }},
	}

	for _, tc := range cases {
		got, err := tc.call()
		if err != nil {
			t.Fatalf("%s: unexpected error: %v", tc.name, err)
		}
		if len(got) != len(close) {
			t.Fatalf("%s: got %d values, want %d", tc.name, len(got), len(close))
		}
		for _, v := range got {
			if v != -100 && v != 0 && v != 100 {
				t.Fatalf("%s: value %d outside TA-Lib's {-100, 0, 100}", tc.name, v)
			}
		}
	}
}

func TestParityInvalidPeriodReturnsError(t *testing.T) {
	close := paritySeries(8)
	if _, err := Cmo(close, 64); err == nil {
		t.Fatal("Cmo with period > length must return an error")
	}
	if _, err := Apo(close, 5, 64); err == nil {
		t.Fatal("Apo with slow period > length must return an error")
	}
}
