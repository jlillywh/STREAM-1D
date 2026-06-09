/**
 * Node smoke test for pkg-node after ./build_wasm.sh
 * Usage: node examples/wasm/node_smoke_test.mjs
 *
 * Note: pkg-node (wasm-pack --target nodejs) auto-initializes WASM — no init() call.
 */

import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const require = createRequire(import.meta.url);
const __dirname = dirname(fileURLToPath(import.meta.url));
const fixturePath = join(__dirname, '../../tests/fixtures/wasm_steady_culvert_tier1.json');

const {
  getEngineVersion,
  getWasmApiMetadata,
  validateSteadyInputs,
  solveSteady,
} = require('../../pkg-node/stream1d.js');

const meta = getWasmApiMetadata();
if (meta.api_version < 2) {
  throw new Error(`Expected api_version >= 2, got ${meta.api_version}`);
}

const inputs = JSON.parse(readFileSync(fixturePath, 'utf8'));
validateSteadyInputs(inputs);
const result = solveSteady(inputs);

if (!result.culvert_control_types?.length) {
  throw new Error('culvert_control_types missing from WASM result');
}

console.log('WASM smoke test OK');
console.log('  engine:', getEngineVersion());
console.log('  api_version:', meta.api_version);
console.log('  culvert_control_types:', result.culvert_control_types);
