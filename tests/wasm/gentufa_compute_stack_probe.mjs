import { readFileSync } from "node:fs";
import { performance } from "node:perf_hooks";
import { pathToFileURL } from "node:url";

Error.stackTraceLimit = 100;

// Each case is one compute request measured on its own, because the budget this
// probe guards is a property of a COLD instance: the engine's first compilation
// tier uses larger frames than its optimizing tier, so a case that overflows on
// a fresh instance can succeed on a warmed one. The runner therefore gives every
// case a fresh Node process, and this script parses exactly one case per run.
// Do not "optimize" that into a shared process: it would silently turn the gate
// into a warm-path test that no longer measures what a user's first click does.

/// The exact input and settings from the issue #913 report, including its double
/// spaces, which must be preserved because they are what the user typed.
const ISSUE_913_TEXT =
  "mi pu na djuno lo  du'u  ti cinri lo nu tcidu .i ku'i mi pu na drani";

/// One `lo nu` level re-enters the whole bridi and sumti tier chain, so a ladder
/// of them measures the per-level cost that decides whether a user's sentence
/// fits. A fixed depth keeps the gate deterministic; the depth is the assertion.
function nestingLadder(levels) {
  return `${"lo nu ".repeat(levels)}broda`;
}

const CASES = new Map(
  [
    { name: "default-input", text: null },
    { name: "simple-valid", text: "mi klama le zarci" },
    {
      name: "recovered-input",
      text:
        "cadga fa lo nu ro lo prenu goi ko'a cu troci lo nu ko'a tarti lo lo ka ce'u xendo ije cnikansa ro lo jmive kei ta'i lo racli",
    },
    {
      name: "natural-stop-recovered-input",
      text:
        "cadga fa lo nu ro lo prenu goi ko'a cu troci lo nu ko'a tarti li ka ce'u xendo ije cnikansa ro lo jmive ta'i lo racli",
    },
    // The reported configuration and the default one measure identically today.
    // Both are kept so that a future divergence between them is visible instead
    // of hidden behind whichever one we happened to gate on.
    { name: "issue-913-reported", text: ISSUE_913_TEXT, dialect: "(jboponei)", showGlosses: true },
    { name: "issue-913-default-settings", text: ISSUE_913_TEXT },
    { name: "nesting-depth-3", text: nestingLadder(3) },
  ].map((entry) => [entry.name, entry]),
);

const CASE_NAMES = Array.from(CASES.keys());

function usage() {
  return [
    "usage: node --stack-size=<kb> tests/wasm/gentufa_compute_stack_probe.mjs --js <path> --wasm <path> --ready-js <path> --default-text <text> --case <name>",
    "       node tests/wasm/gentufa_compute_stack_probe.mjs --list-cases",
    "",
    "Exactly one case per process: the budget is a cold-instance property.",
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
    if (arg === "--list-cases") {
      // The runner cross-checks this against its own list so a case added on
      // only one side fails the gate instead of being silently skipped.
      console.log(CASE_NAMES.join("\n"));
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
  if (args.cases.length !== 1) {
    throw new Error(
      `expected exactly one --case, got ${args.cases.length}\n${usage()}`,
    );
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

function requestFor(probeCase, defaultText) {
  const text = probeCase.text ?? defaultText;
  const dialect = probeCase.dialect ?? null;
  const showGlosses = probeCase.showGlosses ?? false;
  return {
    type: "gentufa-page",
    base_path: "",
    state: {
      text,
      dialect,
      "view-mode": "blocks",
      "show-elided": false,
      "show-glosses": showGlosses,
    },
    request: {
      text,
      options: {
        dialect,
        "view-mode": "blocks",
        script: "latin",
        "show-elided": false,
        "show-glosses": showGlosses,
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
  if (!caseName.endsWith("recovered-input") && parsed.result?.status !== "success") {
    throw new Error(
      `${caseName}: expected successful gentufa result, got ${JSON.stringify(parsed.result ?? null)}`,
    );
  }
  if (
    caseName.endsWith("recovered-input") &&
    !["success", "error"].includes(parsed.result?.status)
  ) {
    throw new Error(
      `${caseName}: expected structured gentufa result, got ${JSON.stringify(parsed.result ?? null)}`,
    );
  }
  return parsed;
}

const args = parseArgs(process.argv);
const [caseName] = args.cases;
const probeCase = CASES.get(caseName);
const module = await loadAppModule(args.jsPath, args.wasmPath, args.readyJsPath);

const startedAt = performance.now();
try {
  const json = module.jbotciComputeHandle(
    JSON.stringify(requestFor(probeCase, args.defaultText)),
  );
  const elapsedMs = performance.now() - startedAt;
  const parsed = validateResponse(caseName, json);
  console.log(
    `${caseName}: ok ${Math.round(elapsedMs)}ms ${json.length} bytes ${JSON.stringify(parsed.timing ?? null)}`,
  );
} catch (error) {
  const elapsedMs = performance.now() - startedAt;
  console.error(`${caseName}: failed ${Math.round(elapsedMs)}ms`);
  console.error(error?.stack ?? String(error));
  process.exit(1);
}
