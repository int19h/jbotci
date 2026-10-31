const fs = require('node:fs');
const http = require('node:http');
const https = require('node:https');
const { spawnSync } = require('node:child_process');
const { pipeline } = require('node:stream/promises');

const API_BASE = 'https://api.cloudflare.com/client/v4';

function childEnvironment(parent = process.env) {
    const env = { ...parent, WRANGLER_WRITE_LOGS: 'false', WRANGLER_LOG: 'log',
        WRANGLER_LOG_SANITIZE: 'true', WRANGLER_SEND_METRICS: 'false', CI: 'true', NODE_V8_COVERAGE: '' };
    for (const name of ['WRANGLER_LOG_PATH', 'WRANGLER_OUTPUT_FILE_PATH', 'WRANGLER_OUTPUT_FILE_DIRECTORY',
        'DEBUG', 'NODE_DEBUG', 'NODE_DEBUG_NATIVE', 'NODE_OPTIONS',
        'SSLKEYLOGFILE', 'LOG_STREAM', 'LOG_TOKENS']) delete env[name];
    return env;
}

function credentials(script) {
    const result = spawnSync(process.execPath, [script, 'auth', 'token', '--json'], {
        env: childEnvironment(),
        stdio: ['ignore', 'pipe', 'pipe'],
        maxBuffer: 1024 * 1024,
    });
    if (result.error || result.status !== 0) throw new Error('Wrangler could not read the publication credentials');
    let value;
    try { value = JSON.parse(result.stdout.toString('utf8')); }
    catch { throw new Error('Wrangler did not return valid credential JSON'); }
    if ((value.type === 'api_token' || value.type === 'oauth') && typeof value.token === 'string' && value.token.length) {
        return { Authorization: `Bearer ${value.token}` };
    }
    if (value.type === 'api_key' && typeof value.key === 'string' && value.key.length && typeof value.email === 'string' && value.email.length) {
        return { 'X-Auth-Key': value.key, 'X-Auth-Email': value.email };
    }
    throw new Error('Wrangler returned unsupported publication credentials');
}

function request(url, headers) {
    // IncomingMessage preserves content coding. Fetch decodes the body.
    return new Promise((resolve, reject) => {
        const client = url.protocol === 'https:' ? https : http;
        const outgoing = client.get(url, { headers }, response => {
            if (response.statusCode !== 200) {
                response.destroy();
                reject(new Error(`R2 read-back returned HTTP ${response.statusCode}`));
            } else resolve(response);
        });
        outgoing.on('error', reject);
    });
}

async function* metadataPages(base, prefix, headers, perPage) {
    const seen = new Set();
    let cursor;
    do {
        const url = new URL(base);
        url.searchParams.set('prefix', prefix);
        url.searchParams.set('per_page', String(perPage));
        if (cursor) url.searchParams.set('cursor', cursor);
        const response = await request(url, { ...headers, Accept: 'application/json', 'Accept-Encoding': 'identity', 'Cache-Control': 'no-cache' });
        const chunks = [];
        let bytes = 0;
        for await (const chunk of response) {
            bytes += chunk.length;
            if (bytes > 64 * 1024 * 1024) {
                response.destroy();
                throw new Error('R2 metadata page exceeds the size limit');
            }
            chunks.push(chunk);
        }
        let value;
        try { value = JSON.parse(Buffer.concat(chunks).toString('utf8')); }
        catch { throw new Error('R2 did not return valid metadata JSON'); }
        if (value.success !== true || !Array.isArray(value.result)) throw new Error('R2 did not return object metadata');
        if (value.errors !== undefined && (!Array.isArray(value.errors) || value.errors.length)) throw new Error('R2 metadata response contains errors');
        const info = value.result_info;
        if (info !== undefined && (info === null || typeof info !== 'object' || Array.isArray(info))) throw new Error('R2 returned invalid pagination metadata');
        if (info?.is_truncated !== undefined && typeof info.is_truncated !== 'boolean') throw new Error('R2 returned invalid truncation metadata');
        yield value.result;
        cursor = info?.cursor;
        if (cursor !== undefined && typeof cursor !== 'string') throw new Error('R2 returned an invalid metadata cursor');
        if (value.result_info?.is_truncated === true && !cursor) throw new Error('R2 omitted the next metadata cursor');
        if (cursor && seen.has(cursor)) throw new Error('R2 repeated a metadata cursor');
        if (cursor) seen.add(cursor);
        if (info?.is_truncated === false) cursor = undefined;
    } while (cursor);
}

async function objectMetadata(base, key, headers) {
    for await (const objects of metadataPages(base, key, headers, 1)) {
        const object = objects.find(object => object.key === key);
        if (object) {
            if (!Number.isSafeInteger(object.size) || object.size < 0 || typeof object.etag !== 'string' || !object.etag.length) {
                throw new Error('R2 returned incomplete object metadata');
            }
            return object;
        }
    }
    throw new Error('R2 did not list the uploaded object');
}

async function listMetadata({ account, bucket, prefix, headers, apiBase = API_BASE }) {
    const base = `${apiBase}/accounts/${encodeURIComponent(account)}/r2/buckets/${encodeURIComponent(bucket)}/objects`;
    const objects = [];
    const keys = new Set();
    for await (const page of metadataPages(base, prefix, headers, 1000)) {
        for (const object of page) {
            if (typeof object.key !== 'string' || !object.key.startsWith(prefix) || keys.has(object.key)) {
                throw new Error('R2 returned invalid or duplicate metadata keys');
            }
            if (object.size !== undefined && (!Number.isSafeInteger(object.size) || object.size < 0)) {
                throw new Error('R2 returned an invalid stored size');
            }
            keys.add(object.key);
            // Store only proof fields. Credentials and custom metadata never enter the receipt.
            objects.push({ key: object.key, size: object.size, etag: object.etag,
                http_metadata: object.http_metadata && {
                    contentType: object.http_metadata.contentType,
                    cacheControl: object.http_metadata.cacheControl,
                    contentEncoding: object.http_metadata.contentEncoding,
                } });
        }
    }
    return objects;
}

async function metadataMain([script, account, bucket, prefix, output]) {
    const objects = await listMetadata({ account, bucket, prefix, headers: credentials(script) });
    fs.writeFileSync(output, JSON.stringify(objects), { flag: 'wx' });
}

async function readback({ account, bucket, key, encoding, downloaded, metadata, headers, apiBase = API_BASE }) {
    const base = `${apiBase}/accounts/${encodeURIComponent(account)}/r2/buckets/${encodeURIComponent(bucket)}/objects`;
    const stored = await objectMetadata(base, key, headers);
    const objectUrl = new URL(`${base}/${key.split('/').map(encodeURIComponent).join('/')}`);
    const response = await request(objectUrl, { ...headers, Accept: 'application/octet-stream', 'Accept-Encoding': encoding });
    if (encoding === 'br' && (response.headers['content-encoding'] === undefined || response.headers['content-encoding'] === 'identity')) {
        response.destroy();
        throw new Error('R2 returned a decoded Brotli body. Exact stored bytes cannot be verified');
    }
    if (response.headers.etag !== JSON.stringify(stored.etag)) {
        response.destroy();
        throw new Error('R2 object changed between metadata and body reads');
    }
    const receipt = {
        stored_size: stored.size,
        stored_encoding: stored.http_metadata?.contentEncoding ?? null,
        body_encoding: response.headers['content-encoding'] ?? null,
    };
    await pipeline(response, fs.createWriteStream(downloaded, { flags: 'wx' }));
    fs.writeFileSync(metadata, JSON.stringify(receipt), { flag: 'wx' });
}

async function main([script, account, bucket, key, encoding, downloaded, metadata]) {
    await readback({ script, account, bucket, key, encoding, downloaded, metadata, headers: credentials(script) });
}

module.exports = { credentials, readback, listMetadata };
