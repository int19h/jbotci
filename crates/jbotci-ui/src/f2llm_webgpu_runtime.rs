//! Stable `wasm_bindgen` ABI shims for the shared F2LLM runtime crate.

#[allow(unused_imports)]
use bityzba::{contract_trait, data, ensures, invariant, new, requires};
use jbotci_f2llm_runtime::{
    ArtifactError, ArtifactPath, ArtifactSource, CorpusShard, CorpusVectorSpec,
    DEFAULT_MAX_SEQUENCE_LENGTH, ProgressError, ProgressEvent, ProgressSink, QwenByteBpeTokenizer,
    RuntimeCapabilities, RuntimeFuture, RuntimeLoadOptions, VectorStore, VectorStoreError,
    VectorStoreKey, WebGpuRuntime,
};
use js_sys::{Array, Float32Array, Function, Object, Promise, Reflect, Uint8Array, Uint32Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
#[invariant(true)]
#[derive(Debug)]
pub struct JbotciF2LlmTokenizer {
    tokenizer: QwenByteBpeTokenizer,
}

#[wasm_bindgen]
impl JbotciF2LlmTokenizer {
    #[wasm_bindgen(js_name = tokenWindows)]
    #[requires(true)]
    #[ensures(true)]
    pub fn token_windows(&self, text: &str, max_length: usize) -> Result<Array, JsValue> {
        let windows = self
            .tokenizer
            .token_windows(text, max_length)
            .map_err(js_error)?;
        let array = Array::new();
        for window in windows {
            array.push(&Uint32Array::from(window.as_slice()));
        }
        Ok(array)
    }
}

#[wasm_bindgen(js_name = jbotciF2LlmTokenizerLoad)]
#[requires(true)]
#[ensures(true)]
pub fn jbotci_f2llm_tokenizer_load(bytes: JsValue) -> Result<JbotciF2LlmTokenizer, JsValue> {
    let bytes = bytes_from_js(&bytes)?;
    let tokenizer = QwenByteBpeTokenizer::from_compact_json(&bytes).map_err(js_error)?;
    Ok(JbotciF2LlmTokenizer { tokenizer })
}

#[wasm_bindgen]
#[invariant(true)]
#[derive(Debug)]
pub struct JbotciF2LlmWebGpuRuntime {
    inner: WebGpuRuntime,
}

#[wasm_bindgen]
impl JbotciF2LlmWebGpuRuntime {
    #[wasm_bindgen(js_name = embedTexts)]
    #[requires(true)]
    #[ensures(true)]
    pub async fn embed_texts(&mut self, texts: Array) -> Result<Array, JsValue> {
        let mut rust_texts = Vec::with_capacity(texts.length() as usize);
        for index in 0..texts.length() {
            let text = texts.get(index);
            let Some(text) = text.as_string() else {
                return Err(JsValue::from_str(&format!(
                    "embedding text row {index} must be a string"
                )));
            };
            rust_texts.push(text);
        }
        let vectors = self
            .inner
            .embed_texts(&rust_texts)
            .await
            .map_err(js_error)?;
        let output = Array::new();
        for vector in vectors {
            output.push(&Float32Array::from(vector.as_slice()));
        }
        Ok(output)
    }

    #[wasm_bindgen(js_name = scoreF16Vectors)]
    #[requires(true)]
    #[ensures(true)]
    pub async fn score_f16_vectors(
        &mut self,
        corpus: JsValue,
        query: Float32Array,
        read_binary: Function,
    ) -> Result<Float32Array, JsValue> {
        let corpus = parse_corpus_vector_spec(&corpus)?;
        let query = query.to_vec();
        let vector_store = JsVectorStore { read_binary };
        let scores = self
            .inner
            .score_f16_vectors(&corpus, &query, &vector_store)
            .await
            .map_err(js_error)?;
        Ok(Float32Array::from(scores.as_slice()))
    }
}

#[wasm_bindgen(js_name = jbotciF2LlmWebGpuRuntimeLoad)]
#[requires(true)]
#[ensures(true)]
pub async fn jbotci_f2llm_webgpu_runtime_load(
    options: JsValue,
    fetch_array_buffer: Function,
    progress: JsValue,
) -> Result<JbotciF2LlmWebGpuRuntime, JsValue> {
    let parsed = ParsedRuntimeLoadOptions::from_js(&options)?;
    let data!(ParsedRuntimeLoadOptions { base_url, runtime }) = parsed.into_data();
    let artifact_source = new!(JsArtifactSource {
        base_url: base_url,
        fetch_array_buffer: fetch_array_buffer,
    });
    let mut progress_sink = JsProgressSink {
        callback: progress.dyn_into::<Function>().ok(),
    };
    let inner = WebGpuRuntime::load(runtime, &artifact_source, &mut progress_sink)
        .await
        .map_err(js_error)?;
    Ok(JbotciF2LlmWebGpuRuntime { inner })
}

#[invariant(!base_url.is_empty())]
struct ParsedRuntimeLoadOptions {
    base_url: String,
    runtime: RuntimeLoadOptions,
}

impl ParsedRuntimeLoadOptions {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|options| !options.base_url.is_empty()) || ret.is_err())]
    fn from_js(value: &JsValue) -> Result<Self, JsValue> {
        let base_url = required_string(value, "baseUrl")?
            .trim_end_matches('/')
            .to_owned();
        if base_url.is_empty() {
            return Err(JsValue::from_str("baseUrl must not be empty"));
        }
        let runtime = RuntimeLoadOptions::new(
            required_string(value, "expectedModelKey")?,
            required_string(value, "expectedRuntime")?,
            required_string(value, "expectedVersion")?,
            optional_usize(value, "maxSequenceLength")?.unwrap_or(DEFAULT_MAX_SEQUENCE_LENGTH),
            required_usize(value, "dimensions")?,
            match optional_string(value, "capabilities")?.as_deref() {
                None | Some("embedding-and-f16-scoring") => {
                    RuntimeCapabilities::EmbeddingAndF16Scoring
                }
                Some("embedding-only") => RuntimeCapabilities::EmbeddingOnly,
                Some(value) => {
                    return Err(JsValue::from_str(&format!(
                        "capabilities must be `embedding-only` or `embedding-and-f16-scoring`, got `{value}`"
                    )));
                }
            },
        );
        Ok(new!(ParsedRuntimeLoadOptions {
            base_url: base_url,
            runtime: runtime,
        }))
    }
}

#[invariant(!base_url.is_empty())]
struct JsArtifactSource {
    base_url: String,
    fetch_array_buffer: Function,
}

#[contract_trait]
impl ArtifactSource for JsArtifactSource {
    fn fetch<'a>(
        &'a self,
        path: &'a ArtifactPath,
    ) -> RuntimeFuture<'a, Result<Vec<u8>, ArtifactError>> {
        Box::pin(async move {
            let url = format!("{}/{}", self.base_url, path.as_str());
            let value = self
                .fetch_array_buffer
                .call2(
                    &JsValue::NULL,
                    &JsValue::from_str(&url),
                    &JsValue::from_str(path.as_str()),
                )
                .map_err(|error| {
                    ArtifactError::unavailable(path.clone(), js_value_message(&error))
                })?;
            promise_bytes(value)
                .await
                .map_err(|message| ArtifactError::unavailable(path.clone(), message))
        })
    }
}

#[invariant(true)]
struct JsVectorStore {
    read_binary: Function,
}

#[contract_trait]
impl VectorStore for JsVectorStore {
    fn read<'a>(
        &'a self,
        key: &'a VectorStoreKey,
    ) -> RuntimeFuture<'a, Result<Vec<u8>, VectorStoreError>> {
        Box::pin(async move {
            let value = self
                .read_binary
                .call1(&JsValue::NULL, &JsValue::from_str(key.as_str()))
                .map_err(|error| VectorStoreError::new(key.clone(), js_value_message(&error)))?;
            promise_bytes(value)
                .await
                .map_err(|message| VectorStoreError::new(key.clone(), message))
        })
    }
}

#[invariant(true)]
struct JsProgressSink {
    callback: Option<Function>,
}

#[contract_trait]
impl ProgressSink for JsProgressSink {
    fn report<'a>(
        &'a mut self,
        event: &'a ProgressEvent,
    ) -> RuntimeFuture<'a, Result<(), ProgressError>> {
        Box::pin(async move {
            let Some(callback) = &self.callback else {
                return Ok(());
            };
            let value = Object::new();
            Reflect::set(
                &value,
                &JsValue::from_str("status"),
                &JsValue::from_str(&event.status),
            )
            .map_err(progress_js_error)?;
            Reflect::set(
                &value,
                &JsValue::from_str("detail"),
                &JsValue::from_str(&event.detail),
            )
            .map_err(progress_js_error)?;
            if let Some(progress) = &event.progress {
                let progress_value = Object::new();
                Reflect::set(
                    &progress_value,
                    &JsValue::from_str("kind"),
                    &JsValue::from_str("model"),
                )
                .map_err(progress_js_error)?;
                Reflect::set(
                    &progress_value,
                    &JsValue::from_str("loaded"),
                    &JsValue::from_f64(progress.loaded as f64),
                )
                .map_err(progress_js_error)?;
                Reflect::set(
                    &progress_value,
                    &JsValue::from_str("total"),
                    &JsValue::from_f64(progress.total as f64),
                )
                .map_err(progress_js_error)?;
                Reflect::set(&value, &JsValue::from_str("progress"), &progress_value)
                    .map_err(progress_js_error)?;
            }
            let result = callback
                .call1(&JsValue::NULL, &value)
                .map_err(progress_js_error)?;
            if result.is_instance_of::<Promise>() {
                JsFuture::from(Promise::from(result))
                    .await
                    .map_err(progress_js_error)?;
            }
            Ok(())
        })
    }
}

#[requires(true)]
#[ensures(true)]
async fn promise_bytes(value: JsValue) -> Result<Vec<u8>, String> {
    let value = JsFuture::from(Promise::from(value))
        .await
        .map_err(|error| js_value_message(&error))?;
    bytes_from_js(&value).map_err(|error| js_value_message(&error))
}

#[requires(true)]
#[ensures(true)]
fn bytes_from_js(value: &JsValue) -> Result<Vec<u8>, JsValue> {
    if value.is_instance_of::<Uint8Array>() {
        return Ok(Uint8Array::new(value).to_vec());
    }
    if value.is_instance_of::<js_sys::ArrayBuffer>() {
        return Ok(Uint8Array::new(value).to_vec());
    }
    Err(JsValue::from_str("expected ArrayBuffer or Uint8Array"))
}

#[requires(true)]
#[ensures(true)]
fn parse_corpus_vector_spec(value: &JsValue) -> Result<CorpusVectorSpec, JsValue> {
    let shards_value = Reflect::get(value, &JsValue::from_str("shards"))?;
    if !Array::is_array(&shards_value) {
        return Err(JsValue::from_str("shards must be an array"));
    }
    let shards_array = Array::from(&shards_value);
    if shards_array.length() == 0 {
        return Err(JsValue::from_str("shards must contain at least one shard"));
    }
    let mut shards = Vec::with_capacity(shards_array.length() as usize);
    for index in 0..shards_array.length() {
        let shard = shards_array.get(index);
        let key = required_string(&shard, "key")?;
        let key = VectorStoreKey::parse(key)
            .map_err(|error| JsValue::from_str(&format!("invalid vector shard key: {error}")))?;
        shards.push(CorpusShard::new(key, required_usize(&shard, "byteLen")?));
    }
    let element_type = required_string(value, "elementType")?;
    if element_type != "f16le" {
        return Err(JsValue::from_str(&format!(
            "elementType must be f16le, got {element_type}"
        )));
    }
    Ok(CorpusVectorSpec::new(
        optional_string(value, "corpusId")?.unwrap_or_default(),
        optional_string(value, "inputHash")?.unwrap_or_default(),
        required_usize(value, "rowCount")?,
        required_usize(value, "dimensions")?,
        shards,
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| !value.is_empty()) || ret.is_err())]
fn required_string(value: &JsValue, key: &str) -> Result<String, JsValue> {
    let value = optional_string(value, key)?
        .ok_or_else(|| JsValue::from_str(&format!("{key} is required")))?;
    if value.is_empty() {
        Err(JsValue::from_str(&format!(
            "{key} must be a non-empty string"
        )))
    } else {
        Ok(value)
    }
}

#[requires(true)]
#[ensures(true)]
fn optional_string(value: &JsValue, key: &str) -> Result<Option<String>, JsValue> {
    let field = Reflect::get(value, &JsValue::from_str(key))?;
    if field.is_undefined() || field.is_null() {
        Ok(None)
    } else {
        field
            .as_string()
            .map(Some)
            .ok_or_else(|| JsValue::from_str(&format!("{key} must be a string")))
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| *value > 0) || ret.is_err())]
fn required_usize(value: &JsValue, key: &str) -> Result<usize, JsValue> {
    optional_usize(value, key)?.ok_or_else(|| JsValue::from_str(&format!("{key} is required")))
}

#[requires(true)]
#[ensures(true)]
fn optional_usize(value: &JsValue, key: &str) -> Result<Option<usize>, JsValue> {
    let field = Reflect::get(value, &JsValue::from_str(key))?;
    if field.is_undefined() || field.is_null() {
        return Ok(None);
    }
    let number = field
        .as_f64()
        .ok_or_else(|| JsValue::from_str(&format!("{key} must be a number")))?;
    if !number.is_finite() || number <= 0.0 || number.fract() != 0.0 {
        return Err(JsValue::from_str(&format!(
            "{key} must be a positive integer"
        )));
    }
    Ok(Some(number as usize))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn js_value_message(value: &JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}

#[requires(true)]
#[ensures(!ret.message.is_empty())]
fn progress_js_error(value: JsValue) -> ProgressError {
    ProgressError::new(js_value_message(&value))
}

#[requires(true)]
#[ensures(true)]
fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
