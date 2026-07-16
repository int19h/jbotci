# jbotci Lojban for VS Code

This extension connects VS Code to the language server built into
[`jbotci`](https://github.com/int19h/jbotci). The server currently provides
diagnostics, hover information, and semantic tokens as those capabilities are
available in the installed `jbotci` version. Capabilities are negotiated with
the server; the extension does not assume that a particular feature is present.

The extension recognizes `.jbo` files as the `lojban` language. Files ending in
`.jbo.md` remain ordinary Markdown documents, so VS Code's Markdown preview,
outline, links, and other providers keep working while jbotci providers are
stacked alongside them. You can opt into jbotci for every Markdown document,
but Markdown URLs, code fences, and HTML may produce noisy diagnostics until
structural Markdown support is available.

The editor word pattern treats apostrophes and commas as word-internal Lojban
characters, while periods remain word delimiters.

## Requirements

Install `jbotci` and make it available on `PATH`, or configure the absolute path
to the executable with `jbotci.serverPath`. The extension launches:

```text
<server> lsp --stdio
```

## Settings

| Setting | Default | Description |
| --- | --- | --- |
| `jbotci.serverPath` | `""` | Absolute path to the `jbotci` executable. An empty value uses `jbotci` from `PATH`. |
| `jbotci.enableInAllMarkdown` | `false` | Enable jbotci language features in every Markdown document. |

The extension activates when VS Code opens Markdown, but it does not start the
language server merely for an unrelated Markdown file. It starts immediately
when a `.jbo`, `.jbo.md`, or opted-in Markdown document is already open;
otherwise an open-document listener defers startup until one appears. Changing
either setting restarts the client when an eligible document is open.

Run **jbotci: Restart jbotci Language Server** from the Command Palette to
restart the client and server.

## Local build and installation

From `editors/vscode/`:

```sh
npm ci
npm run compile
npm run lint
npx @vscode/vsce package
code --install-extension ./jbotci-lojban-*.vsix
```

The headless server smoke test uses the current `jbotci lsp` spelling. Point it
at a particular build with `JBOTCI_SERVER`:

```sh
JBOTCI_SERVER=/absolute/path/to/jbotci npm run smoke
```

The extension itself passes `--stdio`, the conventional explicit transport
flag expected by editor clients.
