const assert = require('node:assert/strict')
const path = require('node:path')

function main() {
  const packageDir = path.resolve(process.argv[2] || '../../../dist/wasm/nodejs')
  const modulePath = path.join(packageDir, 'finkit_wasm.js')
  const wasm = require(modulePath)
  const result = wasm.sma(new Float64Array([1, 2, 3, 4]), 3)
  assert.equal(result.length, 4)
  assert.ok(Number.isNaN(result[0]) && Number.isNaN(result[1]))
  assert.deepEqual(Array.from(result.slice(2)), [2, 3])
  console.log('installed WASM Node package SMA smoke passed')
}

try { main() } catch (error) { console.error(error); process.exitCode = 1 }
