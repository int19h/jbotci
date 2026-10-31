#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_f2llm_runtime::{TokenWindowRule, token_weighted_mean_normalized};
use serde_json::Value;

const CURRENT_80M: &[u8] =
    include_bytes!("../testdata/goldens/current-v0.4.0/f2llm-v2-80m-q4-320/goldens.json");

#[test]
#[requires(true)]
#[ensures(true)]
fn windows_and_weighted_pooling_match_the_independent_04_reference() {
    let golden: Value = serde_json::from_slice(CURRENT_80M).unwrap();
    assert_eq!(golden["runtime_version"], "0.4.0");
    let rule = TokenWindowRule::new(512).unwrap();
    for case in golden["cases"].as_array().unwrap() {
        let windows = case["windows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| {
                w.as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u32)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let content = windows
            .iter()
            .flat_map(|w| w[..w.len() - 1].iter().copied())
            .collect::<Vec<_>>();
        assert_eq!(rule.split(&content, 151645), windows);
        let vectors = case["window_embeddings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| {
                w["embedding"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64().unwrap() as f32)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let counts = windows.iter().map(Vec::len).collect::<Vec<_>>();
        let actual = token_weighted_mean_normalized(&vectors, &counts, 320).unwrap();
        for (value, expected) in actual.iter().zip(case["embedding"].as_array().unwrap()) {
            assert!(
                (f64::from(*value) - expected.as_f64().unwrap()).abs() < 0.000_001,
                "{}",
                case["name"]
            );
        }
    }
}
