package ta

// MacdResult represents the result of MACD indicator calculation.
type MacdResult struct {
	Macd   []float64
	Signal []float64
	Hist   []float64
}

// BbandsResult represents the result of Bollinger Bands indicator calculation.
type BbandsResult struct {
	Upper  []float64
	Middle []float64
	Lower  []float64
}

// StochResult represents the result of Stochastic Oscillator calculation.
type StochResult struct {
	K []float64
	D []float64
}

// AroonResult represents the result of Aroon indicator calculation.
type AroonResult struct {
	AroonUp   []float64
	AroonDown []float64
}

// HtPhasorResult represents the result of Hilbert Transform Phasor calculation.
type HtPhasorResult struct {
	InPhase    []float64
	Quadrature []float64
}

// HtSineResult represents the result of Hilbert Transform Sine Wave calculation.
type HtSineResult struct {
	Sine     []float64
	LeadSine []float64
}
