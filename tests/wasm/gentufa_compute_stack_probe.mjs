import { readFileSync } from "node:fs";
import { performance } from "node:perf_hooks";
import { pathToFileURL } from "node:url";

Error.stackTraceLimit = 100;

const CASE_NAMES = ["default-input", "simple-valid", "recovered-input"];

function casesFor(defaultText) {
  return new Map([
    ["default-input", defaultText],
    ["simple-valid", "mi klama le zarci"],
    [
      "recovered-input",
      "cadga fa lo nu ro lo prenu goi ko'a cu troci lo nu ko'a tarti lo lo ka ce'u xendo ije cnikansa ro lo jmive kei ta'i lo racli",
    ],
  ]);
}

function usage() {
  return [
    "usage: node --stack-size=<kb> tests/wasm/gentufa_compute_stack_probe.mjs --js <path> --wasm <path> --ready-js <path> --default-text <text> [--case <name>]...",
    "",
    "cases:",
    ...CASE_NAMES.map((name) => `  ${name}`),
  ].join("\n");
}

function parseArgs(argv) {
  const args = {
    jsPath: null,
    wasmPath: null,
    readyJsPath: null,
    defaultText: null,
    cases: [],
  };
  for (let i = 2; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--help" || arg === "-h") {
      console.log(usage());
      process.exit(0);
    }
    if (arg === "--js") {
      args.jsPath = requiredValue(argv, ++i, arg);
      continue;
    }
    if (arg === "--wasm") {
      args.wasmPath = requiredValue(argv, ++i, arg);
      continue;
    }
    if (arg === "--ready-js") {
      args.readyJsPath = requiredValue(argv, ++i, arg);
      continue;
    }
    if (arg === "--default-text") {
      args.defaultText = requiredValue(argv, ++i, arg);
      continue;
    }
    if (arg === "--case") {
      args.cases.push(requiredValue(argv, ++i, arg));
      continue;
    }
    throw new Error(`unknown argument: ${arg}\n${usage()}`);
  }
  if (!args.jsPath) {
    throw new Error(`missing --js\n${usage()}`);
  }
  if (!args.wasmPath) {
    throw new Error(`missing --wasm\n${usage()}`);
  }
  if (!args.readyJsPath) {
    throw new Error(`missing --ready-js\n${usage()}`);
  }
  if (args.defaultText === null) {
    throw new Error(`missing --default-text\n${usage()}`);
  }
  if (args.cases.length === 0) {
    args.cases = CASE_NAMES;
  }
  for (const name of args.cases) {
    if (!CASE_NAMES.includes(name)) {
      throw new Error(`unknown case: ${name}\n${usage()}`);
    }
  }
  return args;
}

function requiredValue(argv, index, option) {
  const value = argv[index];
  if (value === undefined || value.startsWith("--")) {
    throw new Error(`missing value for ${option}\n${usage()}`);
  }
  return value;
}

function requestFor(text) {
  return {
    type: "gentufa-page",
    base_path: "",
    state: {
      text,
      dialect: null,
      "view-mode": "blocks",
      "show-elided": false,
      "show-glosses": false,
    },
    request: {
      text,
      options: {
        dialect: null,
        "view-mode": "blocks",
        script: "latin",
        "show-elided": false,
        "show-glosses": false,
        "show-definitions": false,
        "error-context-depth": 1,
        phonemes: {
          "mark-stress": "acute",
          "mark-glides": "breve",
        },
      },
    },
  };
}

async function loadAppModule(jsPath, wasmPath, readyJsPath) {
  const wasmBytes = readFileSync(wasmPath);
  globalThis.fetch = async () =>
    new Response(wasmBytes, {
      status: 200,
      headers: { "Content-Type": "application/wasm" },
    });
  const readinessModule = await import(pathToFileURL(readyJsPath).href);
  const module = await import(pathToFileURL(jsPath).href);
  await readinessModule.waitForAppModuleReady(module);
  if (typeof module.jbotciComputeHandle !== "function") {
    throw new Error("Dioxus app module does not export jbotciComputeHandle");
  }
  return module;
}

function validateResponse(caseName, json) {
  let parsed;
  try {
    parsed = JSON.parse(json);
  } catch (error) {
    throw new Error(`${caseName}: response is not valid JSON: ${error.message}`);
  }
  if (parsed.type !== "gentufa-page") {
    throw new Error(`${caseName}: expected gentufa-page response, got ${parsed.type}`);
  }
  if (caseName !== "recovered-input" && parsed.result?.status !== "success") {
    throw new Error(
      `${caseName}: expected successful gentufa result, got ${JSON.stringify(parsed.result ?? null)}`,
    );
  }
  if (
    caseName === "recovered-input" &&
    !["success", "error"].includes(parsed.result?.status)
  ) {
    throw new Error(
      `${caseName}: expected structured gentufa result, got ${JSON.stringify(parsed.result ?? null)}`,
    );
  }
  return parsed;
}

const args = parseArgs(process.argv);
const cases = casesFor(args.defaultText);
const module = await loadAppModule(args.jsPath, args.wasmPath, args.readyJsPath);
let failed = false;

for (const caseName of args.cases) {
  const text = cases.get(caseName);
  const startedAt = performance.now();
  try {
    const json = module.jbotciComputeHandle(JSON.stringify(requestFor(text)));
    const elapsedMs = performance.now() - startedAt;
    const parsed = validateResponse(caseName, json);
    console.log(
      `${caseName}: ok ${Math.round(elapsedMs)}ms ${json.length} bytes ${JSON.stringify(parsed.timing ?? null)}`,
    );
  } catch (error) {
    failed = true;
    const elapsedMs = performance.now() - startedAt;
    console.error(`${caseName}: failed ${Math.round(elapsedMs)}ms`);
    console.error(error?.stack ?? String(error));
  }
}

if (failed) {
  process.exit(1);
}
