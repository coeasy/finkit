type StreamingMacd struct {
	handle unsafe.Pointer
}

// MaType identifies one of the scalar moving-average variants supported by
// the streaming MACDEXT implementation. The numeric values are part of the
// C ABI and intentionally mirror the Rust selector order.
type MaType int

const (
	MaSMA MaType = iota
	MaEMA
	MaWMA
	MaDEMA
	MaTEMA
	MaKAMA
	MaT3
	MaTRIMA
	MaHMA
	MaALMA
	MaVIDYA
)

type StreamingMacdExt struct {
	handle unsafe.Pointer
}

// NewStreamingMacdExt creates a streaming MACDEXT with independent MA types
// for the fast, slow, and signal lines. It returns nil for invalid periods or
// unsupported selector values. MAMA and FRAMA are intentionally batch-only.
func NewStreamingMacdExt(fastPeriod int, fastMa MaType, slowPeriod int, slowMa MaType, signalPeriod int, signalMa MaType) *StreamingMacdExt {
	h := C.ta_streaming_macd_ext_new(
		cInt(fastPeriod), cInt(int(fastMa)), cInt(slowPeriod), cInt(int(slowMa)), cInt(signalPeriod), cInt(int(signalMa)),
	)
	if h == nil {
		return nil
	}
	return &StreamingMacdExt{handle: unsafe.Pointer(h)}
}

func (s *StreamingMacdExt) Update(value float64) (MacdOutput, bool) {
	if s == nil || s.handle == nil {
		return MacdOutput{}, false
	}
	var macd, sig, hist C.double
	ready := C.ta_streaming_macd_ext_update(s.handle, cDouble(value), &macd, &sig, &hist)
	if ready == 0 {
		return MacdOutput{}, false
	}
	return MacdOutput{Macd: float64(macd), Signal: float64(sig), Hist: float64(hist)}, true
}