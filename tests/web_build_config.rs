#[allow(unused_imports)]
use bityzba::{ensures, requires};

#[test]
#[requires(true)]
#[ensures(true)]
fn production_web_optimization_avoids_parser_forwarding_frames() {
    let config: toml::Value = toml::from_str(include_str!("../apps/jbotci-app/Dioxus.toml"))
        .expect("the production Dioxus configuration must parse");
    let wasm_opt = &config["web"]["wasm_opt"];

    assert_eq!(wasm_opt["level"].as_str(), Some("z"));
    let arguments = wasm_opt["extra_features"]
        .as_array()
        .expect("Dioxus must forward the configured wasm-opt arguments");
    assert!(
        arguments.windows(2).any(|pair| {
            pair[0].as_str() == Some("--skip-pass")
                && pair[1].as_str() == Some("merge-similar-functions")
        }),
        "the production optimizer must skip function sharing that adds native parser frames"
    );
}
