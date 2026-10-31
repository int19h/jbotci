const test = require('node:test');
const assert = require('node:assert/strict');
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const zlib = require('node:zlib');
const { once } = require('node:events');
const { spawnSync } = require('node:child_process');
const { createHash } = require('node:crypto');
const { FAKE_TOKEN, FORCED, REMOVED } = require('./fixtures/wrangler-4.149.0-logging.cjs');
const { readback, credentials } = require('../src/r2_readback.cjs');

const original = Buffer.from('validated original bytes');
const encoded = Buffer.from('830b8076616c696461746564206f726967696e616c20627974657303', 'hex');
const alternate = Buffer.from('850b8076616c696461746564206f726967696e616c20627974657303', 'hex');
const key = 'packs/space ?# Unicode-é.f16';
let root;
let server;
let apiBase;
let mode;
let gets;
let requestedEncoding = 'br';

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
                key, size: requestedEncoding === 'br' ? encoded.length : original.length,
                etag: createHash('md5').update(requestedEncoding === 'br' ? encoded : original).digest('hex'),
                last_modified: '2026-10-31', custom_metadata: {}, storage_class: 'Standard',
                http_metadata: { ...(requestedEncoding === 'br' ? { contentEncoding: 'br' } : {}),
                    contentType: 'application/octet-stream', cacheControl: 'immutable' },
            }] }));
        } else {
            gets++;
            assert.equal(decodeURIComponent(url.pathname.split('/objects/')[1]), key);
            assert.equal(request.headers['accept-encoding'], requestedEncoding);
            const digest = createHash('md5').update(requestedEncoding === 'br' ? encoded : original).digest('hex');
            let etag = requestedEncoding === 'br' ? `"${digest}"` : `W/"${digest}"`;
            if (mode === 'changed' || mode === 'identity-changed') etag = `W/"${'0'.repeat(32)}"`;
            if (mode === 'identity-double-weak') etag = `W/W/"${digest}"`;
            if (mode === 'identity-multipart') etag = `W/"${digest}-2"`;
            if (mode !== 'identity-missing-etag') response.setHeader('ETag', etag);
            if (mode === 'encoded' || mode === 'bad-encoded' || mode === 'alternate-encoded' || mode === 'changed') response.setHeader('Content-Encoding', 'br');
            const bytes = mode === 'alternate-encoded' ? Buffer.from(alternate) : mode === 'encoded' || mode === 'bad-encoded' || mode === 'changed' ? Buffer.from(encoded) : zlib.brotliDecompressSync(alternate);
            if (mode === 'bad-encoded' || mode === 'bad-decoded') bytes[0] ^= 1;
            if (requestedEncoding === 'identity') {
                // Match the measured identity response: no encoding, no length, and a chunked body.
                assert.equal(response.getHeader('Content-Encoding'), undefined);
                response.write(original);
                assert.equal(response.getHeader('Content-Length'), undefined);
                response.end();
            } else response.end(bytes);
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

test('equal-length Brotli streams have equal content and different digests', () => {
    assert.equal(encoded.length, alternate.length);
    assert.deepEqual(zlib.brotliDecompressSync(encoded), original);
    assert.deepEqual(zlib.brotliDecompressSync(alternate), original);
    assert.notEqual(createHash('sha256').update(encoded).digest('hex'), createHash('sha256').update(alternate).digest('hex'));
});

for (const behavior of ['encoded', 'bad-encoded', 'alternate-encoded']) {
    test(`raw HTTP preserves the ${behavior} server representation`, async () => {
        mode = behavior;
        gets = 0;
        const downloaded = path.join(root, behavior);
        const metadata = `${downloaded}.json`;
        await readback({ account: 'stub-account', bucket: 'stub-bucket', key, encoding: 'br', downloaded, metadata,
            headers: { Authorization: 'Bearer stub-token' }, apiBase });
        assert.equal(gets, 1);
        const bytes = fs.readFileSync(downloaded);
        if (behavior === 'encoded') assert.deepEqual(bytes, encoded);
        else assert.notDeepEqual(bytes, encoded);
        assert.deepEqual(JSON.parse(fs.readFileSync(metadata)), {
            stored: { key, size: encoded.length, etag: createHash('md5').update(encoded).digest('hex'),
                http_metadata: { contentType: 'application/octet-stream', cacheControl: 'immutable', contentEncoding: 'br' } },
            body_encoding: 'br',
        });
    });
}

for (const behavior of ['decoded', 'bad-decoded']) {
    test(`raw HTTP refuses the ${behavior} server representation`, async () => {
        mode = behavior;
        const downloaded = path.join(root, behavior);
        const metadata = `${downloaded}.json`;
        await assert.rejects(readback({ account: 'stub-account', bucket: 'stub-bucket', key, encoding: 'br', downloaded, metadata,
            headers: { Authorization: 'Bearer stub-token' }, apiBase }), /decoded Brotli body. Exact stored bytes cannot be verified/);
        assert.equal(fs.existsSync(downloaded), false);
        assert.equal(fs.existsSync(metadata), false);
    });
}

test('metadata and body must name the same stored object', async () => {
    mode = 'changed';
    await assert.rejects(readback({ account: 'stub-account', bucket: 'stub-bucket', key, encoding: 'br',
        downloaded: path.join(root, 'changed'), metadata: path.join(root, 'changed.json'),
        headers: { Authorization: 'Bearer stub-token' }, apiBase }), /changed between metadata and body/);
});

test('identity read-back accepts the measured weak ETag and chunked bytes', async () => {
    requestedEncoding = 'identity';
    mode = 'identity';
    try {
        const downloaded = path.join(root, 'identity');
        const metadata = `${downloaded}.json`;
        await readback({ account: 'stub-account', bucket: 'stub-bucket', key, encoding: 'identity', downloaded, metadata,
            headers: { Authorization: 'Bearer stub-token' }, apiBase });
        assert.deepEqual(fs.readFileSync(downloaded), original);
        const receipt = JSON.parse(fs.readFileSync(metadata));
        assert.equal(receipt.body_encoding, null);
        assert.equal(receipt.stored.size, original.length);
        assert.equal(receipt.stored.etag, createHash('md5').update(original).digest('hex'));
        assert.equal(Object.hasOwn(receipt.stored.http_metadata, 'contentEncoding'), false);
    } finally { requestedEncoding = 'br'; }
});

for (const behavior of ['identity-changed', 'identity-double-weak', 'identity-missing-etag', 'identity-multipart']) {
    test(`identity read-back refuses ${behavior}`, async () => {
        requestedEncoding = 'identity';
        mode = behavior;
        const downloaded = path.join(root, behavior);
        try {
            await assert.rejects(readback({ account: 'stub-account', bucket: 'stub-bucket', key, encoding: 'identity',
                downloaded, metadata: `${downloaded}.json`, headers: { Authorization: 'Bearer stub-token' }, apiBase }),
                /object changed between metadata and body reads/);
            assert.equal(fs.existsSync(downloaded), false);
        } finally { requestedEncoding = 'br'; }
    });
}

test('Wrangler credentials support each declared type', () => {
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

function filesUnder(directory) {
    return fs.readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
        const name = path.join(directory, entry.name);
        return entry.isDirectory() ? filesUnder(name) : [name];
    });
}

test('pinned logging policy exposes the fake token without the environment guard', () => {
    const fixture = path.join(__dirname, 'fixtures/wrangler-4.149.0-logging.cjs');
    for (const setting of [undefined, 'true']) {
        const directory = fs.mkdtempSync(path.join(root, 'unsafe-policy-'));
        const env = { ...process.env, TEST_OUTPUT_DIR: directory, WRANGLER_LOG: 'log', NODE_V8_COVERAGE: '' };
        delete env.WRANGLER_WRITE_LOGS;
        for (const name of REMOVED) delete env[name];
        if (setting !== undefined) env.WRANGLER_WRITE_LOGS = setting;
        const result = spawnSync(process.execPath, [fixture, 'auth', 'token', '--json'], { env, encoding: 'utf8' });
        assert.equal(result.status, 0);
        assert.ok(result.stdout.includes(FAKE_TOKEN));
        assert.ok(filesUnder(directory).some(file => fs.readFileSync(file).includes(FAKE_TOKEN)));
        fs.rmSync(directory, { recursive: true });
    }
});

test('credential child overrides hostile logging controls without token output or files', () => {
    const fixture = path.join(__dirname, 'fixtures/wrangler-4.149.0-logging.cjs');
    const helper = path.join(__dirname, '../src/r2_readback.cjs');
    for (const setting of [undefined, 'true']) {
        const directory = fs.mkdtempSync(path.join(root, 'guarded-policy-'));
        const harness = path.join(root, `credential-harness-${setting}.cjs`);
        fs.writeFileSync(harness, `
const assert = require('node:assert/strict');
const { credentials } = require(${JSON.stringify(helper)});
const { FAKE_TOKEN, REMOVED } = require(${JSON.stringify(fixture)});
for (const name of REMOVED) process.env[name] = 'hostile logging setting';
process.env.WRANGLER_LOG = 'debug';
process.env.WRANGLER_LOG_SANITIZE = 'false';
process.env.WRANGLER_SEND_METRICS = 'true';
process.env.CI = 'false';
process.env.NODE_V8_COVERAGE = ${JSON.stringify(path.join(directory, 'hostile-coverage'))};
${setting === undefined ? 'delete process.env.WRANGLER_WRITE_LOGS;' : "process.env.WRANGLER_WRITE_LOGS = 'true';"}
assert.deepEqual(credentials(${JSON.stringify(fixture)}), { Authorization: 'Bearer ' + FAKE_TOKEN });
console.log('credential stub completed');
`);
        const env = { ...process.env, TEST_OUTPUT_DIR: directory, NODE_V8_COVERAGE: '' };
        for (const name of REMOVED) delete env[name];
        const result = spawnSync(process.execPath, [harness], { env, encoding: 'utf8' });
        assert.equal(result.status, 0, result.stderr);
        assert.equal(result.stdout, 'credential stub completed\n');
        assert.equal(result.stderr, '');
        assert.equal((result.stdout + result.stderr).includes(FAKE_TOKEN), false);
        const observed = JSON.parse(fs.readFileSync(path.join(directory, 'environment.json')));
        for (const [name, value] of Object.entries(FORCED)) assert.equal(observed[name], value);
        for (const name of REMOVED) assert.equal(observed[name], null);
        assert.deepEqual(filesUnder(directory).map(file => path.relative(directory, file)), ['environment.json']);
        for (const file of filesUnder(directory)) assert.equal(fs.readFileSync(file).includes(FAKE_TOKEN), false, file);
    }
});

// Give each stub a distinct origin so HTTP keep-alive cannot reuse a closed server's connection.
let nextStubPort = 18800;
async function listenStub(server) {
    while (nextStubPort <= 18899) {
        const port = nextStubPort++;
        try {
            server.listen(port, '127.0.0.1');
            await once(server, 'listening');
            return;
        } catch (error) {
            if (error.code !== 'EADDRINUSE') throw error;
        }
    }
    throw new Error('No free jbotci test port');
}

// These listeners are local stubs. No Cloudflare endpoint is contacted.
test('prefix listing returns complete proof fields across pages without body reads', async () => {
    const { listMetadata } = require('../src/r2_readback.cjs');
    const requests = [];
    const stub = http.createServer((request, response) => {
        const url = new URL(request.url, 'http://localhost');
        requests.push(url);
        assert.equal(request.headers.authorization, 'Bearer stub-token');
        assert.equal(request.headers['accept-encoding'], 'identity');
        assert.equal(url.searchParams.get('prefix'), 'packs/');
        assert.equal(url.searchParams.get('per_page'), '1000');
        assert.ok(url.pathname.endsWith('/objects'));
        const second = url.searchParams.get('cursor') === 'page-2';
        response.setHeader('Content-Type', 'application/json');
        response.end(JSON.stringify({ success: true, result: [{ key: second ? 'packs/b' : 'packs/a', size: 28,
            etag: 'd41d8cd98f00b204e9800998ecf8427e', custom_metadata: { ignored: 'not-a-proof' },
            http_metadata: { contentEncoding: 'br', contentType: 'application/octet-stream', cacheControl: 'immutable' } }],
            result_info: second ? { is_truncated: false } : { is_truncated: true, cursor: 'page-2' } }));
    });
    await listenStub(stub);
    try {
        const result = await listMetadata({ account: 'stub-account', bucket: 'stub-bucket', prefix: 'packs/',
            headers: { Authorization: 'Bearer stub-token' }, apiBase: `http://127.0.0.1:${stub.address().port}` });
        assert.equal(requests.length, 2);
        assert.equal(result.length, 2);
        assert.deepEqual(result.map(object => object.key), ['packs/a', 'packs/b']);
        for (const object of result) {
            assert.equal(object.size, 28);
            assert.equal(object.etag, 'd41d8cd98f00b204e9800998ecf8427e');
            assert.deepEqual(object.http_metadata, { contentType: 'application/octet-stream', cacheControl: 'immutable', contentEncoding: 'br' });
            assert.equal(object.custom_metadata, undefined);
        }
    } finally { await new Promise(resolve => stub.close(resolve)); }
});

test('listing receipt distinguishes absent encoding from an absent metadata object', async () => {
    const { listMetadata } = require('../src/r2_readback.cjs');
    const stub = http.createServer((request, response) => {
        response.setHeader('Content-Type', 'application/json');
        response.end(JSON.stringify({ success: true, result: [
            { key: 'packs/unencoded', http_metadata: { contentType: 'application/octet-stream', cacheControl: 'immutable' } },
            { key: 'packs/unknown' },
            { key: 'packs/malformed', http_metadata: { contentEncoding: null } },
        ], result_info: { is_truncated: false } }));
    });
    await listenStub(stub);
    try {
        const listed = await listMetadata({ account: 'stub-account', bucket: 'stub-bucket', prefix: 'packs/',
            headers: { Authorization: 'Bearer stub-token' }, apiBase: `http://127.0.0.1:${stub.address().port}` });
        // Use the same JSON boundary as metadataMain and the Rust reader.
        const receipt = JSON.parse(JSON.stringify(listed));
        assert.deepEqual(receipt[0].http_metadata, { contentType: 'application/octet-stream', cacheControl: 'immutable' });
        assert.equal(Object.hasOwn(receipt[0].http_metadata, 'contentEncoding'), false);
        assert.equal(Object.hasOwn(receipt[1], 'http_metadata'), false);
        assert.equal(Object.hasOwn(receipt[2].http_metadata, 'contentEncoding'), true);
        assert.equal(receipt[2].http_metadata.contentEncoding, null);
    } finally { await new Promise(resolve => stub.close(resolve)); }
});

for (const bad of ['http-error', 'bad-json', 'api-error', 'reported-error', 'truncated-no-cursor', 'duplicate-key', 'repeated-cursor']) {
    test(`prefix listing refuses ${bad} and exposes no partial receipt`, async () => {
        const { listMetadata } = require('../src/r2_readback.cjs');
        let calls = 0;
        const stub = http.createServer((request, response) => {
            calls++;
            if (bad === 'http-error') { response.writeHead(503); response.end('untrusted response body'); return; }
            if (bad === 'bad-json') { response.end('not JSON'); return; }
            response.setHeader('Content-Type', 'application/json');
            response.end(JSON.stringify({ success: bad !== 'api-error', errors: bad === 'reported-error' ? [{ code: 1000, message: 'stub' }] : [], result: [{ key: bad === 'repeated-cursor' ? `packs/${calls}` : 'packs/a' }],
                result_info: bad === 'truncated-no-cursor' ? { is_truncated: true } : { cursor: 'next', is_truncated: true } }));
        });
        await listenStub(stub);
        try {
            await assert.rejects(listMetadata({ account: 'stub-account', bucket: 'stub-bucket', prefix: 'packs/',
                headers: {}, apiBase: `http://127.0.0.1:${stub.address().port}` }));
            assert.ok(calls <= 2);
        } finally { await new Promise(resolve => stub.close(resolve)); }
    });
}
