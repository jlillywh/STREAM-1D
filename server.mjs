import http from 'http';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const PORT = 8080;

const MIME_TYPES = {
    '.html': 'text/html',
    '.css': 'text/css',
    '.js': 'text/javascript',
    '.mjs': 'text/javascript',
    '.json': 'application/json',
    '.wasm': 'application/wasm',
    '.png': 'image/png',
    '.jpg': 'image/jpeg',
    '.gif': 'image/gif',
    '.svg': 'image/svg+xml',
};

const server = http.createServer((req, res) => {
    // Basic routing
    let filePath = req.url === '/' ? './index.html' : '.' + req.url;
    
    // Remove query params if any
    filePath = filePath.split('?')[0];
    
    const ext = path.extname(filePath).toLowerCase();
    const resolvedPath = path.resolve(__dirname, filePath);
    
    // Safety check: ensure paths are inside project root to prevent path traversal
    if (!resolvedPath.startsWith(__dirname)) {
        res.statusCode = 403;
        res.end('Forbidden');
        return;
    }

    fs.readFile(resolvedPath, (err, content) => {
        if (err) {
            if (err.code === 'ENOENT') {
                res.statusCode = 404;
                res.end('File not found');
            } else {
                res.statusCode = 500;
                res.end(`Internal server error: ${err.code}`);
            }
            return;
        }

        const mime = MIME_TYPES[ext] || 'application/octet-stream';
        
        // Add headers for WASM security and CORS
        res.writeHead(200, {
            'Content-Type': mime,
            'Cross-Origin-Opener-Policy': 'same-origin',
            'Cross-Origin-Embedder-Policy': 'require-corp',
            'Access-Control-Allow-Origin': '*',
            'Cache-Control': 'no-cache, no-store, must-revalidate',
        });
        
        res.end(content);
    });
});

server.listen(PORT, () => {
    console.log(`STREAMS-1D Dev Server running at http://localhost:${PORT}`);
});
