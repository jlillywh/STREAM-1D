/**
 * Reference Web Worker for STREAM-1D steady solves.
 *
 * Usage in the web app:
 *   new Worker(new URL('./streams1d.worker.js', import.meta.url), { type: 'module' })
 *
 * Copy this file, adjust the import path to your deployed `pkg/streams1d.js`.
 */

import init, {
  getEngineVersion,
  getWasmApiMetadata,
  validateSteadyInputs,
  solveSteady,
} from '../../pkg/streams1d.js';

let ready = false;

async function ensureInit() {
  if (ready) return;
  await init();
  ready = true;
  const metadata = getWasmApiMetadata();
  self.postMessage({
    type: 'ready',
    engineVersion: getEngineVersion(),
    apiVersion: metadata.api_version,
    metadata,
  });
}

self.onmessage = async (event) => {
  try {
    await ensureInit();
    const { type, inputs } = event.data ?? {};

    if (type === 'solveSteady') {
      validateSteadyInputs(inputs);
      const result = solveSteady(inputs);
      self.postMessage({ type: 'steadyResult', result });
      return;
    }

    if (type === 'getMetadata') {
      self.postMessage({
        type: 'metadata',
        engineVersion: getEngineVersion(),
        metadata: getWasmApiMetadata(),
      });
      return;
    }

    self.postMessage({ type: 'error', message: `Unknown message type: ${type}` });
  } catch (err) {
    self.postMessage({
      type: 'error',
      message: err instanceof Error ? err.message : String(err),
    });
  }
};
