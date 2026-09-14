#!/usr/bin/env node

/**
 * Execute the self-contained GPU chart controller without a browser or GPU.
 *
 * This deliberately mocks only the WebGL2/Canvas surface used by the generated
 * document. It is a behavioral contract test for the controller protocol, not a
 * rendering-quality test; real WebGL/WebGPU validation remains a browser matrix.
 */

import { readFile } from "node:fs/promises";
import { Script, createContext } from "node:vm";
import assert from "node:assert/strict";

const file = process.argv[2] ?? "gpu_large_chart.html";
const webgpuMode = process.argv.includes("--webgpu");
const html = await readFile(file, "utf8");
const scriptStart = html.indexOf("<script>");
const scriptEnd = html.lastIndexOf("</script>");
assert.ok(scriptStart >= 0 && scriptEnd > scriptStart, "GPU HTML must contain a script");

const noop = () => {};
const context2d = {
  globalAlpha: 1,
  strokeStyle: "",
  fillStyle: "",
  lineWidth: 1,
  font: "",
  setTransform: noop,
  clearRect: noop,
  fillRect: noop,
  strokeRect: noop,
  beginPath: noop,
  moveTo: noop,
  lineTo: noop,
  closePath: noop,
  stroke: noop,
  fill: noop,
  arc: noop,
  fillText: noop,
  setLineDash: noop,
  save: noop,
  restore: noop,
  transform: noop,
  rect: noop,
  clip: noop,
  scale: noop,
  translate: noop,
};

let nextId = 1;
const webgl = {
  ARRAY_BUFFER: 0x8892,
  DYNAMIC_DRAW: 0x88e8,
  STATIC_DRAW: 0x88e4,
  FLOAT: 0x1406,
  VERTEX_SHADER: 0x8b31,
  FRAGMENT_SHADER: 0x8b30,
  COMPILE_STATUS: 0x8b81,
  LINK_STATUS: 0x8b82,
  COLOR_BUFFER_BIT: 0x4000,
  TRIANGLES: 0x0004,
  LINES: 0x0001,
  createShader: () => ({ id: nextId++ }),
  shaderSource: noop,
  compileShader: noop,
  getShaderParameter: () => true,
  getShaderInfoLog: () => "",
  createProgram: () => ({ id: nextId++ }),
  attachShader: noop,
  linkProgram: noop,
  getProgramParameter: () => true,
  getProgramInfoLog: () => "",
  createBuffer: () => ({ id: nextId++ }),
  bindBuffer: noop,
  bufferData: noop,
  bufferSubData: noop,
  getAttribLocation: (_program, name) => (name === "aVertex" ? 0 : nextId++),
  enableVertexAttribArray: noop,
  vertexAttribPointer: noop,
  vertexAttribDivisor: noop,
  getUniformLocation: (_program, name) => ({ name }),
  uniform2f: noop,
  uniform4f: noop,
  uniform1f: noop,
  uniform1i: noop,
  uniform4fv: noop,
  viewport: noop,
  clearColor: noop,
  clear: noop,
  useProgram: noop,
  drawArraysInstanced: noop,
};

let resolveDeviceLost;
const deviceLost = new Promise((resolve) => {
  resolveDeviceLost = resolve;
});
const makeRenderPass = () => ({
  setBindGroup: noop,
  setPipeline: noop,
  draw: noop,
  end: noop,
});
const makeComputePass = () => ({
  setPipeline: noop,
  setBindGroup: noop,
  dispatchWorkgroups: noop,
  end: noop,
});
const webgpuDevice = {
  lost: deviceLost,
  queue: { writeBuffer: noop, submit: noop },
  createBuffer: () => ({ destroy: noop }),
  createShaderModule: () => ({}),
  createRenderPipeline: () => ({
    getBindGroupLayout: () => ({}),
  }),
  createComputePipeline: () => ({
    getBindGroupLayout: () => ({}),
  }),
  createBindGroup: () => ({}),
  createCommandEncoder: () => ({
    beginRenderPass: makeRenderPass,
    beginComputePass: makeComputePass,
    finish: () => ({}),
  }),
};
const webgpuContext = {
  configure: noop,
  getCurrentTexture: () => ({ createView: () => ({}) }),
};
const webgpu = {
  requestAdapter: async () => ({ requestDevice: async () => webgpuDevice }),
  getPreferredCanvasFormat: () => "rgba8unorm",
};

function element(kind) {
  const listeners = new Map();
  return {
    kind,
    dataset: {},
    style: {},
    innerHTML: "",
    width: 1200,
    height: 600,
    offsetWidth: 220,
    offsetHeight: 100,
    addEventListener: (name, listener) => {
      const handlers = listeners.get(name) ?? [];
      handlers.push(listener);
      listeners.set(name, handlers);
    },
    dispatchEvent: (name, event) => {
      for (const listener of listeners.get(name) ?? []) listener(event);
    },
    setPointerCapture: noop,
    getBoundingClientRect: () => ({ left: 0, top: 0, width: 1200, height: 600 }),
    getContext: (type) => (type === "2d" ? context2d : type === "webgpu" ? webgpuContext : webgl),
    querySelector: noop,
  };
}

const shell = element("shell");
const gpu = element("gpu");
const overlay = element("overlay");
const tooltip = element("tooltip");
const fallback = element("fallback");
shell.querySelector = (selector) => ({
  ".finkit-webgl-gpu": gpu,
  ".finkit-webgl-overlay": overlay,
  ".finkit-webgl-tooltip": tooltip,
  ".finkit-webgl-fallback": fallback,
}[selector] ?? null);

const windowObject = {
  devicePixelRatio: 1,
  innerWidth: 1600,
  innerHeight: 900,
  addEventListener: noop,
};
const documentObject = {
  querySelector: (selector) => selector === ".finkit-webgl-shell" ? shell : null,
};

const vmContext = createContext({
  atob: globalThis.atob,
  console,
  document: documentObject,
  navigator: webgpuMode ? { gpu: webgpu } : {},
  GPUBufferUsage: { STORAGE: 1, COPY_DST: 2, UNIFORM: 4 },
  window: windowObject,
  Float32Array,
  Uint8Array,
  Math,
  Number,
  String,
  Object,
  Array,
  JSON,
  Promise,
  Error,
});
windowObject.window = windowObject;
windowObject.document = documentObject;

new Script(html.slice(scriptStart + "<script>".length, scriptEnd), { filename: file }).runInContext(vmContext);
await new Promise((resolve) => setImmediate(resolve));

const controller = windowObject.__finkitGpuChart;
assert.ok(controller, "GPU controller must be exported");
assert.equal(shell.dataset.controller, "ready");
const initialRenderer = shell.dataset.renderer;
assert.equal(initialRenderer, webgpuMode ? "webgpu" : "webgl2");

const initial = controller.getState();
assert.ok(initial.rawCount > 0);
assert.ok(initial.activeCount <= initial.rawCount);
assert.ok(initial.bucket >= 1);

const ringCapacity = 8;
assert.equal(controller.setRingBuffer(ringCapacity), ringCapacity);
let state = controller.getState();
assert.equal(state.ringEnabled, true);
assert.equal(state.ringCapacity, ringCapacity);
assert.equal(state.rawCount, ringCapacity);

assert.equal(controller.updateBar(0, [101, 103, 100, 102, 900]), true);
for (let index = 0; index < 3; index += 1) {
  assert.equal(controller.appendBar(`runtime-${index}`, [102 + index, 104 + index, 101 + index, 103 + index, 1000 + index]), true);
}
state = controller.getState();
assert.equal(state.rawCount, ringCapacity);
assert.equal(state.ringHead, 3);

assert.ok(controller.setRingBuffer(0) >= ringCapacity);
state = controller.getState();
assert.equal(state.ringEnabled, false);
assert.equal(state.rawCount, ringCapacity);
assert.ok(controller.reserve(32) >= state.rawCount + 32);

const viewport = controller.setViewport(0, Math.min(6, state.rawCount));
assert.equal(viewport.start, 0);
assert.ok(viewport.end > viewport.start);

gpu.dispatchEvent("pointermove", { clientX: 150, clientY: 120 });
assert.equal(tooltip.style.display, "block");
assert.match(tooltip.innerHTML, /GPU 数据窗口/);
if (html.includes("var indicatorRows=[{")) {
  assert.match(tooltip.innerHTML, /指标|MA5|MA20/);
}
if (html.includes("var hitRegions=[{")) {
  assert.match(tooltip.innerHTML, /Chan|突破候选|K线/);
}

if (webgpuMode) {
  resolveDeviceLost({ reason: "destroyed" });
  await new Promise((resolve) => setImmediate(resolve));
  const recovered = controller.getState();
  assert.equal(recovered.gpuLost, true);
  assert.equal(recovered.gpuRecovery, "webgl2");
  assert.equal(recovered.renderer, "webgl2");
}

console.log(JSON.stringify({
  file,
  renderer: shell.dataset.renderer,
  initialRenderer,
  initial,
  final: controller.getState(),
  assertions: ["ring", "append", "update", "reserve", "viewport", "tooltip", "indicator-hit", ...(webgpuMode ? ["webgpu", "device-lost-recovery"] : [])],
}));
