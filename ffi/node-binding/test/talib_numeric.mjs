import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import * as finkit from '../index.mjs'

const fixture = JSON.parse(readFileSync(new URL('../../../tests/contracts/talib_numeric_contract_v1.json', import.meta.url), 'utf8'))

test('executes every TA-Lib numeric contract vector', () => {
  assert.equal(fixture.semantic_profile, 'talib_0_8_0')
  assert.equal(fixture.vectors.length, 201)

  for (const vector of fixture.vectors) {
    const payload = JSON.parse(finkit.operationExecuteJson(JSON.stringify({
      operation: vector.operation,
      semantic_profile: fixture.semantic_profile,
      input_order: vector.input_order,
      inputs: fixture.inputs,
      params: vector.params,
    })))
    assert.equal(payload.error, undefined, `${vector.operation}: ${JSON.stringify(payload)}`)

    for (const [output, expected] of Object.entries(vector.expected)) {
      const actual = payload.values[output]
      assert.ok(actual, `${vector.operation}/${output}: missing output`)
      assert.equal(actual.length, expected.length, `${vector.operation}/${output}: length mismatch`)
      const atol = vector.tolerance.atol
      const rtol = vector.tolerance.rtol
      for (let index = 0; index < expected.length; index += 1) {
        if (expected[index] === null) {
          assert.equal(actual[index], null, `${vector.operation}/${output}[${index}]: expected null`)
          continue
        }
        assert.notEqual(actual[index], null, `${vector.operation}/${output}[${index}]: unexpected null`)
        const error = Math.abs(actual[index] - expected[index])
        const limit = atol + rtol * Math.abs(expected[index])
        assert.ok(error <= limit, `${vector.operation}/${output}[${index}]: error ${error} > ${limit}`)
      }
    }
  }
})
