#!/usr/bin/env node

import { createReadStream } from "node:fs";
import { access, readFile, stat } from "node:fs/promises";
import http from "node:http";
import https from "node:https";
import { dirname, extname, resolve, sep } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(SCRIPT_DIR, "../../..");
const ASSET_PREFIX = "/__jbotci-assets/";
const MODEL_PREFIX = `${ASSET_PREFIX}models/f2llm-v2-80m-webgpu/v1/`;
const VECTORS_PREFIX = `${ASSET_PREFIX}embeddings/web/v1/`;
const Q4_ONNX_PREFIX = "/__jbotci-f2llm-q4-onnx/";
const DEFAULT_Q4_ONNX_PATH =
  "/home/int19h.linux/git/jbotci-f2llm-quant/artifacts/f2llm-v2-80m-q4-hqq32-transformersjs/onnx/model_q4.onnx";

const args = parseArgs(process.argv.slice(2));
if (args.help) {
  printHelp();
  process.exit(0);
}

const host = stringArg(args, "host", "127.0.0.1");
const port = numberArg(args, "port", 7777);
const assetOrigin = optionalStringArg(args, "asset-origin");
const artifactDir = optionalPathArg(args, "artifact-dir");
const vectorsDir = optionalPathArg(args, "vectors-dir");
const q4OnnxPath = optionalPathArg(args, "q4-onnx") ?? await optionalExistingPath(DEFAULT_Q4_ONNX_PATH);
const certPath = optionalPathArg(args, "cert");
const keyPath = optionalPathArg(args, "key");

if ((certPath === null) !== (keyPath === null)) {
  throw new Error("--cert and --key must be provided together");
}

const serverOptions = certPath === null
  ? null
  : {
      cert: await readFile(certPath),
      key: await readFile(keyPath),
    };

const server = serverOptions === null
  ? http.createServer(handleRequest)
  : https.createServer(serverOptions, handleRequest);

server.listen(port, host, () => {
  const scheme = serverOptions === null ? "http" : "https";
  console.log(`F2LLM WebGPU harness: ${scheme}://${host}:${port}/tools/webgpu-embedding-runtime/browser-harness/`);
  console.log(`repo root: ${REPO_ROOT}`);
  console.log("default model artifact: .jbotci-build/f2llm-v2-80m-webgpu/v1");
  console.log("default vector pack root: .jbotci-build/r2-web-embeddings");
  if (assetOrigin !== null) {
    console.log(`explicit remote asset proxy: ${assetOrigin}`);
  }
  if (artifactDir !== null) {
    console.log(`local model artifact: ${artifactDir}`);
  }
  if (vectorsDir !== null) {
    console.log(`local vector pack root: ${vectorsDir}`);
  }
  if (q4OnnxPath !== null) {
    console.log(`local q4 ONNX: ${q4OnnxPath}`);
  }
});

async function handleRequest(request, response) {
  const requestUrl = new URL(request.url || "/", `http://${request.headers.host || "localhost"}`);
  try {
    if (requestUrl.pathname === "/") {
      redirect(response, "/tools/webgpu-embedding-runtime/browser-harness/");
      return;
    }
    if (requestUrl.pathname.startsWith(ASSET_PREFIX)) {
      await handleAssetRequest(request, response, requestUrl);
      return;
    }
    if (requestUrl.pathname === `${Q4_ONNX_PREFIX}model_q4.onnx`) {
      await handleQ4OnnxRequest(request, response);
      return;
    }
    await serveFile(REPO_ROOT, requestUrl.pathname, request, response);
  } catch (error) {
    console.error(error);
    sendText(response, 500, `internal harness server error: ${error instanceof Error ? error.message : String(error)}\n`);
  }
}

async function handleQ4OnnxRequest(request, response) {
  if (q4OnnxPath === null) {
    sendText(
      response,
      404,
      `q4 ONNX file is not configured; pass --q4-onnx or create ${DEFAULT_Q4_ONNX_PATH}\n`,
    );
    return;
  }
  await serveResolvedFile(q4OnnxPath, request, response);
}

async function handleAssetRequest(request, response, requestUrl) {
  if (artifactDir !== null && requestUrl.pathname.startsWith(MODEL_PREFIX)) {
    await serveFile(artifactDir, requestUrl.pathname.slice(MODEL_PREFIX.length), request, response);
    return;
  }
  if (vectorsDir !== null && requestUrl.pathname.startsWith(VECTORS_PREFIX)) {
    await serveFile(vectorsDir, requestUrl.pathname.slice(VECTORS_PREFIX.length), request, response);
    return;
  }
  if (assetOrigin !== null) {
    await proxyAssetRequest(request, response, requestUrl);
    return;
  }
  sendText(
    response,
    404,
    "asset proxy is disabled for local testing; use local .jbotci-build paths or pass --asset-origin explicitly\n",
  );
}

async function serveFile(root, rawPath, request, response) {
  const filePath = await resolveFilePath(root, rawPath);
  if (filePath === null) {
    sendText(response, 404, "not found\n");
    return;
  }
  await serveResolvedFile(filePath, request, response);
}

async function serveResolvedFile(filePath, request, response) {
  const fileStat = await stat(filePath);
  if (!fileStat.isFile()) {
    sendText(response, 404, "not found\n");
    return;
  }
  const range = parseRange(request.headers.range, fileStat.size);
  if (range?.invalid) {
    response.writeHead(416, {
      "Content-Range": `bytes */${fileStat.size}`,
      "Content-Type": "text/plain; charset=utf-8",
    });
    response.end("range not satisfiable\n");
    return;
  }

  const headers = {
    "Content-Type": contentType(filePath),
    "Cache-Control": "no-cache",
  };
  if (range !== null) {
    headers["Content-Length"] = String(range.end - range.start + 1);
    headers["Content-Range"] = `bytes ${range.start}-${range.end}/${fileStat.size}`;
    headers["Accept-Ranges"] = "bytes";
    response.writeHead(206, headers);
    if (request.method !== "HEAD") {
      createReadStream(filePath, { start: range.start, end: range.end }).pipe(response);
    } else {
      response.end();
    }
    return;
  }

  headers["Content-Length"] = String(fileStat.size);
  headers["Accept-Ranges"] = "bytes";
  response.writeHead(200, headers);
  if (request.method !== "HEAD") {
    createReadStream(filePath).pipe(response);
  } else {
    response.end();
  }
}

async function resolveFilePath(root, rawPath) {
  let decodedPath;
  try {
    decodedPath = decodeURIComponent(rawPath);
  } catch {
    return null;
  }
  if (decodedPath.includes("\0")) {
    return null;
  }
  const relativePath = decodedPath.replace(/^\/+/, "");
  let filePath = resolve(root, relativePath);
  if (!isPathInside(root, filePath)) {
    return null;
  }
  let fileStat;
  try {
    fileStat = await stat(filePath);
  } catch {
    return null;
  }
  if (fileStat.isDirectory()) {
    filePath = resolve(filePath, "index.html");
    if (!isPathInside(root, filePath)) {
      return null;
    }
    try {
      await access(filePath);
    } catch {
      return null;
    }
  }
  return filePath;
}

function proxyAssetRequest(request, response, requestUrl) {
  const targetPath = requestUrl.pathname.slice(ASSET_PREFIX.length);
  const targetUrl = new URL(`${targetPath}${requestUrl.search}`, `${assetOrigin}/`);
  const client = targetUrl.protocol === "http:" ? http : https;
  const headers = { ...request.headers, host: targetUrl.host };
  delete headers.origin;

  const proxyRequest = client.request(
    targetUrl,
    {
      method: request.method,
      headers,
    },
    (proxyResponse) => {
      response.writeHead(proxyResponse.statusCode || 502, proxyResponse.headers);
      proxyResponse.pipe(response);
    },
  );
  proxyRequest.on("error", (error) => {
    sendText(response, 502, `asset proxy failed: ${error.message}\n`);
  });
  request.pipe(proxyRequest);
}

function parseRange(rangeHeader, fileSize) {
  if (typeof rangeHeader !== "string") {
    return null;
  }
  const match = /^bytes=(\d*)-(\d*)$/.exec(rangeHeader.trim());
  if (match === null) {
    return { invalid: true };
  }
  const [, rawStart, rawEnd] = match;
  if (rawStart === "" && rawEnd === "") {
    return { invalid: true };
  }
  let start;
  let end;
  if (rawStart === "") {
    const suffixLength = Number(rawEnd);
    if (!Number.isSafeInteger(suffixLength) || suffixLength <= 0) {
      return { invalid: true };
    }
    start = Math.max(0, fileSize - suffixLength);
    end = fileSize - 1;
  } else {
    start = Number(rawStart);
    end = rawEnd === "" ? fileSize - 1 : Number(rawEnd);
  }
  if (
    !Number.isSafeInteger(start)
    || !Number.isSafeInteger(end)
    || start < 0
    || end < start
    || start >= fileSize
  ) {
    return { invalid: true };
  }
  return { start, end: Math.min(end, fileSize - 1) };
}

function redirect(response, path) {
  response.writeHead(302, { Location: path });
  response.end();
}

function sendText(response, statusCode, text) {
  response.writeHead(statusCode, {
    "Content-Type": "text/plain; charset=utf-8",
    "Content-Length": String(Buffer.byteLength(text)),
  });
  response.end(text);
}

function contentType(filePath) {
  switch (extname(filePath)) {
    case ".html":
      return "text/html; charset=utf-8";
    case ".js":
    case ".mjs":
      return "text/javascript; charset=utf-8";
    case ".json":
      return "application/json; charset=utf-8";
    case ".wasm":
      return "application/wasm";
    case ".onnx":
    case ".bin":
      return "application/octet-stream";
    case ".f16":
      return "application/octet-stream";
    case ".css":
      return "text/css; charset=utf-8";
    case ".png":
      return "image/png";
    case ".svg":
      return "image/svg+xml";
    default:
      return "application/octet-stream";
  }
}

function isPathInside(root, path) {
  const resolvedRoot = resolve(root);
  const resolvedPath = resolve(path);
  return resolvedPath === resolvedRoot || resolvedPath.startsWith(`${resolvedRoot}${sep}`);
}

function parseArgs(argv) {
  const parsed = {};
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help" || arg === "-h") {
      parsed.help = true;
      continue;
    }
    if (!arg.startsWith("--")) {
      throw new Error(`unexpected argument: ${arg}`);
    }
    const name = arg.slice(2);
    const value = argv[index + 1];
    if (value === undefined || value.startsWith("--")) {
      throw new Error(`missing value for --${name}`);
    }
    parsed[name] = value;
    index += 1;
  }
  return parsed;
}

function stringArg(parsed, name, fallback) {
  const value = parsed[name];
  return value === undefined ? fallback : String(value);
}

function optionalStringArg(parsed, name) {
  const value = parsed[name];
  return value === undefined ? null : trimTrailingSlash(String(value));
}

function numberArg(parsed, name, fallback) {
  const value = Number(stringArg(parsed, name, String(fallback)));
  if (!Number.isSafeInteger(value) || value <= 0 || value > 65535) {
    throw new Error(`--${name} must be a TCP port`);
  }
  return value;
}

function optionalPathArg(parsed, name) {
  const value = parsed[name];
  return value === undefined ? null : resolve(String(value));
}

async function optionalExistingPath(path) {
  try {
    await access(path);
  } catch {
    return null;
  }
  return path;
}

function trimTrailingSlash(value) {
  return value.replace(/\/+$/, "");
}

function printHelp() {
  console.log(`Usage: node tools/webgpu-embedding-runtime/browser-harness/server.mjs [options]

Options:
  --host <host>             Bind host. Default: 127.0.0.1
  --port <port>             Bind port. Default: 7777
  --asset-origin <url>      Explicit remote asset origin proxy, e.g. https://assets.jbotci.app.
  --artifact-dir <path>     Serve local F2LLM model artifact instead of proxying it.
  --vectors-dir <path>      Serve local web embedding vector root instead of proxying it.
  --q4-onnx <path>          Serve this q4 ONNX model at ${Q4_ONNX_PREFIX}model_q4.onnx.
  --cert <path> --key <path> Serve HTTPS for iOS/macOS WebGPU testing.
`);
}
