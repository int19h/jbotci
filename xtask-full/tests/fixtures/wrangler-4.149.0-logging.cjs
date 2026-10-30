const fs = require('node:fs');
const path = require('node:path');

const FAKE_TOKEN = 'FAKE_PUBLICATION_TOKEN_1068_REGRESSION_ONLY';
const FORCED = { WRANGLER_WRITE_LOGS: 'false', WRANGLER_LOG: 'log',
    WRANGLER_LOG_SANITIZE: 'true', WRANGLER_SEND_METRICS: 'false', CI: 'true' };
const REMOVED = ['WRANGLER_LOG_PATH', 'WRANGLER_OUTPUT_FILE_PATH', 'WRANGLER_OUTPUT_FILE_DIRECTORY',
    'DEBUG', 'NODE_DEBUG', 'NODE_DEBUG_NATIVE', 'NODE_OPTIONS', 'NODE_V8_COVERAGE',
    'SSLKEYLOGFILE', 'LOG_STREAM', 'LOG_TOKENS'];

// This function is verbatim from Wrangler 4.149.0, wrangler-dist/cli.js:69710.
function shouldLogToDisk(isTestEnvironment = typeof vitest !== "undefined") {
  if (isTestEnvironment) {
    return false;
  }
  const setting = process.env.WRANGLER_WRITE_LOGS?.toLowerCase();
  return setting !== "false" && setting !== "0";
}

if (require.main === module) {
    const directory = process.env.TEST_OUTPUT_DIR;
    const observed = {};
    for (const name of [...Object.keys(FORCED), ...REMOVED]) observed[name] = process.env[name] ?? null;
    fs.writeFileSync(path.join(directory, 'environment.json'), JSON.stringify(observed));
    const json = JSON.stringify({ type: 'api_token', token: FAKE_TOKEN });
    // Logger.doLog appends to disk before it applies the console level filter.
    if (shouldLogToDisk()) fs.appendFileSync(path.join(directory, 'wrangler.log'), json);
    if (process.argv[2] === 'auth') {
        if (process.env.WRANGLER_LOG !== 'error' && process.env.WRANGLER_LOG !== 'none') console.log(json);
    } else {
        if (process.env.WRANGLER_LOG === 'debug') console.error(json);
        console.log('upload stub completed');
    }
}

module.exports = { FAKE_TOKEN, FORCED, REMOVED };
