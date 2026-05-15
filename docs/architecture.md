# Architecture Notes

jbotci v1 is organized as a Cargo workspace. Library crates model the shared
language machinery, while binaries remain thin frontends.

## Crate Direction

The dependency direction should stay acyclic:

```text
jbotci-source
  -> jbotci-morphology
      -> jbotci-syntax
          -> jbotci-semantics
              -> jbotci-output
```

Domain crates such as `jbotci-dictionary`, `jbotci-cll`, `jbotci-search`, and
`jbotci-jvozba` should depend only on the narrower crates they actually need.

The morphology, syntax, and semantics crates are expected to become publishable
public APIs later. API stability is not required during the port, but these
crates should avoid CLI/server assumptions and should stay suitable for WASM.

## Applications

`jbotci` is the CLI application. It owns interactive command-line behavior and
batch transformations.

`jbotci-server` is the long-running server application. It will own the Dioxus
web app serving path and HTTP-facing integrations such as Discord. Dioxus 0.7
supports a workspace layout where frontend and backend crates are selected
explicitly, so the workspace keeps applications separate from reusable crates.

## Resources

Reference material that is required to build, test, or serve jbotci belongs in
this repo. The CLL source is kept as a submodule under `vendor/cll`. Larger or
experimental resources, such as the v0 Lojban wiki snapshot, should be added
only when the corresponding feature is ready to consume them.
