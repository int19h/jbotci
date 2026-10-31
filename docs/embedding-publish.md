# Publish embedding packs

Run the R2 publishers on Linux or macOS.
They refuse non-Unix hosts before they read credentials, build files, or start uploads.
They use Wrangler 4.149.0 from one local installation.
They invoke the installed CLI with Node 22 or later and never resolve packages through npx.
The publisher reads the installed package version before it starts a build or upload.
A missing installation, wrong version, or missing account stops the command.

Install Wrangler once on the publishing host, before you start any publisher:

```sh
export JBOTCI_R2_WRANGLER_DIR=/build/jbotci/tools/r2-publisher
npm install --prefix "$JBOTCI_R2_WRANGLER_DIR" --save-exact wrangler@4.149.0
export CLOUDFLARE_ACCOUNT_ID=fe55e0fbc7fbba2097b5f1e31957470f
```

Keep this installation unchanged while publishers run.
Set `CLOUDFLARE_API_TOKEN` with object read and write access, or use existing Wrangler credentials.
`JBOTCI_R2_WRANGLER_DIR` names the installation directory, which contains `node_modules/wrangler`.
`CLOUDFLARE_ACCOUNT_ID` must contain 32 hexadecimal digits.
The publisher passes this account to every upload and download command.
It disables Wrangler metrics and interactive input.

The read-back obtains credentials from the pinned Wrangler `auth token --json` command.
The publisher captures credential stdout and stderr in memory.
It forces `WRANGLER_WRITE_LOGS=false` for every child, including the credential child.
This overrides any value from the parent environment.
The pinned CLI otherwise writes credential JSON to its log file.
Capturing stdout alone does not stop that file write.

The publisher forces `WRANGLER_LOG=log` and `WRANGLER_LOG_SANITIZE=true`.
It removes inherited Wrangler log paths and structured output paths.
It removes Node debug, startup instrumentation, TLS key logging, and YAML token trace controls.
It forces `NODE_V8_COVERAGE` to an empty value because Node forwards that variable when a child environment omits it.
These controls cannot turn child logging back on.

Each upload command has a 180-second limit.
Each read-back command has the same limit.
The read-back uses Node's raw HTTP client and the Cloudflare API.
The same 180-second limit covers credentials, metadata, and body download.
The request advertises `br` for a Brotli upload and `identity` for an uncompressed upload.
Node keeps the response bytes encoded when the server returns `Content-Encoding: br`.

The publisher lists the exact object to read its stored size and Content-Encoding.
It compares the metadata ETag with the body ETag to refuse an object that changes between reads.
The response must match the uploaded file through size and SHA-256.
For a Brotli upload, only an encoded response can succeed.
A decoded response fails because it does not prove the identity of the stored bytes.
The planner also validates each Brotli sibling against the original file before publication.

Stored size and encoding must match the upload.
An unexpected encoding or a different body stops the attempt.
Wrangler's fetch client decodes compressed responses, so `r2 object get` does not preserve stored Brotli bytes.
The [Cloudflare compression guide](https://developers.cloudflare.com/speed/optimization/content/compression/) describes response transformations and content negotiation.
The [R2 object API](https://developers.cloudflare.com/api/resources/r2/subresources/buckets/subresources/objects/) describes metadata and body reads.

A failed command, timeout, or different digest retries the upload and read-back together.
The publisher makes at most three attempts, with waits of two and four seconds.
A final failure names the object and stops publication.
A timeout kills the entire command process group and reaps the child.

Read-back files use `CARGO_TARGET_DIR/r2-readback`, or `target/r2-readback` when the environment variable is absent.
The publisher removes each temporary directory after success or failure.
Four data uploads can run together.
Manifests and catalogs run in order after all required data comparisons pass.
A failed batch prevents its later manifests and catalogs from publication.

## Skip unchanged data

The default mode uploads and reads back every object, as before.
Add `--skip-unchanged` to any R2 publish command to reuse matching data objects.
This mode still uploads and reads back every manifest and catalog.
The order is data objects, their sampled comparison, manifests, and then catalogs.
A failed data upload or sample prevents the later manifests and catalogs.

The publisher obtains a fresh REST listing for each target prefix before its data uploads.
It requests up to 1,000 entries per page and follows the returned cursors.
The [R2 listing API](https://developers.cloudflare.com/api/resources/r2/subresources/buckets/subresources/objects/methods/list/) can include size, ETag, and HTTP metadata.
This lets one listing supply the proof fields without a separate request for each object.
A listing error discards the prefix receipt and uses the normal upload path.
No cached listing or earlier publication receipt can authorize a skip.

A skip requires the stored size, content-type, cache-control, and content-encoding to equal the intended upload values.
It also requires a plain 32-digit hexadecimal MD5 ETag that equals the local upload file's MD5.
The size and digest describe the encoded `.br` file when the planner selects a Brotli sibling.

An absent `contentEncoding` field in a present HTTP metadata object means that the stored object has no Content-Encoding.
It matches only an upload that sets no encoding.
A present encoding must match exactly, and an empty value is malformed.
The publisher does not replace absence with `identity`.

A missing HTTP metadata object, content-type, or cache-control still prevents a skip.
The publisher always sets content-type and cache-control, so their absence cannot prove a match.
A multipart ETag, malformed value, other missing proof field, or mismatch uses the normal upload and read-back path.

After the data uploads, the publisher reads back a random sample of the skipped objects.
For each prefix batch, it samples the larger of 20 objects and 1% rounded up, limited to the skipped count.
It compares these downloads with the local files through the existing size, encoding, and SHA-256 comparison.
A different body or a failed read stops the publish.
The sampler never repairs a mismatch by silently uploading that object.

The publisher prints its random seed and the sample size for each prefix batch.
Use `--skip-unchanged --skip-sample-seed NUMBER` to reproduce a sample with the same publisher build and object plan.
The sampler sorts keys before it shuffles them, so listing and filesystem order do not change the sample.
The seed option requires `--skip-unchanged`.
A command without the flag keeps the established full comparison.

Each object emits a JSON status line for `skipped` or `uploaded-verified`.
A skipped line includes the matching live metadata.
Sampled objects also emit `sample-verified` after the full comparison succeeds.
At command exit, the publisher prints totals by prefix and corpus, including partial totals after a failed publish.
Model assets and objects outside a corpus pack use the `shared` corpus label.

Use this release order after review and merge approval:

1. Merge the accepted pack changes into `main`.
2. Publish the GGUF packs with `cargo xtask publish-gguf-embeddings-r2 --skip-build` from that checkout.
3. Publish the web packs with `cargo xtask publish-f2llm-webgpu-r2 --skip-build` from the same checkout.
4. Dispatch `render-image.yml` on `main`.

The Render image build needs the published GGUF packs.
A merge does not dispatch the workflow.
These commands write remote objects, so run them only with explicit publication authority.
