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
            if (mode === 'encoded' || mode === 'bad-encoded' || mode === 'alternate-encoded' || mode === 'changed') response.setHeader('Content-Encoding', 'br');
            const bytes = mode === 'alternate-encoded' ? Buffer.from(alternate) : mode === 'encoded' || mode === 'bad-encoded' || mode === 'changed' ? Buffer.from(encoded) : zlib.brotliDecompressSync(alternate);
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
            stored_size: encoded.length, stored_encoding: 'br', body_encoding: 'br',
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
