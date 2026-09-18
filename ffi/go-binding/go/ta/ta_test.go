package ta

import (
	"encoding/json"
	"math"
	"os"
	"path/filepath"
	"testing"
)

func loadEngineContract(t *testing.T) map[string]interface{} {
	t.Helper()
	paths := []string{
		filepath.Join("..", "..", "..", "..", "tests", "contracts", "engine_contract_v1.json"),
		filepath.Join("..", "..", "..", "tests", "contracts", "engine_contract_v1.json"),
		filepath.Join("tests", "contracts", "engine_contract_v1.json"),
	}
	var data []byte
	var err error
	for _, path := range paths {
		data, err = os.ReadFile(path)
		if err == nil {
			break
		}
	}
	if err != nil {
		t.Fatalf("read shared engine contract: %v", err)
	}
	var fixture map[string]interface{}
	if err := json.Unmarshal(data, &fixture); err != nil {
		t.Fatalf("decode shared engine contract: %v", err)
	}
	return fixture
}

func loadTalibCoverageMatrix(t *testing.T) map[string]interface{} {
	t.Helper()
	paths := []string{
		filepath.Join("..", "..", "..", "..", "tests", "contracts", "talib_coverage_matrix_v1.json"),
		filepath.Join("..", "..", "..", "tests", "contracts", "talib_coverage_matrix_v1.json"),
		filepath.Join("tests", "contracts", "talib_coverage_matrix_v1.json"),
	}
	var data []byte
	var err error
	for _, path := range paths {
		data, err = os.ReadFile(path)
		if err == nil {
			break
		}
	}
	if err != nil {
		t.Fatalf("read TA-Lib coverage matrix: %v", err)
	}
	var fixture map[string]interface{}
	if err := json.Unmarshal(data, &fixture); err != nil {
		t.Fatalf("decode TA-Lib coverage matrix: %v", err)
	}
	return fixture
}

func asFloatSlice(t *testing.T, value interface{}) []float64 {
	t.Helper()
	values, ok := value.([]interface{})
	if !ok {
		t.Fatalf("expected JSON array, got %T", value)
	}
	result := make([]float64, len(values))
	for i, item := range values {
		number, ok := item.(float64)
		if !ok {
			t.Fatalf("expected numeric JSON value at %d, got %T", i, item)
		}
		result[i] = number
	}
	return result
}

func assertContractSeries(t *testing.T, actual interface{}, expected interface{}) {
	t.Helper()
	actualValues, ok := actual.([]interface{})
	if !ok {
		t.Fatalf("expected actual JSON series, got %T", actual)
	}
	expectedValues, ok := expected.([]interface{})
	if !ok {
		t.Fatalf("expected fixture JSON series, got %T", expected)
	}
	if len(actualValues) != len(expectedValues) {
		t.Fatalf("series length mismatch: got %d want %d", len(actualValues), len(expectedValues))
	}
	for i := range expectedValues {
		if expectedValues[i] == nil {
			if actualValues[i] != nil {
				t.Fatalf("series[%d]: got %v want null", i, actualValues[i])
			}
			continue
		}
		got, ok := actualValues[i].(float64)
		want, wantOK := expectedValues[i].(float64)
		if !ok || !wantOK || math.Abs(got-want) > 1e-12 {
			t.Fatalf("series[%d]: got %v want %v", i, actualValues[i], expectedValues[i])
		}
	}
}

func decodeContractResult(t *testing.T, result string) map[string]interface{} {
	t.Helper()
	var payload map[string]interface{}
	if err := json.Unmarshal([]byte(result), &payload); err != nil {
		t.Fatalf("decode binding result: %v", err)
	}
	if payload["error"] != nil {
		t.Fatalf("binding returned error: %s", result)
	}
	return payload
}

func TestSharedEngineContractV1(t *testing.T) {
	fixture := loadEngineContract(t)

	operation := fixture["operation"].(map[string]interface{})
	operationRequest, err := json.Marshal(operation["request"])
	if err != nil {
		t.Fatal(err)
	}
	operationResult, err := OperationExecuteJSON(string(operationRequest))
	if err != nil {
		t.Fatalf("operation execution failed: %v", err)
	}
	assertContractSeries(t, decodeContractResult(t, operationResult)["values"].(map[string]interface{})["SMA"], operation["expected_primary"])

	formula := fixture["formula"].(map[string]interface{})
	formulaResult, err := FormulaEvalContractJSON(
		formula["source"].(string), formula["dialect"].(string),
		asFloatSlice(t, formula["open"]), asFloatSlice(t, formula["high"]),
		asFloatSlice(t, formula["low"]), asFloatSlice(t, formula["close"]),
		asFloatSlice(t, formula["volume"]),
	)
	if err != nil {
		t.Fatalf("formula execution failed: %v", err)
	}
	assertContractSeries(t, decodeContractResult(t, formulaResult)["values"].(map[string]interface{})["__PRIMARY__"], formula["expected_primary"])

	factor := fixture["factor"].(map[string]interface{})
	factorRequest, _ := json.Marshal(factor["request"])
	factorResult, err := FactorExecuteJSON(string(factorRequest))
	if err != nil {
		t.Fatalf("factor execution failed: %v", err)
	}
	assertContractSeries(t, decodeContractResult(t, factorResult)["values"].(map[string]interface{})["momentum_5"], factor["expected_primary"])

	composite := fixture["composite"].(map[string]interface{})
	compositeRequest, _ := json.Marshal(composite["request"])
	compositeResult, err := CompositeExecuteJSON(string(compositeRequest))
	if err != nil {
		t.Fatalf("composite execution failed: %v", err)
	}
	assertContractSeries(t, decodeContractResult(t, compositeResult)["values"].(map[string]interface{})["sma3"], composite["expected_primary"])
}

func TestCurrentTalibCatalogContract(t *testing.T) {
	matrix := loadTalibCoverageMatrix(t)
	catalogJSON, err := OperationCatalogJSON()
	if err != nil {
		t.Fatalf("operation catalog failed: %v", err)
	}
	var catalog map[string]interface{}
	if err := json.Unmarshal([]byte(catalogJSON), &catalog); err != nil {
		t.Fatalf("decode operation catalog: %v", err)
	}
	profile := matrix["semantic_profile"].(string)
	if profile != "talib_0_8_0" {
		t.Fatalf("unexpected TA-Lib profile: %s", profile)
	}
	expected := make(map[string]bool)
	for _, item := range matrix["surfaces"].(map[string]interface{})["numeric_reference"].(map[string]interface{})["indicators"].([]interface{}) {
		expected[item.(string)] = true
	}
	seen := make(map[string]bool)
	for _, item := range catalog["operations"].([]interface{}) {
		operation := item.(map[string]interface{})
		for _, value := range operation["semantic_profiles"].([]interface{}) {
			if value.(string) == profile {
				seen[operation["name"].(string)] = true
			}
		}
	}
	if len(seen) != int(matrix["surfaces"].(map[string]interface{})["dispatcher_smoke"].(map[string]interface{})["expected_count"].(float64)) {
		t.Fatalf("TA-Lib catalog count: got %d want %d", len(seen), len(expected))
	}
	for name := range expected {
		if !seen[name] {
			t.Fatalf("TA-Lib catalog is missing %s", name)
		}
	}
}

func TestVersion(t *testing.T) {
	v := Version()
	if v == "" {
		t.Error("Version should not be empty")
	}
}

func TestSma(t *testing.T) {
	input := []float64{1, 2, 3, 4, 5, 6, 7, 8, 9, 10}
	result, err := Sma(input, 3)
	if err != nil {
		t.Fatalf("Sma failed: %v", err)
	}
	if len(result) != 10 {
		t.Errorf("expected length 10, got %d", len(result))
	}
	if math.IsNaN(result[0]) || math.IsNaN(result[1]) {
		t.Log("First values are NaN as expected")
	}
}

func TestEma(t *testing.T) {
	input := []float64{1, 2, 3, 4, 5, 6, 7, 8, 9, 10}
	result, err := Ema(input, 3)
	if err != nil {
		t.Fatalf("Ema failed: %v", err)
	}
	if len(result) != 10 {
		t.Errorf("expected length 10, got %d", len(result))
	}
}

func TestRsi(t *testing.T) {
	input := []float64{44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 45.5, 45.5, 45.5, 46.0, 45.75, 46.25, 45.5, 45.25, 46.0, 46.25, 47.0, 47.0, 47.25, 48.25}
	result, err := Rsi(input, 14)
	if err != nil {
		t.Fatalf("Rsi failed: %v", err)
	}
	if len(result) != 20 {
		t.Errorf("expected length 20, got %d", len(result))
	}
	if result[14] <= 0 || result[14] > 100 {
		t.Errorf("RSI value should be between 0 and 100, got %f", result[14])
	}
}

func TestMacd(t *testing.T) {
	input := make([]float64, 35)
	for i := range input {
		input[i] = float64(i + 1)
	}
	result, err := Macd(input, 12, 26, 9)
	if err != nil {
		t.Fatalf("Macd failed: %v", err)
	}
	if len(result.Macd) != 35 {
		t.Errorf("expected MACD length 35, got %d", len(result.Macd))
	}
	if len(result.Signal) != 35 {
		t.Errorf("expected Signal length 35, got %d", len(result.Signal))
	}
	if len(result.Hist) != 35 {
		t.Errorf("expected Hist length 35, got %d", len(result.Hist))
	}
}

func TestBbands(t *testing.T) {
	input := []float64{1, 2, 3, 4, 5, 6, 7, 8, 9, 10}
	result, err := Bbands(input, 5, 2.0, 2.0)
	if err != nil {
		t.Fatalf("Bbands failed: %v", err)
	}
	if len(result.Upper) != 10 {
		t.Errorf("expected Upper length 10, got %d", len(result.Upper))
	}
	if len(result.Middle) != 10 {
		t.Errorf("expected Middle length 10, got %d", len(result.Middle))
	}
	if len(result.Lower) != 10 {
		t.Errorf("expected Lower length 10, got %d", len(result.Lower))
	}
	if !math.IsNaN(result.Middle[0]) {
		t.Error("first middle value should be NaN")
	}
}

func TestStoch(t *testing.T) {
	high := []float64{10, 12, 14, 16, 18, 17, 16, 15}
	low := []float64{8, 10, 12, 14, 16, 15, 14, 13}
	close := []float64{9, 11, 13, 15, 17, 16, 15, 14}
	result, err := Stoch(high, low, close, 5, 1, 3)
	if err != nil {
		t.Fatalf("Stoch failed: %v", err)
	}
	if len(result.K) != 8 {
		t.Errorf("expected K length 8, got %d", len(result.K))
	}
	if len(result.D) != 8 {
		t.Errorf("expected D length 8, got %d", len(result.D))
	}
}

func TestAroon(t *testing.T) {
	high := []float64{10, 11, 12, 13, 14, 13, 12, 11}
	low := []float64{8, 9, 10, 11, 12, 11, 10, 9}
	result, err := Aroon(high, low, 5)
	if err != nil {
		t.Fatalf("Aroon failed: %v", err)
	}
	if len(result.AroonUp) != 8 {
		t.Errorf("expected AroonUp length 8, got %d", len(result.AroonUp))
	}
	if len(result.AroonDown) != 8 {
		t.Errorf("expected AroonDown length 8, got %d", len(result.AroonDown))
	}
}

func TestAdx(t *testing.T) {
	high := []float64{10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30, 32, 34, 36, 38, 40}
	low := []float64{8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30, 32, 34, 36, 38}
	close := []float64{9, 11, 13, 15, 17, 19, 21, 23, 25, 27, 29, 31, 33, 35, 37, 39}
	result, err := Adx(high, low, close, 5)
	if err != nil {
		t.Fatalf("Adx failed: %v", err)
	}
	if len(result) != 16 {
		t.Errorf("expected ADX length 16, got %d", len(result))
	}
}

func TestAtr(t *testing.T) {
	high := []float64{10, 12, 14, 16, 18, 20, 22}
	low := []float64{8, 10, 12, 14, 16, 18, 20}
	close := []float64{9, 11, 13, 15, 17, 19, 21}
	result, err := Atr(high, low, close, 5)
	if err != nil {
		t.Fatalf("Atr failed: %v", err)
	}
	if len(result) != 7 {
		t.Errorf("expected ATR length 7, got %d", len(result))
	}
}

func TestObv(t *testing.T) {
	close := []float64{10, 11, 10, 12}
	volume := []float64{100, 200, 150, 300}
	result, err := Obv(close, volume)
	if err != nil {
		t.Fatalf("Obv failed: %v", err)
	}
	if len(result) != 4 {
		t.Errorf("expected OBV length 4, got %d", len(result))
	}
	if result[0] != 100 {
		t.Errorf("expected OBV[0]=100, got %f", result[0])
	}
	if result[1] != 300 {
		t.Errorf("expected OBV[1]=300, got %f", result[1])
	}
}

func TestCci(t *testing.T) {
	high := []float64{10, 12, 14, 16, 18, 17, 16, 15}
	low := []float64{8, 10, 12, 14, 16, 15, 14, 13}
	close := []float64{9, 11, 13, 15, 17, 16, 15, 14}
	result, err := Cci(high, low, close, 5)
	if err != nil {
		t.Fatalf("Cci failed: %v", err)
	}
	if len(result) != 8 {
		t.Errorf("expected CCI length 8, got %d", len(result))
	}
}

func TestMom(t *testing.T) {
	input := []float64{1, 2, 3, 4, 5}
	result, err := Mom(input, 2)
	if err != nil {
		t.Fatalf("Mom failed: %v", err)
	}
	if len(result) != 5 {
		t.Errorf("expected MOM length 5, got %d", len(result))
	}
	if result[2] != 2.0 {
		t.Errorf("expected MOM[2]=2.0, got %f", result[2])
	}
}

func TestRoc(t *testing.T) {
	input := []float64{10, 12, 15}
	result, err := Roc(input, 1)
	if err != nil {
		t.Fatalf("Roc failed: %v", err)
	}
	if len(result) != 3 {
		t.Errorf("expected ROC length 3, got %d", len(result))
	}
	if result[1] != 20.0 {
		t.Errorf("expected ROC[1]=20.0, got %f", result[1])
	}
}

func TestWillr(t *testing.T) {
	high := []float64{10, 12, 14, 16, 18}
	low := []float64{8, 10, 12, 14, 16}
	close := []float64{9, 11, 13, 15, 17}
	result, err := Willr(high, low, close, 3)
	if err != nil {
		t.Fatalf("Willr failed: %v", err)
	}
	if len(result) != 5 {
		t.Errorf("expected WillR length 5, got %d", len(result))
	}
}

func TestTrange(t *testing.T) {
	high := []float64{10, 12, 14}
	low := []float64{8, 10, 12}
	close := []float64{9, 11, 13}
	result, err := Trange(high, low, close)
	if err != nil {
		t.Fatalf("Trange failed: %v", err)
	}
	if len(result) != 3 {
		t.Errorf("expected TRange length 3, got %d", len(result))
	}
}

func TestNatr(t *testing.T) {
	high := []float64{10, 12, 14, 16, 18, 20, 22}
	low := []float64{8, 10, 12, 14, 16, 18, 20}
	close := []float64{9, 11, 13, 15, 17, 19, 21}
	result, err := Natr(high, low, close, 5)
	if err != nil {
		t.Fatalf("Natr failed: %v", err)
	}
	if len(result) != 7 {
		t.Errorf("expected NATR length 7, got %d", len(result))
	}
}

func TestHtDcPeriod(t *testing.T) {
	input := make([]float64, 100)
	for i := range input {
		input[i] = math.Sin(float64(i)*0.1)*1.0 + 50.0
	}
	result, err := HtDcPeriod(input)
	if err != nil {
		t.Fatalf("HtDcPeriod failed: %v", err)
	}
	if len(result) != 100 {
		t.Errorf("expected HtDcPeriod length 100, got %d", len(result))
	}
}

func TestHtDcPhase(t *testing.T) {
	input := make([]float64, 100)
	for i := range input {
		input[i] = math.Sin(float64(i)*0.1)*1.0 + 50.0
	}
	result, err := HtDcPhase(input)
	if err != nil {
		t.Fatalf("HtDcPhase failed: %v", err)
	}
	if len(result) != 100 {
		t.Errorf("expected HtDcPhase length 100, got %d", len(result))
	}
}

func TestHtPhasor(t *testing.T) {
	input := make([]float64, 50)
	for i := range input {
		input[i] = math.Sin(float64(i)*0.1)*1.0 + 50.0
	}
	result, err := HtPhasor(input)
	if err != nil {
		t.Fatalf("HtPhasor failed: %v", err)
	}
	if len(result.InPhase) != 50 {
		t.Errorf("expected InPhase length 50, got %d", len(result.InPhase))
	}
	if len(result.Quadrature) != 50 {
		t.Errorf("expected Quadrature length 50, got %d", len(result.Quadrature))
	}
}

func TestHtSine(t *testing.T) {
	input := make([]float64, 100)
	for i := range input {
		input[i] = math.Sin(float64(i)*0.1)*1.0 + 50.0
	}
	result, err := HtSine(input)
	if err != nil {
		t.Fatalf("HtSine failed: %v", err)
	}
	if len(result.Sine) != 100 {
		t.Errorf("expected Sine length 100, got %d", len(result.Sine))
	}
	if len(result.LeadSine) != 100 {
		t.Errorf("expected LeadSine length 100, got %d", len(result.LeadSine))
	}
}

func TestHtTrendMode(t *testing.T) {
	input := make([]float64, 100)
	for i := range input {
		input[i] = math.Sin(float64(i)*0.1)*1.0 + 50.0
	}
	result, err := HtTrendMode(input)
	if err != nil {
		t.Fatalf("HtTrendMode failed: %v", err)
	}
	if len(result) != 100 {
		t.Errorf("expected HtTrendMode length 100, got %d", len(result))
	}
}

func TestHtTrendLine(t *testing.T) {
	input := make([]float64, 100)
	for i := range input {
		input[i] = math.Sin(float64(i)*0.1)*1.0 + 50.0
	}
	result, err := HtTrendLine(input)
	if err != nil {
		t.Fatalf("HtTrendLine failed: %v", err)
	}
	if len(result) != 100 {
		t.Errorf("expected HtTrendLine length 100, got %d", len(result))
	}
}

func TestZScore(t *testing.T) {
	input := []float64{1, 2, 3, 4, 5, 6, 7, 8, 9, 10}
	result, err := ZScore(input, 5)
	if err != nil {
		t.Fatalf("ZScore failed: %v", err)
	}
	if len(result) != 10 {
		t.Errorf("expected ZScore length 10, got %d", len(result))
	}
}

func TestBeta(t *testing.T) {
	benchmark := []float64{100, 101, 102, 103, 104, 105, 106, 107}
	asset := []float64{100, 102, 104, 106, 108, 110, 112, 114}
	result, err := Beta(asset, benchmark, 5)
	if err != nil {
		t.Fatalf("Beta failed: %v", err)
	}
	if len(result) != 8 {
		t.Errorf("expected Beta length 8, got %d", len(result))
	}
}

func TestCorrelation(t *testing.T) {
	x := []float64{1, 2, 3, 4, 5}
	y := []float64{2, 4, 6, 8, 10}
	result, err := Correlation(x, y, 3)
	if err != nil {
		t.Fatalf("Correlation failed: %v", err)
	}
	if len(result) != 5 {
		t.Errorf("expected Correlation length 5, got %d", len(result))
	}
}

func TestStdDev(t *testing.T) {
	input := []float64{2, 4, 4, 4, 5, 5, 7, 9}
	result, err := StdDev(input, 5, 1.0)
	if err != nil {
		t.Fatalf("StdDev failed: %v", err)
	}
	if len(result) != 8 {
		t.Errorf("expected StdDev length 8, got %d", len(result))
	}
}

func TestLinearReg(t *testing.T) {
	input := []float64{1, 2, 3, 4, 5}
	result, err := LinearReg(input, 3)
	if err != nil {
		t.Fatalf("LinearReg failed: %v", err)
	}
	if len(result) != 5 {
		t.Errorf("expected LinearReg length 5, got %d", len(result))
	}
}

func TestTsf(t *testing.T) {
	input := []float64{1, 2, 3, 4, 5, 6, 7, 8, 9, 10}
	result, err := Tsf(input, 5)
	if err != nil {
		t.Fatalf("Tsf failed: %v", err)
	}
	if len(result) != 10 {
		t.Errorf("expected TSF length 10, got %d", len(result))
	}
}

func TestWma(t *testing.T) {
	input := []float64{1, 2, 3, 4, 5, 6, 7, 8, 9, 10}
	result, err := Wma(input, 3)
	if err != nil {
		t.Fatalf("Wma failed: %v", err)
	}
	if len(result) != 10 {
		t.Errorf("expected Wma length 10, got %d", len(result))
	}
}

func TestDema(t *testing.T) {
	input := []float64{1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15}
	result, err := Dema(input, 5)
	if err != nil {
		t.Fatalf("Dema failed: %v", err)
	}
	if len(result) != 15 {
		t.Errorf("expected Dema length 15, got %d", len(result))
	}
}

func TestTema(t *testing.T) {
	input := []float64{1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20}
	result, err := Tema(input, 5)
	if err != nil {
		t.Fatalf("Tema failed: %v", err)
	}
	if len(result) != 20 {
		t.Errorf("expected Tema length 20, got %d", len(result))
	}
}

func TestKama(t *testing.T) {
	input := []float64{1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15}
	result, err := Kama(input, 5)
	if err != nil {
		t.Fatalf("Kama failed: %v", err)
	}
	if len(result) != 15 {
		t.Errorf("expected Kama length 15, got %d", len(result))
	}
}

func TestT3(t *testing.T) {
	input := []float64{1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
		21, 22, 23, 24, 25, 26, 27, 28, 29, 30}
	result, err := T3(input, 5, 0.7)
	if err != nil {
		t.Fatalf("T3 failed: %v", err)
	}
	if len(result) != 30 {
		t.Errorf("expected T3 length 30, got %d", len(result))
	}
}

func TestAd(t *testing.T) {
	high := []float64{10, 12, 14}
	low := []float64{8, 10, 12}
	close := []float64{9, 11, 13}
	volume := []float64{100, 200, 300}
	result, err := Ad(high, low, close, volume)
	if err != nil {
		t.Fatalf("Ad failed: %v", err)
	}
	if len(result) != 3 {
		t.Errorf("expected AD length 3, got %d", len(result))
	}
}

func TestAdOsc(t *testing.T) {
	high := []float64{10, 12, 14, 16, 18, 20, 22, 24}
	low := []float64{8, 10, 12, 14, 16, 18, 20, 22}
	close := []float64{9, 11, 13, 15, 17, 19, 21, 23}
	volume := []float64{100, 200, 300, 400, 500, 600, 700, 800}
	result, err := AdOsc(high, low, close, volume, 3, 6)
	if err != nil {
		t.Fatalf("AdOsc failed: %v", err)
	}
	if len(result) != 8 {
		t.Errorf("expected ADOsc length 8, got %d", len(result))
	}
}

func TestEmptyInput(t *testing.T) {
	_, err := Sma([]float64{}, 3)
	if err == nil {
		t.Error("expected error for empty input")
	}
}

func TestNilInput(t *testing.T) {
	_, err := Sma(nil, 3)
	if err == nil {
		t.Error("expected error for nil input")
	}
}

func TestInvalidPeriod(t *testing.T) {
	input := []float64{1, 2, 3}
	_, err := Sma(input, 0)
	if err == nil {
		t.Error("expected error for period=0")
	}
}

func TestInsufficientData(t *testing.T) {
	input := []float64{1, 2}
	_, err := Sma(input, 10)
	if err == nil {
		t.Error("expected error for insufficient data")
	}
}

func TestStreamingSma(t *testing.T) {
	sma := NewStreamingSma(3)
	defer sma.Free()

	input := []float64{1, 2, 3, 4, 5, 6, 7, 8, 9, 10}
	var results []float64
	for _, v := range input {
		results = append(results, sma.Update(v))
	}

	if len(results) != 10 {
		t.Errorf("expected 10 results, got %d", len(results))
	}
	if !math.IsNaN(results[0]) || !math.IsNaN(results[1]) {
		t.Log("First two values are NaN as expected")
	}
	if math.IsNaN(results[2]) {
		t.Error("Third value should not be NaN")
	}

	sma.Reset()
	resetResult := sma.Update(1.0)
	if !math.IsNaN(resetResult) {
		t.Error("After reset, first update should return NaN")
	}
}

func TestStreamingEma(t *testing.T) {
	ema := NewStreamingEma(3)
	defer ema.Free()

	input := []float64{1, 2, 3, 4, 5}
	var results []float64
	for _, v := range input {
		results = append(results, ema.Update(v))
	}

	if len(results) != 5 {
		t.Errorf("expected 5 results, got %d", len(results))
	}
}

func TestStreamingRsi(t *testing.T) {
	rsi := NewStreamingRsi(14)
	defer rsi.Free()

	input := make([]float64, 20)
	for i := range input {
		input[i] = float64(i + 44)
	}
	var results []float64
	for _, v := range input {
		results = append(results, rsi.Update(v))
	}

	if len(results) != 20 {
		t.Errorf("expected 20 results, got %d", len(results))
	}
	if results[14] <= 0 || results[14] > 100 {
		t.Errorf("RSI value should be between 0 and 100, got %f", results[14])
	}
}

func TestStreamingMacd(t *testing.T) {
	macd := NewStreamingMacd(12, 26, 9)
	defer macd.Free()

	input := make([]float64, 40)
	for i := range input {
		input[i] = float64(i + 1)
	}
	var macdResults, signalResults, histResults []float64
	for _, v := range input {
		out, _ := macd.Update(v)
		macdResults = append(macdResults, out.Macd)
		signalResults = append(signalResults, out.Signal)
		histResults = append(histResults, out.Hist)
	}

	if len(macdResults) != 40 {
		t.Errorf("expected 40 MACD results, got %d", len(macdResults))
	}
	if len(signalResults) != 40 {
		t.Errorf("expected 40 Signal results, got %d", len(signalResults))
	}
	if len(histResults) != 40 {
		t.Errorf("expected 40 Hist results, got %d", len(histResults))
	}
}

func TestStreamingMacdExtVariants(t *testing.T) {
	macd := NewStreamingMacdExt(5, MaSMA, 10, MaWMA, 3, MaTEMA)
	if macd == nil {
		t.Fatal("NewStreamingMacdExt returned nil")
	}
	defer macd.Free()
	ready := 0
	for i := 0; i < 50; i++ {
		if _, ok := macd.Update(50 + math.Sin(float64(i)/3)); ok {
			ready++
		}
	}
	if ready == 0 {
		t.Fatal("MACDEXT never became ready")
	}
	if NewStreamingMacdExt(12, MaType(99), 26, MaEMA, 9, MaEMA) != nil {
		t.Fatal("unsupported MA type must return nil")
	}
}

func TestStreamingBbands(t *testing.T) {
	bbands := NewStreamingBbands(5, 2.0, 2.0)
	defer bbands.Free()

	input := []float64{1, 2, 3, 4, 5, 6, 7, 8, 9, 10}
	var upperResults, middleResults, lowerResults []float64
	for _, v := range input {
		out, _ := bbands.Update(v)
		upperResults = append(upperResults, out.Upper)
		middleResults = append(middleResults, out.Middle)
		lowerResults = append(lowerResults, out.Lower)
	}

	if len(upperResults) != 10 {
		t.Errorf("expected 10 Upper results, got %d", len(upperResults))
	}
	if len(middleResults) != 10 {
		t.Errorf("expected 10 Middle results, got %d", len(middleResults))
	}
	if len(lowerResults) != 10 {
		t.Errorf("expected 10 Lower results, got %d", len(lowerResults))
	}
}

func TestStreamingAtr(t *testing.T) {
	atr := NewStreamingAtr(5)
	defer atr.Free()

	high := []float64{10, 12, 14, 16, 18, 20, 22}
	low := []float64{8, 10, 12, 14, 16, 18, 20}
	close := []float64{9, 11, 13, 15, 17, 19, 21}

	var results []float64
	for i := range high {
		results = append(results, atr.Update(high[i], low[i], close[i]))
	}

	if len(results) != 7 {
		t.Errorf("expected 7 results, got %d", len(results))
	}
	if !math.IsNaN(results[0]) {
		t.Error("First value should be NaN")
	}
}
