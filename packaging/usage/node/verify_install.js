const assert = require('node:assert/strict')
const finkit = require('finkit')

const result = finkit.sma([1, 2, 3, 4], 3)
assert.equal(result.length, 4)
assert.ok(Number.isNaN(result[0]) && Number.isNaN(result[1]))
assert.equal(result[2], 2)
assert.equal(result[3], 3)
console.log('installed Node package SMA smoke passed')
