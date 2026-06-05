import init, { solveSteady } from './pkg/streams1d.js';

// Unpacks a flat Float64Array buffer back into structured cross-sections
function unpackGeometry(flatArray) {
    let offset = 0;
    const numSections = flatArray[offset++];
    const crossSections = [];
    
    for (let i = 0; i < numSections; i++) {
        const station = flatArray[offset++];
        const numPoints = flatArray[offset++];
        const numNPoints = flatArray[offset++];
        
        const x = [];
        const y = [];
        for (let j = 0; j < numPoints; j++) {
            x.push(flatArray[offset++]);
            y.push(flatArray[offset++]);
        }
        
        const n_stations = [];
        const n_values = [];
        for (let j = 0; j < numNPoints; j++) {
            n_stations.push(flatArray[offset++]);
            n_values.push(flatArray[offset++]);
        }
        
        crossSections.push({
            station,
            x,
            y,
            n_stations,
            n_values,
            unit_system: "Metric"
        });
    }
    return crossSections;
}

self.onmessage = async (e) => {
    const { type, payload } = e.data;

    if (type === 'INIT') {
        try {
            await init();
            self.postMessage({ type: 'INIT_COMPLETE' });
        } catch (err) {
            self.postMessage({ type: 'INIT_FAILED', error: err.message });
        }
    }

    else if (type === 'RUN_BENCHMARK') {
        const { mode, iterations, flow_rate, downstream_wsel, num_slices, regime } = payload;
        
        let crossSections = null;
        let flatGeometry = null;

        if (mode === 'transferable') {
            // Unpack from flat ArrayBuffer (zero-copy geometry ingestion)
            flatGeometry = new Float64Array(payload.geometryBuffer);
            crossSections = unpackGeometry(flatGeometry);
        } else {
            // Standard JSON copy
            crossSections = payload.crossSections;
        }

        const inputs = {
            cross_sections: crossSections,
            flow_rate,
            num_slices,
            regime,
            downstream_wsel,
            coeff_contraction: 0.1,
            coeff_expansion: 0.3
        };

        const startTime = performance.now();
        let lastUpdateTime = startTime;
        const throttleInterval = 16.67; // ~60 FPS update gate

        let results = null;

        for (let i = 0; i < iterations; i++) {
            // Run steady-state solver
            results = solveSteady(inputs);

            const currentTime = performance.now();
            const elapsedSinceLastUpdate = currentTime - lastUpdateTime;

            // Visual Throttling Check: send results back to main thread at most once every 16.6ms
            if (elapsedSinceLastUpdate >= throttleInterval || i === iterations - 1) {
                if (mode === 'transferable') {
                    // Zero-copy transfer of output WSEL and Velocity arrays
                    const wselBuffer = new Float64Array(results.wsel).buffer;
                    const velBuffer = new Float64Array(results.velocity).buffer;

                    self.postMessage({
                        type: 'BENCHMARK_PROGRESS',
                        mode,
                        iteration: i + 1,
                        wsel: wselBuffer,
                        velocity: velBuffer,
                        elapsed: currentTime - startTime
                    }, [wselBuffer, velBuffer]); // Transferred ownership
                } else {
                    // Standard copied arrays
                    self.postMessage({
                        type: 'BENCHMARK_PROGRESS',
                        mode,
                        iteration: i + 1,
                        wsel: results.wsel,
                        velocity: results.velocity,
                        elapsed: currentTime - startTime
                    });
                }
                lastUpdateTime = currentTime;
            }
        }

        const endTime = performance.now();
        const totalDuration = endTime - startTime;
        const avgDuration = totalDuration / iterations;
        const throughput = iterations / (totalDuration / 1000.0);

        self.postMessage({
            type: 'BENCHMARK_COMPLETE',
            mode,
            totalDuration,
            avgDuration,
            throughput
        });
    }
};
