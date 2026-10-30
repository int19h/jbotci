const fs = require('node:fs');
const http = require('node:http');
const https = require('node:https');
const { spawnSync } = require('node:child_process');
const { pipeline } = require('node:stream/promises');

const API_BASE = 'https://api.cloudflare.com/client/v4';

function credentials(script) {
    const result = spawnSync(process.execPath, [script, 'auth', 'token', '--json'], {
        env: process.env,
        stdio: ['ignore', 'pipe', 'inherit'],
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

async function objectMetadata(base, key, headers) {
    const seen = new Set();
    let cursor;
    do {
        const url = new URL(base);
        url.searchParams.set('prefix', key);
        url.searchParams.set('per_page', '1');
        if (cursor) url.searchParams.set('cursor', cursor);
        const response = await request(url, { ...headers, Accept: 'application/json', 'Accept-Encoding': 'identity' });
        const chunks = [];
        for await (const chunk of response) chunks.push(chunk);
        let value;
        try { value = JSON.parse(Buffer.concat(chunks).toString('utf8')); }
        catch { throw new Error('R2 did not return valid metadata JSON'); }
        if (value.success !== true || !Array.isArray(value.result)) throw new Error('R2 did not return object metadata');
        const object = value.result.find(object => object.key === key);
        if (object) {
            if (!Number.isSafeInteger(object.size) || object.size < 0 || typeof object.etag !== 'string' || !object.etag.length) {
                throw new Error('R2 returned incomplete object metadata');
            }
            return object;
        }
        cursor = value.result_info?.cursor;
        if (cursor && (typeof cursor !== 'string' || seen.has(cursor))) throw new Error('R2 returned an invalid metadata cursor');
        if (cursor) seen.add(cursor);
    } while (cursor);
    throw new Error('R2 did not list the uploaded object');
}

async function readback({ account, bucket, key, encoding, downloaded, metadata, headers, apiBase = API_BASE }) {
    const base = `${apiBase}/accounts/${encodeURIComponent(account)}/r2/buckets/${encodeURIComponent(bucket)}/objects`;
    const stored = await objectMetadata(base, key, headers);
    const objectUrl = new URL(`${base}/${key.split('/').map(encodeURIComponent).join('/')}`);
    const response = await request(objectUrl, { ...headers, Accept: 'application/octet-stream', 'Accept-Encoding': encoding });
    // A decoded representation can use a weak ETag for the same stored object.
    if (response.headers.etag?.replace(/^W\//, '') !== JSON.stringify(stored.etag)) {
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

module.exports = { credentials, readback };
