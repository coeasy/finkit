import test from 'node:test'
import assert from 'node:assert/strict'

import { createFinkitLightweightChart } from './lightweight-charts-adapter.js'

function fakeChartEnvironment() {
  const element = { id: 'chart' }
  const state = { added: [], removed: [], range: null }
  const chart = {
    addSeries(type, options) {
      const series = {
        type,
        options,
        data: [],
        updates: [],
        markers: [],
        setData(data) { this.data = data },
        update(point) { this.updates.push(point) },
        setMarkers(markers) { this.markers = markers },
      }
      state.added.push(series)
      return series
    },
    removeSeries(series) { state.removed.push(series) },
    timeScale() {
      return { setVisibleLogicalRange(range) { state.range = range } }
    },
  }
  const lightweightCharts = {
    CandlestickSeries: {},
    HistogramSeries: {},
    LineSeries: {},
    createChart() { return chart },
  }
  globalThis.document = { getElementById() { return element } }
  return { chart, lightweightCharts, state }
}

function payload(lines = []) {
  return {
    schema_version: 1,
    revision: 7,
    candles: [
      { time: 1, open: 1, high: 2, low: 0, close: 1.5 },
      { time: 2, open: 1.5, high: 3, low: 1, close: 2.5 },
    ],
    volume: [
      { time: 1, value: 10, color: '#26a69a' },
      { time: 2, value: 12, color: '#26a69a' },
    ],
    lines,
    scene: {
      panels: [],
      layers: [],
      markers: [{ time: 2, position: 'aboveBar', shape: 'arrowUp', color: '#fff' }],
      viewport: { start: 0, end: 2, follow_latest: true },
    },
  }
}

test('maps payloads and supports incremental creation of new lines', () => {
  const { lightweightCharts, state } = fakeChartEnvironment()
  const initial = payload([{
    name: 'sma',
    data: [{ time: 1, value: null }, { time: 2, value: 2.0 }],
  }])
  const view = createFinkitLightweightChart('chart', initial, lightweightCharts)

  assert.equal(view.lines.size, 1)
  assert.deepEqual(view.lines.get('sma').data[0], { time: 1 })
  assert.deepEqual(view.candle.markers, initial.scene.markers)
  assert.deepEqual(state.range, { from: 0, to: 1 })

  view.update(payload([{
    name: 'ema',
    data: [{ time: 2, value: 2.5 }],
  }]))
  assert.equal(view.lines.size, 2)
  assert.deepEqual(view.lines.get('ema').updates, [{ time: 2, value: 2.5 }])
})

test('removes lines on full payload replacement and rejects wrong schema', () => {
  const { lightweightCharts, state } = fakeChartEnvironment()
  const view = createFinkitLightweightChart(
    'chart',
    payload([{ name: 'sma', data: [{ time: 1, value: 1 }, { time: 2, value: 2 }] }]),
    lightweightCharts,
  )
  view.setPayload(payload([]))
  assert.equal(view.lines.size, 0)
  assert.equal(state.removed.length, 1)
  assert.throws(() => view.setPayload({ schema_version: 99 }), /Unsupported/)
})
