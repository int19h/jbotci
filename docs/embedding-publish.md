# Publish embedding packs

The three R2 publishers use Wrangler 4.149.0 from one local installation.
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
Set `CLOUDFLARE_API_TOKEN` with access to the selected R2 bucket, or use existing Wrangler credentials.
`JBOTCI_R2_WRANGLER_DIR` names the installation directory, which contains `node_modules/wrangler`.
`CLOUDFLARE_ACCOUNT_ID` must contain 32 hexadecimal digits.
The publisher passes this account to every upload and download command.
It disables Wrangler metrics and interactive input.

Each upload command has a 180-second limit.
Each read-back command has the same limit.
The publisher downloads the object through Wrangler and compares its size and SHA-256 digest with the source.
A failed command, timeout, or different digest retries the upload and read-back together.
The publisher makes at most three attempts, with waits of two and four seconds.
A final failure names the object and stops publication.
On Unix, a timeout kills the entire command process group and reaps the child.

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
