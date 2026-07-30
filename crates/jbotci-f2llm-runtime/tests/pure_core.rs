#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_f2llm_runtime::{Sha256Digest, TokenWindow, mean_pool_normalized, pack_token_windows};
use serde_json::Value;

const CURRENT_80M: &[u8] =
    include_bytes!("../testdata/goldens/current-v0.2.0/f2llm-v2-80m-q4-320/goldens.json");

#[test]
#[requires(true)]
#[ensures(true)]
fn n0_windows_and_wasm_pooling_bytes_match_the_pre_move_baseline() {
    let golden: Value = serde_json::from_slice(CURRENT_80M).expect("current N0 golden parses");
    assert_eq!(golden["schema_version"], 2);
    assert_eq!(golden["runtime"], "jbotci-webgpu-f2llm");
    assert_eq!(golden["runtime_version"], "0.2.0");
    let dimensions = golden["dimensions"].as_u64().expect("dimensions") as usize;
    let max_sequence_length = golden["max_sequence_length"]
        .as_u64()
        .expect("maximum sequence length") as usize;
    assert_eq!(dimensions, 320);
    assert_eq!(max_sequence_length, 512);

    let cases = golden["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 13);
    let mut all_windows = Vec::new();
    for (text_index, case) in cases.iter().enumerate() {
        let name = case["name"].as_str().expect("case name");
        let token_ids = json_u32s(case["token_ids"].as_array().expect("token IDs"));
        let windows = case["windows"]
            .as_array()
            .expect("windows")
            .iter()
            .map(|window| json_u32s(window.as_array().expect("window")))
            .collect::<Vec<_>>();
        assert_eq!(
            windows.iter().flatten().copied().collect::<Vec<_>>(),
            token_ids,
            "exact token/window structure changed for {name}"
        );
        assert!(
            windows
                .iter()
                .all(|window| !window.is_empty() && window.len() <= max_sequence_length)
        );
        for token_ids in &windows {
            all_windows.push(TokenWindow::new(text_index, token_ids.clone()));
        }

        let window_vectors = case["window_embeddings"]
            .as_array()
            .expect("window embeddings")
            .iter()
            .map(|window| json_f32s(window["embedding"].as_array().expect("window embedding")))
            .collect::<Vec<_>>();
        assert_eq!(window_vectors.len(), windows.len());
        let embedding = mean_pool_normalized(&window_vectors, dimensions);
        let bytes = f32le_bytes(&embedding);
        assert_eq!(
            Sha256Digest::of_bytes(&bytes).as_str(),
            pre_move_wasm_embedding_digest(name),
            "bit-identical pooled f32 bytes changed for {name}"
        );
    }

    let batch_shape = pack_token_windows(&all_windows, max_sequence_length)
        .into_iter()
        .map(|batch| {
            serde_json::json!({
                "total_tokens": batch.total_tokens,
                "segments": batch.segments.iter().map(|segment| serde_json::json!({
                    "text_index": segment.text_index,
                    "token_count": segment.token_ids.len(),
                })).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        Value::Array(batch_shape),
        serde_json::json!([
            {"total_tokens":512,"segments":[{"text_index":10,"token_count":512}]},
            {"total_tokens":512,"segments":[{"text_index":11,"token_count":512}]},
            {"total_tokens":512,"segments":[{"text_index":12,"token_count":512}]},
            {"total_tokens":512,"segments":[{"text_index":12,"token_count":512}]},
            {"total_tokens":512,"segments":[
                {"text_index":9,"token_count":511},
                {"text_index":0,"token_count":1}
            ]},
            {"total_tokens":114,"segments":[
                {"text_index":4,"token_count":29},
                {"text_index":3,"token_count":25},
                {"text_index":2,"token_count":23},
                {"text_index":1,"token_count":11},
                {"text_index":7,"token_count":7},
                {"text_index":8,"token_count":7},
                {"text_index":5,"token_count":5},
                {"text_index":6,"token_count":5},
                {"text_index":11,"token_count":1},
                {"text_index":12,"token_count":1}
            ]}
        ])
    );
}

#[requires(values.iter().all(|value| value.as_u64().is_some()))]
#[ensures(ret.len() == values.len())]
fn json_u32s(values: &[Value]) -> Vec<u32> {
    values
        .iter()
        .map(|value| value.as_u64().expect("u32 token ID") as u32)
        .collect()
}

#[requires(values.iter().all(|value| value.as_f64().is_some()))]
#[ensures(ret.len() == values.len())]
fn json_f32s(values: &[Value]) -> Vec<f32> {
    values
        .iter()
        .map(|value| value.as_f64().expect("f32 embedding component") as f32)
        .collect()
}

#[requires(true)]
#[ensures(ret.len() == values.len() * 4)]
fn f32le_bytes(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

#[requires(!name.is_empty())]
#[ensures(ret.len() == 64)]
fn pre_move_wasm_embedding_digest(name: &str) -> &'static str {
    match name {
        "empty" => "60caeb31fe6bb4e1d0e6426ac239bf43dc03f4b3e8b35f43a626f3a9aadaf90a",
        "non-ascii" => "49a4b0eec8a4512f4a806062ab09570c8bf600aea4bcfe4edead29281c33ce06",
        "query-coi-ro-do" => "cb1276091f9526277a5d5fad68487c3b046f715196f673afcbff10fa4fb4f016",
        "query-klama-zarci" => "23cb3d199942786ae293775fead22707ffe2178b64df6da044d96689eddc33d6",
        "document-klama-definition" => {
            "dcc24867e65073a23d467b4c825151324c321c36c7405a51f68fd79384c9f275"
        }
        "batch-filler-5" => "96b7af9fccc6067ba097b931553b57b9742e7f12e1f57004f20cbc3423485d8b",
        "batch-filler-6" => "cce916e152268e50fc46d77ca08f14eaeab04cb2c174fe2b4529967a23c21ad9",
        "batch-last-slot" => "3aac834660ba2f60b3f8671640a96fca6ef21eb4b1bd952e7582d95426eb1208",
        "batch-next-slot" => "4c0fe511654a4085f0aafb9508fac529206736d71a9aac8f61f2b6defe67bd67",
        "token-length-511" => "1a6ddf91c578d4f4c0d52fef28015398fc99afdd79300be5a3b7bc24f6261c15",
        "token-length-512" => "38100e4507c2d0b0984bf6d5127b110f1d57f1520ed90922c595f1ab568b5bdd",
        "token-length-513" => "d78fd1010a75d1b9de079b2eee1bf080990bd47f4c4f88560a5645e772084fa8",
        "multi-window-1025" => "7f9e7a7c5cb056fa074875d432cf39ac316377612f0537623a5436cbee70a8d2",
        other => panic!("unrecognized N0 golden case: {other}"),
    }
}
