import { spawn } from 'node:child_process';

const server = process.env.JBOTCI_SERVER ?? 'jbotci';
const child = spawn(server, ['lsp'], {
  stdio: ['pipe', 'pipe', 'pipe'],
  windowsHide: true,
});

let stdout = Buffer.alloc(0);
let stderr = '';
let initialized = false;
let shutdown = false;
let finished = false;

const timeout = setTimeout(() => {
  fail('timed out waiting for the jbotci language server');
}, 15_000);

child.stderr.setEncoding('utf8');
child.stderr.on('data', (chunk) => {
  stderr += chunk;
});

child.on('error', (error) => {
  fail(`could not spawn ${server}: ${error.message}`);
});

child.stdout.on('data', (chunk) => {
  stdout = Buffer.concat([stdout, chunk]);
  readMessages();
});

child.on('exit', (code, signal) => {
  if (finished) {
    return;
  }
  if (!shutdown || code !== 0) {
    fail(
      `server exited before a clean shutdown (code ${String(code)}, signal ${String(signal)})${stderr.length > 0 ? `: ${stderr}` : ''}`,
    );
    return;
  }
  finished = true;
  clearTimeout(timeout);
  process.stdout.write(
    'jbotci lsp smoke passed: initialize, shutdown, and exit completed\n',
  );
});

send({
  jsonrpc: '2.0',
  id: 1,
  method: 'initialize',
  params: {
    processId: process.pid,
    capabilities: {},
  },
});

function send(message) {
  const body = Buffer.from(JSON.stringify(message));
  child.stdin.write(`Content-Length: ${body.length}\r\n\r\n`);
  child.stdin.write(body);
}

function readMessages() {
  while (true) {
    const headerEnd = stdout.indexOf('\r\n\r\n');
    if (headerEnd < 0) {
      return;
    }
    const header = stdout.subarray(0, headerEnd).toString('ascii');
    const match = /^Content-Length:\s*(\d+)$/im.exec(header);
    if (match === null) {
      fail(`server returned an invalid LSP header: ${header}`);
      return;
    }
    const bodyLength = Number(match[1]);
    const messageEnd = headerEnd + 4 + bodyLength;
    if (stdout.length < messageEnd) {
      return;
    }
    const body = stdout.subarray(headerEnd + 4, messageEnd);
    stdout = stdout.subarray(messageEnd);
    handleMessage(JSON.parse(body.toString('utf8')));
  }
}

function handleMessage(message) {
  if (message.id === 1) {
    if (message.error !== undefined || message.result?.capabilities === undefined) {
      fail(`initialize failed: ${JSON.stringify(message)}`);
      return;
    }
    initialized = true;
    send({ jsonrpc: '2.0', method: 'initialized', params: {} });
    send({ jsonrpc: '2.0', id: 2, method: 'shutdown', params: null });
    return;
  }
  if (message.id === 2) {
    if (!initialized || message.error !== undefined || message.result !== null) {
      fail(`shutdown failed: ${JSON.stringify(message)}`);
      return;
    }
    shutdown = true;
    send({ jsonrpc: '2.0', method: 'exit', params: null });
    child.stdin.end();
  }
}

function fail(message) {
  if (finished) {
    return;
  }
  finished = true;
  clearTimeout(timeout);
  child.kill();
  process.stderr.write(`jbotci lsp smoke failed: ${message}\n`);
  process.exitCode = 1;
}
