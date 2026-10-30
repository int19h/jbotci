const test = require('node:test');
const assert = require('node:assert/strict');
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const zlib = require('node:zlib');
const { once } = require('node:events');
const { readback, credentials } = require('../src/r2_readback.cjs');

const original = Buffer.from('validated original bytes');
const encoded = zlib.brotliCompressSync(original);
const key = 'packs/space ?# Unicode-é.f16';
let root;
let server;
let apiBase;
let mode;
let gets;

test.before(async () => {
    const target = process.env.CARGO_TARGET_DIR;
    assert.ok(target, 'CARGO_TARGET_DIR must name the test build lane');
    root = fs.mkdtempSync(path.join(target, 'r2-http-stub-'));
    server = http.createServer((request, response) => {
        assert.equal(request.headers.authorization, 'Bearer stub-token');
        const url = new URL(request.url, 'http://localhost');
        if (url.pathname.endsWith('/objects')) {
            assert.equal(url.searchParams.get('prefix'), key);
            response.setHeader('Content-Type', 'application/json');
            response.end(JSON.stringify({ success: true, result: [{
                key, size: encoded.length, etag: 'stub-etag', http_metadata: { contentEncoding: 'br' },
            }] }));
        } else {
            gets++;
            assert.equal(decodeURIComponent(url.pathname.split('/objects/')[1]), key);
            assert.equal(request.headers['accept-encoding'], 'br');
            response.setHeader('ETag', mode === 'changed' ? '"other-etag"' : mode === 'decoded' ? 'W/"stub-etag"' : '"stub-etag"');
            if (mode === 'encoded' || mode === 'bad-encoded') response.setHeader('Content-Encoding', 'br');
            const bytes = mode === 'encoded' || mode === 'bad-encoded' ? Buffer.from(encoded) : zlib.brotliDecompressSync(encoded);
            if (mode === 'bad-encoded' || mode === 'bad-decoded') bytes[0] ^= 1;
            response.end(bytes);
        }
    });
    for (let port = 18858; port <= 18899; port++) {
        try {
            server.listen(port, '127.0.0.1');
            await once(server, 'listening');
            apiBase = `http://127.0.0.1:${port}`;
            return;
        } catch (error) {
            if (error.code !== 'EADDRINUSE') throw error;
        }
    }
    throw new Error('No free jbotci test port');
});

test.after(async () => {
    if (server?.listening) await new Promise(resolve => server.close(resolve));
    if (root) fs.rmSync(root, { recursive: true, force: true });
});

for (const behavior of ['encoded', 'decoded', 'bad-encoded', 'bad-decoded']) {
    test(`raw HTTP preserves the ${behavior} server representation`, async () => {
        mode = behavior;
        gets = 0;
        const downloaded = path.join(root, behavior);
        const metadata = `${downloaded}.json`;
        await readback({ account: 'stub-account', bucket: 'stub-bucket', key, encoding: 'br', downloaded, metadata,
            headers: { Authorization: 'Bearer stub-token' }, apiBase });
        assert.equal(gets, 1);
        const expected = behavior.includes('encoded') ? encoded : original;
        const bytes = fs.readFileSync(downloaded);
        if (behavior.startsWith('bad-')) assert.notDeepEqual(bytes, expected);
        else assert.deepEqual(bytes, expected);
        assert.deepEqual(JSON.parse(fs.readFileSync(metadata)), {
            stored_size: encoded.length, stored_encoding: 'br',
            body_encoding: behavior.includes('encoded') ? 'br' : null,
        });
    });
}

test('metadata and body must name the same stored object', async () => {
    mode = 'changed';
    await assert.rejects(readback({ account: 'stub-account', bucket: 'stub-bucket', key, encoding: 'br',
        downloaded: path.join(root, 'changed'), metadata: path.join(root, 'changed.json'),
        headers: { Authorization: 'Bearer stub-token' }, apiBase }), /changed between metadata and body/);
});

test('Wrangler credentials stay in memory and support each declared type', () => {
    const script = path.join(root, 'auth.cjs');
    for (const [value, headers] of [
        [{ type: 'api_token', token: 'stub-token' }, { Authorization: 'Bearer stub-token' }],
        [{ type: 'oauth', token: 'stub-oauth' }, { Authorization: 'Bearer stub-oauth' }],
        [{ type: 'api_key', key: 'stub-key', email: 'stub-email' }, { 'X-Auth-Key': 'stub-key', 'X-Auth-Email': 'stub-email' }],
    ]) {
        fs.writeFileSync(script, `if (process.argv.slice(2).join(' ') !== 'auth token --json') process.exit(1); console.log(${JSON.stringify(JSON.stringify(value))});`);
        assert.deepEqual(credentials(script), headers);
    }
    fs.writeFileSync(script, 'console.log("not credential JSON");');
    assert.throws(() => credentials(script), /valid credential JSON/);
});
