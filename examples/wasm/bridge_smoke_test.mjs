/**
 * Node smoke test for API v22 bridge BU/BD steady solve.
 * Usage: node examples/wasm/bridge_smoke_test.mjs
 *
 * Requires pkg-node from ./build_wasm.sh
 */

import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const require = createRequire(import.meta.url);
const __dirname = dirname(fileURLToPath(import.meta.url));
const fixturePath = join(__dirname, 'steady_bridge_bu_bd_v22.json');

import { wasmResult } from './wasm_result.js';

const {
  getEngineVersion,
  getWasmApiMetadata,
  validateSteadyInputs,
  solveSteady,
} = require('../../pkg-node/stream1d.js');

const meta = getWasmApiMetadata();
if (meta.api_version < 22) {
  throw new Error(`Expected api_version >= 22, got ${meta.api_version}`);
}

const inputs = JSON.parse(readFileSync(fixturePath, 'utf8'));
validateSteadyInputs(inputs);
const result = wasmResult(solveSteady(inputs));

if (result.wsel.length !== inputs.cross_sections.length) {
  throw new Error(`wsel length ${result.wsel.length} != cross_sections ${inputs.cross_sections.length}`);
}
if (result.wsel[2] !== 3.0) {
  throw new Error(`downstream WSEL expected 3.0, got ${result.wsel[2]}`);
}
if (!(result.wsel[1] > 3.0)) {
  throw new Error(`bridge should backwater upstream approach, got wsel[1]=${result.wsel[1]}`);
}

console.log('Bridge BU/BD smoke test OK');
console.log('  engine:', getEngineVersion());
console.log('  api_version:', meta.api_version);
console.log('  wsel @ stations 200/100/0:', result.wsel.map((w) => w.toFixed(4)).join(', '));
