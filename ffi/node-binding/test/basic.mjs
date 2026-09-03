import test from 'node:test'
import assert from 'node:assert/strict'
import * as finkit from '../index.mjs'

test('loads the native binding and computes SMA', () => {
  const result = finkit.sma([1, 2, 3, 4, 5], 3)
  assert.equal(result.length, 5)
  assert.ok(Number.isFinite(result[4]))
  assert.ok(Math.abs(result[4] - 4) < 1e-12)
})

test('exports core indicator and formula entry points', () => {
  for (const name of ['sma', 'ema', 'rsi', 'macd', 'formulaEval', 'formulaValidate']) {
    assert.equal(typeof finkit[name], 'function', `${name} must be exported`)
  }
})
