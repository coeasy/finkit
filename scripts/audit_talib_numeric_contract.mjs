import { readFileSync } from 'node:fs'
import * as finkit from '../ffi/node-binding/index.mjs'

const fixture = JSON.parse(readFileSync(new URL('../tests/contracts/talib_numeric_contract_v1.json', import.meta.url), 'utf8'))
const failures = []

for (const vector of fixture.vectors) {
  const payload = JSON.parse(finkit.operationExecuteJson(JSON.stringify({
    operation: vector.operation,
    semantic_profile: fixture.semantic_profile,
    input_order: vector.input_order,
    inputs: fixture.inputs,
    params: vector.params,
  })))
  if (payload.error) {
    failures.push({ operation: vector.operation, kind: 'error', detail: payload.error })
    continue
  }
  for (const [output, expected] of Object.entries(vector.expected)) {
    const actual = payload.values[output]
    if (!actual || actual.length !== expected.length) {
      failures.push({ operation: vector.operation, output, kind: 'shape' })
      continue
    }
    const atol = vector.tolerance.atol
    const rtol = vector.tolerance.rtol
    for (let index = 0; index < expected.length; index += 1) {
      const want = expected[index]
      const got = actual[index]
      const mismatch = want === null
        ? got !== null
        : got === null || Math.abs(got - want) > atol + rtol * Math.abs(want)
      if (mismatch) {
        failures.push({ operation: vector.operation, output, index, expected: want, actual: got })
        break
      }
    }
  }
}

console.log(JSON.stringify({ vectors: fixture.vectors.length, failures: failures.length, details: failures }, null, 2))
process.exitCode = failures.length === 0 ? 0 : 1
