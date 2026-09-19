import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import * as finkit from '../index.mjs'

const require = createRequire(import.meta.url)
const commonjs = require('../index.js')
const engineContract = JSON.parse(readFileSync(new URL('../../../tests/contracts/engine_contract_v1.json', import.meta.url), 'utf8'))

test('loads the native binding and computes SMA', () => {
  const result = finkit.sma([1, 2, 3, 4, 5], 3)
  assert.equal(result.length, 5)
  assert.ok(Number.isFinite(result[4]))
  assert.ok(Math.abs(result[4] - 4) < 1e-12)
})

test('exports the complete public runtime contract', () => {
  for (const name of ['sma', 'ema', 'rsi', 'macd', 'formulaEval', 'formulaEvalContractJson', 'formulaEvalTemporalContractJson', 'formulaEvalPanelContractJson', 'formulaEvalCrossSectionalContractJson', 'formulaStreamExecuteJson', 'operationCatalogJson', 'factorCatalogJson', 'compositeExecuteJson', 'compositeStreamExecuteJson', 'factorExecuteJson', 'factorCrossSectionalExecuteJson', 'factorStreamExecuteJson', 'operationExecuteJson', 'factorStudyJson', 'quantEvaluationJson', 'formulaValidate', 'formulaAnalyze', 'formulaMetadata', 'formulaCompatibilityReport']) {
    assert.equal(typeof finkit[name], 'function', `${name} must be exported`)
  }
  assert.equal(typeof finkit.computeComposite, 'function')
})

test('keeps ESM and CommonJS export surfaces identical', () => {
  assert.deepEqual(Object.keys(finkit).sort(), Object.keys(commonjs).sort())
})

test('executes the shared engine contract vector', () => {
  const operation = engineContract.operation
  const operationResult = JSON.parse(finkit.operationExecuteJson(JSON.stringify(operation.request)))
  assert.deepEqual(operationResult.values.SMA, operation.expected_primary)

  const formula = engineContract.formula
  const formulaResult = JSON.parse(finkit.formulaEvalContractJson(
    formula.source,
    formula.dialect,
    formula.open,
    formula.high,
    formula.low,
    formula.close,
    formula.volume,
  ))
  assert.deepEqual(formulaResult.values.__PRIMARY__, formula.expected_primary)

  const factor = engineContract.factor
  const factorResult = JSON.parse(finkit.factorExecuteJson(JSON.stringify(factor.request)))
  assert.deepEqual(factorResult.values.momentum_5, factor.expected_primary)

  const composite = engineContract.composite
  const compositeResult = JSON.parse(finkit.compositeExecuteJson(JSON.stringify(composite.request)))
  assert.deepEqual(compositeResult.values.sma3, composite.expected_primary)

  const compositeMultiInput = engineContract.composite_multi_input
  const compositeMultiResult = JSON.parse(finkit.compositeExecuteJson(JSON.stringify(compositeMultiInput.request)))
  assert.equal(compositeMultiResult.shape, 'multi_series')
  assert.equal(compositeMultiResult.primary, null)
  for (const [name, expected] of Object.entries(compositeMultiInput.expected)) {
    assert.deepEqual(compositeMultiResult.values[name], expected)
  }
})

test('publishes the current TA-Lib catalog contract', () => {
  const matrix = JSON.parse(readFileSync(new URL('../../../tests/contracts/talib_coverage_matrix_v1.json', import.meta.url), 'utf8'))
  const catalog = JSON.parse(finkit.operationCatalogJson())
  const talibNames = catalog.operations
    .filter((operation) => operation.semantic_profiles.includes(matrix.semantic_profile))
    .map((operation) => operation.name)
    .sort()

  assert.equal(matrix.semantic_profile, 'talib_0_8_0')
  assert.equal(talibNames.length, matrix.surfaces.dispatcher_smoke.expected_count)
  assert.deepEqual(talibNames, [...matrix.surfaces.numeric_reference.indicators].sort())
  assert.equal(catalog.operations.find((operation) => operation.name === 'SMA').semantic_profiles.includes('talib_0_7_1'), false)
})

test('executes Pine request.security through explicit temporal provider data', () => {
  const payload = JSON.parse(finkit.formulaEvalTemporalContractJson(JSON.stringify({
    schema_version: 1,
    source: '//@version=5\nindicator("HTF")\nhtf = request.security(syminfo.tickerid, "D", close)\nhtf',
    dialect: 'pine',
    frame: {
      symbol: 'AAA',
      timeframe: '1m',
      timestamps: [10, 20, 30, 40],
      open: [1, 2, 3, 4],
      high: [1, 2, 3, 4],
      low: [1, 2, 3, 4],
      close: [1, 2, 3, 4],
      volume: [10, 20, 30, 40],
    },
    security: [{
      symbol: 'AAA',
      timeframe: 'D',
      expression: 'close',
      timestamps: [10, 30],
      values: [100, 300],
      alignment: 'as_of_closed',
    }],
  })))
  assert.deepEqual(payload.values.__PRIMARY__, [100, 100, 300, 300])
  assert.equal(payload.security[0].timeframe, 'D')
})

test('supports configurable streaming MACDEXT MA variants', () => {
  const macd = new finkit.NapiStreamingMacdExt(5, 'sma', 10, 'wma', 3, 'tema')
  const values = macd.updateBatch(Array.from({ length: 40 }, (_, i) => 50 + Math.sin(i / 3)))
  assert.equal(values.length, 40)
  assert.equal(macd.count, 40)
  assert.ok(values.some((value) => Number.isFinite(value.macd)))
  assert.throws(() => new finkit.NapiStreamingMacdExt(12, 'mama', 26, 'ema', 9, 'ema'))
})

test('supports persistent composable formula components', () => {
  const registry = new finkit.FormulaRegistryNapi()
  registry.register('ZMA', ['X', 'N'], 'MA(X, N) + EMA(X, N)')
  assert.deepEqual(registry.names(), ['ZMA'])
  const data = [1, 2, 3, 4, 5, 6]
  const result = registry.eval('zma(CLOSE, 3)', data, data, data, data, data)
  assert.equal(result.__result__.length, data.length)
  assert.ok(Number.isFinite(result.__result__[5]))
  assert.equal(registry.unregister('zma'), true)
})

test('evaluates dependency-aware composite indicators', () => {
  const result = finkit.computeComposite(
    [1, 2, 3, 4, 5, 6],
    [
      { name: 'trend', function: 'sma', inputs: ['close'], params: [3] },
      { name: 'signal', function: 'cross_up', inputs: ['close', 'trend'], params: [] },
    ],
    ['trend', 'signal'],
  )
  assert.deepEqual(result.trend.slice(-2), [4, 5])
  assert.equal(result.signal.length, 6)
})

test('preserves weighted-average parameters in composite indicators', () => {
  const result = finkit.computeComposite(
    [10, 20],
    [{ name: 'weighted', function: 'weighted_average', inputs: ['close', 'const:20'], params: [1, 3] }],
    ['weighted'],
  )
  assert.deepEqual(result.weighted, [17.5, 20])
})

test('supports threshold and rolling composite operators', () => {
  const result = finkit.computeComposite(
    [1, 2, 3, 2, 4],
    [
      { name: 'above', function: 'threshold', inputs: ['close'], params: [3] },
      { name: 'range', function: 'between', inputs: ['close'], params: [2, 3] },
      { name: 'vol', function: 'rolling_std', inputs: ['close'], params: [3] },
    ],
    ['above', 'range', 'vol'],
  )
  assert.deepEqual(result.above, [0, 0, 1, 0, 1])
  assert.deepEqual(result.range, [0, 1, 1, 1, 0])
  assert.ok(Number.isFinite(result.vol[2]))
})

test('resolves configurable exchange calendars and timezone sessions', () => {
  const aShareOpen = 1704418200 // 2024-01-05 09:30 Asia/Shanghai
  const session = finkit.resolveMarketSession('a_share', aShareOpen)
  assert.ok(session)
  assert.equal(session.sessionDay, 19727)
  assert.ok(session.closeTimestamp > session.openTimestamp)
  assert.equal(finkit.resolveMarketSession('a_share', aShareOpen + 86400), null)
  assert.equal(finkit.resolveMarketSession('a_share', aShareOpen, undefined, ['2024-01-05']), null)
  assert.equal(
    finkit.resolveMarketSession('a_share', aShareOpen, undefined, undefined, undefined, [
      { date: '2024-01-05', sessions: [] },
    ]),
    null,
  )
  const usSession = finkit.resolveMarketSession('us_equity', 1709908200)
  assert.ok(usSession)
  const versioned = finkit.resolveMarketSessionConfig(
    JSON.stringify({
      market: 'a_share',
      timezone: 'Asia/Shanghai',
      holidays: ['2026-01-01'],
      source: 'sse-official',
      revision: '2026.1',
    }),
    1767317400,
  )
  assert.equal(versioned.source, 'sse-official')
  assert.equal(versioned.revision, '2026.1')
  const annual = finkit.resolveMarketSessionCsv(
    'date,status,sessions\n2026-01-01,closed,\n2026-01-02,open,09:30-11:30;13:00-15:00\n',
    'a_share',
    1767317400,
    'Asia/Shanghai',
  )
  assert.ok(annual)
  assert.equal(annual.sessionIndex, 0)
})

test('exports the TDX-style chart data and interaction surface', () => {
  const data = finkit.klineDataNew(
    ['2026-01-01', '2026-01-02', '2026-01-03'],
    [10, 11, 12],
    [11, 12, 13],
    [9, 10, 11],
    [10.5, 11.5, 12.5],
    [100, 120, 140],
    [1704067200, 1704153600, 1704240000],
  )
  assert.equal(finkit.klineDataValidateOhlcv(data), true)
  assert.deepEqual(finkit.klineDataValidationErrors(data), [])

  const chart = new finkit.KlineChartNapi(data, 'zh', 'Node chart', 800, 420)
  chart.addCustomIndicator('signal', [1, 2, 3])
  chart.setCustomIndicator('signal', [2, 3, 4])
  chart.addEventMarker(1, '突破候选', 12, '#f59e0b', 50)
  chart.setInteraction(true, true, true, true, true)
  chart.setViewport(0, 3, 800, 420, 1, false)
  chart.setLodPolicy('auto')
  assert.equal(chart.setLayerVisible('volume', false), true)
  chart.addChan(1, true, 'aggressive', 'configurable', 'dynamic', 0, true)
  chart.setChanThresholds(0.001, 0.002, 0.1, 0.001)

  const html = chart.toHtml()
  assert.match(html, /数据窗口/)
  assert.match(html, /signal/)
  assert.match(html, /突破候选/)
  assert.match(html, /pointermove/)

  const payload = JSON.parse(chart.toJson())
  assert.equal(payload.data.dates.length, 3)
  assert.deepEqual(payload.data.timestamps, [1704067200, 1704153600, 1704240000])
  assert.equal(payload.data.source_ranges.length, 3)
  assert.equal(payload.scene.hit_regions.some((region) => region.target.type === 'event'), true)

  const canvasHtml = chart.toCanvasHtml()
  assert.match(canvasHtml, /data-renderer="canvas2d"/)
  assert.match(canvasHtml, /getContext\('2d'\)/)
  assert.match(canvasHtml, /finkit-canvas-tooltip/)
  assert.match(canvasHtml, /pointermove/)
  const webglHtml = chart.toWebglHtml()
  assert.match(webglHtml, /data-renderer="webgl2"/)
  assert.match(webglHtml, /drawArraysInstanced/)
  assert.match(webglHtml, /navigator\.gpu/)
  assert.match(webglHtml, /indicatorRows/)
  assert.match(webglHtml, /enhancedTooltip/)
  assert.match(webglHtml, /突破候选/)
  const webgpuHtml = chart.toWebgpuHtml()
  assert.match(webgpuHtml, /@vertex fn vsBody/)
  assert.match(webgpuHtml, /decodeF32/)
  assert.match(webgpuHtml, /setRingBuffer:setRingBuffer/)
  assert.match(webgpuHtml, /device\.lost/)
  assert.match(webgpuHtml, /gpuRecovery/)
  assert.equal(chart.upsertKline('2026-01-03', 12, 13, 11, 12.5, 140, 1704240000), 'updated')
  assert.equal(chart.upsertKline('2026-01-04', 12.5, 14, 12, 13.5, 160, 1704326400), 'appended')
  assert.deepEqual(chart.upsertKlines([
    { date: '2026-01-04', open: 12.5, high: 14.5, low: 12, close: 14, volume: 180 },
    { date: '2026-01-05', open: 14, high: 15, low: 13.5, close: 14.5, volume: 200 },
  ]), ['updated', 'appended'])
  assert.deepEqual(chart.setReplayWindow(2, 1), [0, 2])
  assert.deepEqual(chart.replayNext(), [1, 3])
})
