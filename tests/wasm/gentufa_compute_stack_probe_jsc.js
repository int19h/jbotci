// JavaScriptCore does not provide browser or Node globals. This harness adapts
// the self-contained release Dioxus module just enough to initialize its
// unchanged Wasm and repeatedly call the exported compute entry point. It exercises WebKit's
// engine family on Linux, but its stack limit is not interchangeable with an
// Apple WebKit/Safari limit because the JSC build and embedding differ.

globalThis.console = { log: print, warn: print, error: print, debug() {} };

globalThis.TextEncoder = class TextEncoder {
  encode(text) {
    const bytes = unescape(encodeURIComponent(text));
    return Uint8Array.from(bytes, (character) => character.charCodeAt(0));
  }

  encodeInto(text, target) {
    const bytes = this.encode(text);
    target.set(bytes);
    return { read: text.length, written: bytes.length };
  }
};

globalThis.TextDecoder = class TextDecoder {
  decode(bytes) {
    if (bytes === undefined) return "";
    let encoded = "";
    for (let offset = 0; offset < bytes.length; offset += 4096) {
      encoded += String.fromCharCode(...bytes.subarray(offset, offset + 4096));
    }
    return decodeURIComponent(escape(encoded));
  }
};

function usage() {
  return "usage: jsc [engine options] gentufa_compute_stack_probe_jsc.js -- <app.js> <app.wasm> <default-text> <iterations>";
}

function exportedBinding(source, exportName) {
  const identifier = "([A-Za-z_$][\\w$]*)";
  const escapedExportName = exportName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const alias = source.match(new RegExp(`${identifier} as ${escapedExportName}\\b`));
  if (alias !== null) return alias[1];

  const declaration = source.match(
    new RegExp(`export\\s+(?:async\\s+)?function\\s+(${escapedExportName})\\b`),
  );
  if (declaration !== null) return declaration[1];

  const directList = source.match(
    new RegExp(`export\\s*\\{[^}]*\\b(${escapedExportName})\\b(?:\\s*,|\\s*\\})`),
  );
  if (directList !== null) return directList[1];

  throw new Error(`generated app module does not export ${exportName}`);
}

if (arguments.length !== 4) throw new Error(usage());
const [jsPath, wasmPath, defaultText, iterationsText] = arguments;
const iterations = Number(iterationsText);
if (!Number.isSafeInteger(iterations) || iterations < 2) {
  throw new Error(`iterations must be an integer of at least 2; got ${iterationsText}`);
}

let source = readFile(jsPath);
const initBinding = exportedBinding(source, "initSync");
const computeBinding = exportedBinding(source, "jbotciComputeHandle");

const bootstrapMarker = /globalThis\.__wasm_split_main_initSync\s*=\s*/;
const bootstrapMatch = source.match(bootstrapMarker);
if (bootstrapMatch === null) {
  throw new Error("generated app module is missing the Wasm split-main bootstrap marker");
}

// `import.meta` is syntax and cannot appear inside an eval script. The value is
// only used to resolve browser assets, none of which this compute-only harness
// loads after synchronous Wasm initialization.
source = source
  .slice(0, bootstrapMatch.index)
  .replaceAll("import.meta.url", JSON.stringify(`file://${jsPath}`));
source += `;globalThis.__jbotciProbeInit=${initBinding};`;
source += `globalThis.__jbotciProbeCompute=${computeBinding};`;
(0, eval)(source);

globalThis.__jbotciProbeInit({ module: readFile(wasmPath, "binary") });

const request = JSON.stringify({
  type: "gentufa-page",
  base_path: "",
  state: {
    text: "",
    dialect: null,
    "view-mode": "blocks",
    "show-elided": false,
    "show-glosses": false,
    "show-compounds": true,
  },
  request: {
    text: defaultText,
    options: {
      dialect: null,
      "view-mode": "blocks",
      script: "latin",
      "show-elided": false,
      "show-glosses": false,
      "show-compounds": true,
      "show-definitions": false,
      "error-context-depth": 1,
      phonemes: { "mark-stress": "acute", "mark-glides": "breve" },
    },
  },
});

for (let iteration = 0; iteration < iterations; iteration += 1) {
  const startedAt = Date.now();
  try {
    const json = globalThis.__jbotciProbeCompute(request);
    const response = JSON.parse(json);
    if (response.type !== "gentufa-page" || response.result?.status !== "success") {
      throw new Error(`unexpected Gentufa response: ${json.slice(0, 500)}`);
    }
    print(`repeat-default[${iteration + 1}/${iterations}]: ok ${Date.now() - startedAt}ms ${json.length} bytes`);
  } catch (error) {
    print(`repeat-default[${iteration + 1}/${iterations}]: failed ${Date.now() - startedAt}ms`);
    throw error;
  }
}
