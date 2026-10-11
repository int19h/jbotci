# Architecture Notes

jbotci v1 is organized as a Cargo workspace. Library crates model the shared
language machinery, while binaries remain thin frontends.

## Crate Direction

The dependency direction is acyclic:

```text
jbotci-source
  -> jbotci-morphology
      -> jbotci-syntax
          -> jbotci-semantics
              -> jbotci-output
```

Domain crates such as `jbotci-dictionary`, `jbotci-cll`, `jbotci-search`, and
`jbotci-jvozba` depend only on the narrower crates that they use.

The morphology, syntax, and semantics crates contain shared language APIs.
They do not require CLI or server behavior and support WebAssembly hosts.
API stability is not required for these shared language crates.

## Applications

`jbotci` is the CLI application. It owns interactive command-line behavior and
batch transformations.

`jbotci-server` serves the Dioxus web application and HTTP integrations such as
Discord. Dioxus 0.7
supports a workspace layout where frontend and backend crates are selected
explicitly, so the workspace keeps applications separate from reusable crates.

The Discord application lives in `apps/jbotci-server/src/discord`. It calls the
same shared analyses the CLI and the web app call and presents them as Discord
messages; see [the Discord application](discord-app.md) for its configuration,
bounds and registration.

## Resources

Reference material that is required to build, test, or serve jbotci belongs in
this repo. The CLL source is a submodule under `vendor/cll`.
Add larger or experimental resources only when a feature consumes them.
