#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  printf 'usage: %s /path/to/zantufa-1.js /path/to/upstream-parity.json\n' "$0" >&2
  exit 64
fi

parser_js=$1
fixture_json=$2

if [[ ! -f "$parser_js" ]]; then
  printf 'Zantufa parser not found: %s\n' "$parser_js" >&2
  exit 66
fi

node -e '
const fs = require("fs");
const childProcess = require("child_process");

const parser = process.argv[1];
const fixturePath = process.argv[2];
const fixture = JSON.parse(fs.readFileSync(fixturePath, "utf8"));

for (const testCase of fixture.cases) {
  const result = childProcess.spawnSync("node", [parser, testCase.source], {
    encoding: "utf8",
  });
  const accepted = result.status === 0;
  console.log(`${accepted ? "OK" : "FAIL"}\t${testCase.id}\t${testCase.source}`);
  if (accepted !== testCase.upstreamAccept) {
    console.error(`fixture mismatch for ${testCase.id}: expected upstreamAccept=${testCase.upstreamAccept}`);
    if (result.stderr) {
      console.error(result.stderr);
    }
    process.exitCode = 1;
  }
}
' "$parser_js" "$fixture_json"
