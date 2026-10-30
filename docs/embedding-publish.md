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
It also removes Node debug, startup instrumentation, coverage, TLS key logging, and YAML token trace controls.
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
Manifests and catalogs run in order after the data objects pass the read-back comparison.
A failed batch prevents its later manifests and catalogs from publication.

Use this release order after review and merge approval:

1. Merge the accepted pack changes into `main`.
2. Publish the GGUF packs with `cargo xtask publish-gguf-embeddings-r2 --skip-build` from that checkout.
3. Publish the web packs with `cargo xtask publish-f2llm-webgpu-r2 --skip-build` from the same checkout.
4. Dispatch `render-image.yml` on `main`.

The Render image build needs the published GGUF packs.
A merge does not dispatch the workflow.
These commands write remote objects, so run them only with explicit publication authority.
