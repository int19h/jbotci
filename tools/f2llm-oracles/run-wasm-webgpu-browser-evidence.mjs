#!/usr/bin/env node

import { spawn } from "node:child_process";
import { writeFile } from "node:fs/promises";

function parseArgs() {
  const values = new Map();
  for (let index = 2; index < process.argv.length; index += 2) {
    const key = process.argv[index];
    const value = process.argv[index + 1];
    if (!key?.startsWith("--") || value === undefined) {
      throw new Error(`invalid argument at position ${index}`);
    }
    values.set(key.slice(2), value);
  }
  for (const required of ["chrome", "url", "output", "chrome-log", "profile-dir"]) {
    if (!values.has(required)) {
      throw new Error(`missing required argument: --${required}`);
    }
  }
  return values;
}

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

async function waitForDebugger(port, deadline) {
  const endpoint = `http://127.0.0.1:${port}/json/list`;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(endpoint);
      if (response.ok) {
        const targets = await response.json();
        const page = targets.find((target) => target.type === "page");
        if (page?.webSocketDebuggerUrl) {
          return page.webSocketDebuggerUrl;
        }
      }
    } catch {
      // Chrome has not opened the debugger socket yet.
    }
    await delay(100);
  }
  throw new Error("timed out waiting for the Chrome DevTools endpoint");
}

function connectCdp(webSocketUrl) {
  const socket = new WebSocket(webSocketUrl);
  const pending = new Map();
  let nextId = 1;
  socket.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    if (message.id === undefined) {
      return;
    }
    const request = pending.get(message.id);
    if (!request) {
      return;
    }
    pending.delete(message.id);
    if (message.error) {
      request.reject(new Error(message.error.message));
    } else {
      request.resolve(message.result);
    }
  });
  const opened = new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });
  return {
    async send(method, params = {}) {
      await opened;
      const id = nextId;
      nextId += 1;
      const result = new Promise((resolve, reject) => {
        pending.set(id, { resolve, reject });
      });
      socket.send(JSON.stringify({ id, method, params }));
      return result;
    },
    close() {
      socket.close();
    },
  };
}

async function main() {
  const args = parseArgs();
  const timeoutSeconds = Number(args.get("timeout-seconds") ?? "1200");
  if (!Number.isFinite(timeoutSeconds) || timeoutSeconds <= 0) {
    throw new Error("--timeout-seconds must be a positive number");
  }
  const port = Number(args.get("debug-port") ?? "9224");
  if (!Number.isSafeInteger(port) || port <= 0 || port > 65535) {
    throw new Error("--debug-port must be a valid TCP port");
  }

  const chromeArgs = [
    "--headless",
    "--no-sandbox",
    "--disable-dev-shm-usage",
    "--disable-gpu-sandbox",
    "--enable-unsafe-webgpu",
    "--use-angle=vulkan",
    "--enable-features=Vulkan,WebGPUDeveloperFeatures",
    "--enable-dawn-features=allow_unsafe_apis",
    `--remote-debugging-port=${port}`,
    `--user-data-dir=${args.get("profile-dir")}`,
    args.get("url"),
  ];
  const chrome = spawn(args.get("chrome"), chromeArgs, {
    env: {
      ...process.env,
      VK_ICD_FILENAMES: "/usr/share/vulkan/icd.d/lvp_icd.json",
      VK_DRIVER_FILES: "/usr/share/vulkan/icd.d/lvp_icd.json",
    },
    stdio: ["ignore", "ignore", "pipe"],
  });
  let chromeLog = "";
  chrome.stderr.setEncoding("utf8");
  chrome.stderr.on("data", (chunk) => {
    chromeLog += chunk;
  });

  const deadline = Date.now() + timeoutSeconds * 1000;
  let cdp;
  try {
    const webSocketUrl = await waitForDebugger(port, deadline);
    cdp = connectCdp(webSocketUrl);
    await cdp.send("Runtime.enable");
    while (Date.now() < deadline) {
      const evaluated = await cdp.send("Runtime.evaluate", {
        expression: "document.getElementById('result')?.textContent ?? null",
        returnByValue: true,
      });
      const value = evaluated.result?.value;
      if (typeof value === "string" && value !== "pending") {
        const evidence = JSON.parse(value);
        if (evidence.error) {
          throw new Error(`browser evidence failed: ${evidence.error}`);
        }
        await writeFile(args.get("output"), `${JSON.stringify(evidence, null, 2)}\n`);
        return;
      }
      await delay(250);
    }
    throw new Error("timed out waiting for browser evidence");
  } finally {
    cdp?.close();
    chrome.kill("SIGTERM");
    await writeFile(args.get("chrome-log"), chromeLog);
  }
}

await main();
