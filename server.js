const express = require('express');
const crypto = require('crypto');
const fs = require('fs').promises;
const { execFile } = require('child_process');
const { promisify } = require('util');
const path = require('path');
const os = require('os');
const rateLimit = require('express-rate-limit');

const execFilePromise = promisify(execFile);

// Configuration
const CONFIG = {
    PORT: process.env.PORT || 8080,
    MAX_FILE_SIZE: parseInt(process.env.MAX_FILE_SIZE) || 5 * 1024 * 1024, // 5MB default
    RUST_BINARY_PATH: process.env.RUST_BINARY_PATH || '/usr/local/bin/luau-lifter', // Updated binary name
    RATE_LIMIT_WINDOW: parseInt(process.env.RATE_LIMIT_WINDOW) || 15 * 60 * 1000, // 15 minutes
    RATE_LIMIT_MAX_REQUESTS: parseInt(process.env.RATE_LIMIT_MAX_REQUESTS) || 100,
    BINARY_TIMEOUT: parseInt(process.env.BINARY_TIMEOUT) || 30000, // 30 seconds
    ALPHANUMERIC: 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'
};

const app = express();

// Security: Rate limiting
const limiter = rateLimit({
    windowMs: CONFIG.RATE_LIMIT_WINDOW,
    max: CONFIG.RATE_LIMIT_MAX_REQUESTS,
    message: { error: 'Too many requests, please try again later.' },
    standardHeaders: true,
    legacyHeaders: false
});

app.use(limiter);

// Security: Remove powered-by header
app.disable('x-powered-by');

// Middleware to handle raw binary data with size limit
app.use(express.raw({ 
    type: '*/*',
    limit: CONFIG.MAX_FILE_SIZE
}));

// Health check endpoint (required by Render)
app.get('/health', (req, res) => {
    res.status(200).json({ status: 'healthy', timestamp: new Date().toISOString() });
});

// Root endpoint
app.get('/', (req, res) => {
    res.json({ 
        message: 'Luau Decompiler API',
        version: '1.0.0',
        endpoints: {
            decompile: 'POST /decompile',
            health: 'GET /health'
        }
    });
});

// Decompile endpoint
app.post('/decompile', async (req, res) => {
    let tempFilePath = null;

    try {
        // Validate request body
        if (!req.body || req.body.length === 0) {
            return res.status(400).json({ error: 'No bytecode provided' });
        }

        // Check file size
        if (req.body.length > CONFIG.MAX_FILE_SIZE) {
            return res.status(413).json({ 
                error: `Payload too large. Maximum size is ${CONFIG.MAX_FILE_SIZE} bytes` 
            });
        }

        // Generate secure random filename
        const randomBytes = crypto.randomBytes(16);
        const filename = `temp_${randomBytes.toString('hex')}.bin`;
        const tempDir = os.tmpdir();
        tempFilePath = path.join(tempDir, filename);

        // Decode base64 bytecode if it's a string, otherwise use as buffer
        let bytecode;
        if (req.headers['content-type'] === 'text/plain') {
            try {
                bytecode = Buffer.from(req.body.toString(), 'base64');
            } catch (error) {
                console.warn('Base64 decode failed, treating as raw binary');
                bytecode = req.body;
            }
        } else {
            bytecode = req.body;
        }

        // Validate bytecode (simple header check for Luau)
        if (bytecode.length < 4) {
            return res.status(400).json({ error: 'Invalid bytecode: too short' });
        }

        // Write file to temp directory
        await fs.writeFile(tempFilePath, bytecode, { mode: 0o600 }); // Restrictive permissions

        // Execute decompiler with timeout
        const { stdout, stderr } = await execFilePromise(
            CONFIG.RUST_BINARY_PATH,
            [tempFilePath, '-e'],
            { 
                timeout: CONFIG.BINARY_TIMEOUT,
                maxBuffer: 10 * 1024 * 1024, // 10MB max output
                env: {
                    ...process.env,
                    RUST_LOG: process.env.RUST_LOG || 'info'
                }
            }
        );

        // Log stderr if any (non-fatal)
        if (stderr) {
            console.warn(`Decompiler warning: ${stderr}`);
        }

        // Clean up temp file immediately
        await fs.unlink(tempFilePath).catch(() => {}); // Ignore cleanup errors
        tempFilePath = null;

        // Validate output
        if (!stdout || stdout.trim().length === 0) {
            console.error('Decompiler returned empty output');
            return res.status(500).json({ error: 'Decompilation failed: empty output' });
        }

        // Return successful result
        res.setHeader('content-type', 'text/plain; charset=utf-8');
        res.send(stdout);

    } catch (error) {
        console.error(`Decompilation error: ${error.message}`);
        
        // Attempt cleanup
        if (tempFilePath) {
            await fs.unlink(tempFilePath).catch(() => {});
        }

        // Handle specific error types
        if (error.code === 'ETIMEDOUT') {
            return res.status(504).json({ error: 'Decompilation timeout exceeded' });
        } else if (error.code === 'ENOENT') {
            return res.status(500).json({ error: 'Decompiler binary not found' });
        } else if (error.signal === 'SIGKILL') {
            return res.status(413).json({ error: 'Bytecode too complex or output too large' });
        }

        res.status(500).json({ error: 'Internal decompilation error' });
    }
});

// Global error handler
app.use((err, req, res, next) => {
    console.error('Unhandled error:', err);
    res.status(500).json({ error: 'Internal server error' });
});

// Graceful shutdown
process.on('SIGTERM', async () => {
    console.info('SIGTERM received, shutting down gracefully');
    process.exit(0);
});

process.on('SIGINT', async () => {
    console.info('SIGINT received, shutting down gracefully');
    process.exit(0);
});

// Start server
app.listen(CONFIG.PORT, '0.0.0.0', () => {
    console.log(`🚀 Luau Decompiler API running on http://0.0.0.0:${CONFIG.PORT}`);
    console.log(`📦 Max file size: ${(CONFIG.MAX_FILE_SIZE / 1024 / 1024).toFixed(2)} MB`);
    console.log(`⚙️  Binary path: ${CONFIG.RUST_BINARY_PATH}`);
});
