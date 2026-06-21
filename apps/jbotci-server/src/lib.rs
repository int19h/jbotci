//! HTTP server for the jbotci web app and API integrations.

mod discord;
mod mcp;

use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use anyhow::{Context, Result, anyhow};
use axum::body::Body;
use axum::extract::Extension;
use axum::http::header::{
    ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_TYPE, HOST, HeaderMap, HeaderValue,
    LOCATION,
};
use axum::http::{Response, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use dioxus::server::{DioxusRouterExt, FullstackState};
use jbotci_cli::{
    TOOL_DEFAULT_EMBEDDING_MODEL_KEY, ToolCuktaRequest, ToolEmbeddingSearchService,
    ToolExecutionContext, ToolRenderedOutput, ToolVlackuRequest, run_tool_cukta,
    run_tool_cukta_with_context, run_tool_vlacku, run_tool_vlacku_with_context,
};
use jbotci_web_core::{
    FAVICON_ASSET_PATH, GentufaError, GentufaExportFormat, GentufaWebRequest, GentufaWebResult,
    MANIFEST_ASSET_PATH, META_BLOCK_END, META_BLOCK_START, PageMeta, WebFeatureAvailability,
    WebRoute, build_page_meta, parse_gentufa_for_web, parse_gentufa_web_export_request,
    parse_web_route, render_gentufa_state_web_export, render_page_head_metadata_block,
    web_route_url,
};
use serde::Serialize;

#[derive(Debug, Clone)]
#[invariant(true)]
pub struct ServerConfig {
    pub address: SocketAddr,
    pub base_path: String,
    pub public_dir: PathBuf,
}

#[derive(Debug, Clone)]
#[invariant(true)]
pub(crate) struct AppState {
    base_path: String,
    public_dir: PathBuf,
    tool_services: ToolServices,
}

impl AppState {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn tool_services(&self) -> ToolServices {
        self.tool_services.clone()
    }
}

#[derive(Debug, Clone)]
#[invariant(true)]
pub(crate) struct ToolServices {
    embedding_worker: Arc<Mutex<mpsc::Sender<EmbeddingToolJob>>>,
}

#[invariant(true)]
struct EmbeddingToolJob {
    request: EmbeddingToolRequest,
    response: mpsc::SyncSender<Result<ToolRenderedOutput>>,
}

#[invariant(::Cukta { .. } => true)]
#[invariant(::Vlacku { .. } => true)]
enum EmbeddingToolRequest {
    Cukta { request: ToolCuktaRequest },
    Vlacku { request: ToolVlackuRequest },
}

#[invariant(!message.trim().is_empty())]
#[derive(Debug, Clone)]
struct CachedEmbeddingError {
    message: String,
}

#[invariant(::Unloaded => true)]
#[invariant(::Loaded { .. } => true)]
#[invariant(::Unavailable { .. } => true)]
#[derive(Debug)]
enum EmbeddingSearchCache {
    Unloaded,
    Loaded { service: ToolEmbeddingSearchService },
    Unavailable { error: CachedEmbeddingError },
}

impl ToolServices {
    #[requires(true)]
    #[ensures(true)]
    fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        spawn_embedding_worker(receiver);
        Self {
            embedding_worker: Arc::new(Mutex::new(sender)),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub(crate) fn run_cukta(&self, request: ToolCuktaRequest) -> Result<ToolRenderedOutput> {
        if request.uses_semantic_search() {
            self.run_embedding_request(EmbeddingToolRequest::Cukta { request })
        } else {
            run_tool_cukta(request)
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub(crate) fn run_vlacku(&self, request: ToolVlackuRequest) -> Result<ToolRenderedOutput> {
        if request.uses_semantic_search() {
            self.run_embedding_request(EmbeddingToolRequest::Vlacku { request })
        } else {
            run_tool_vlacku(request)
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn run_embedding_request(&self, request: EmbeddingToolRequest) -> Result<ToolRenderedOutput> {
        let (response, received) = mpsc::sync_channel(1);
        let job = EmbeddingToolJob { request, response };
        self.embedding_worker
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .send(job)
            .map_err(|_| anyhow!("embedding search worker is not running"))?;
        received
            .recv()
            .map_err(|_| anyhow!("embedding search worker stopped before returning output"))?
    }
}

#[requires(true)]
#[ensures(true)]
fn spawn_embedding_worker(receiver: mpsc::Receiver<EmbeddingToolJob>) {
    thread::Builder::new()
        .name("jbotci-embedding-search".to_owned())
        .spawn(move || embedding_worker_loop(receiver))
        .expect("embedding search worker thread starts");
}

#[requires(true)]
#[ensures(true)]
fn embedding_worker_loop(receiver: mpsc::Receiver<EmbeddingToolJob>) {
    let mut cache = EmbeddingSearchCache::Unloaded;
    for job in receiver {
        let EmbeddingToolJob { request, response } = job;
        let output = run_embedding_tool_request(request, &mut cache);
        if response.send(output).is_err() {
            continue;
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_embedding_tool_request(
    request: EmbeddingToolRequest,
    cache: &mut EmbeddingSearchCache,
) -> Result<ToolRenderedOutput> {
    match request {
        EmbeddingToolRequest::Cukta { request } => run_with_embedding_cache(cache, |context| {
            run_tool_cukta_with_context(request, context)
        }),
        EmbeddingToolRequest::Vlacku { request } => run_with_embedding_cache(cache, |context| {
            run_tool_vlacku_with_context(request, context)
        }),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_with_embedding_cache(
    cache: &mut EmbeddingSearchCache,
    runner: impl FnOnce(&mut ToolExecutionContext<'_>) -> Result<ToolRenderedOutput>,
) -> Result<ToolRenderedOutput> {
    if matches!(cache, EmbeddingSearchCache::Unloaded) {
        *cache =
            match ToolEmbeddingSearchService::load(TOOL_DEFAULT_EMBEDDING_MODEL_KEY, None, None) {
                Ok(service) => EmbeddingSearchCache::Loaded { service },
                Err(error) => EmbeddingSearchCache::Unavailable {
                    error: new!(CachedEmbeddingError {
                        message: error.to_string(),
                    }),
                },
            };
    }
    match cache {
        EmbeddingSearchCache::Loaded { service } => {
            let mut context = ToolExecutionContext::with_embedding_search(service);
            runner(&mut context)
        }
        EmbeddingSearchCache::Unavailable { error } => {
            let mut context =
                ToolExecutionContext::embedding_search_unavailable(error.message.clone());
            runner(&mut context)
        }
        EmbeddingSearchCache::Unloaded => unreachable!("embedding search cache is initialized"),
    }
}

#[derive(Debug, Clone, Serialize)]
#[invariant(true)]
struct HealthResponse {
    status: &'static str,
    features: WebFeatureAvailability,
}

const SERVICE_WORKER_ASSET_PATH: &str = "/service-worker.js";
const DIOXUS_PUBLIC_PATH_ENV: &str = "DIOXUS_PUBLIC_PATH";

#[requires(true)]
#[ensures(ret.base_path.starts_with('/'))]
pub fn config_from_env() -> ServerConfig {
    let address = dioxus::cli_config::fullstack_address_or_localhost();
    let base_path = normalize_base_path(dioxus::cli_config::base_path().as_deref().unwrap_or("/"));
    let public_dir = public_dir_from_env_or_exe(
        std::env::var_os(DIOXUS_PUBLIC_PATH_ENV).map(PathBuf::from),
        std::env::current_exe().ok().as_deref(),
    );
    ServerConfig {
        address,
        base_path,
        public_dir,
    }
}

#[requires(config.base_path.starts_with('/'))]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub async fn run_server(config: ServerConfig) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(config.address)
        .await
        .with_context(|| format!("failed to bind `{}`", config.address))?;
    axum::serve(listener, router(config))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server failed")?;
    Ok(())
}

#[requires(true)]
#[ensures(true)]
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let ctrl_c = tokio::signal::ctrl_c();
        let terminate = async {
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(mut signal) => {
                    signal.recv().await;
                }
                Err(error) => {
                    eprintln!("failed to install SIGTERM handler: {error}");
                    std::future::pending::<()>().await;
                }
            }
        };
        tokio::select! {
            result = ctrl_c => {
                if let Err(error) = result {
                    eprintln!("failed to install Ctrl-C handler: {error}");
                }
            }
            () = terminate => {}
        }
    }

    #[cfg(not(unix))]
    {
        if let Err(error) = tokio::signal::ctrl_c().await {
            eprintln!("failed to install Ctrl-C handler: {error}");
        }
    }
}

#[requires(config.base_path.starts_with('/'))]
#[ensures(true)]
pub fn router(config: ServerConfig) -> Router {
    let state = Arc::new(AppState {
        base_path: normalize_base_path(&config.base_path),
        public_dir: config.public_dir,
        tool_services: ToolServices::new(),
    });
    let use_dioxus_static_assets = dioxus_public_dir()
        .as_ref()
        .is_some_and(|public_dir| public_dir.is_dir() && public_dir == &state.public_dir);
    let router = Router::<FullstackState>::new()
        .route("/api/health", get(health))
        .route("/api/features", get(features))
        .route("/api/gentufa", post(gentufa))
        .route("/mcp", get(mcp::mcp_get).post(mcp::mcp_post))
        .route("/discord", post(discord::discord_post))
        .fallback(static_or_spa)
        .layer(Extension(Arc::clone(&state)));
    let router = if use_dioxus_static_assets {
        router.serve_static_assets()
    } else {
        router
    };
    router.with_state(FullstackState::headless())
}

#[requires(true)]
#[ensures(ret.starts_with('/'))]
fn normalize_base_path(base_path: &str) -> String {
    let trimmed = base_path.trim();
    if trimmed.is_empty() || trimmed == "/" {
        return "/".to_owned();
    }
    let with_leading = if trimmed.starts_with('/') {
        trimmed.to_owned()
    } else {
        format!("/{trimmed}")
    };
    with_leading.trim_end_matches('/').to_owned()
}

#[requires(true)]
#[ensures(!ret.as_os_str().is_empty())]
fn public_dir_from_env_or_exe(
    env_public_path: Option<PathBuf>,
    current_exe: Option<&Path>,
) -> PathBuf {
    env_public_path.unwrap_or_else(|| {
        current_exe
            .and_then(Path::parent)
            .map(|parent| parent.join("public"))
            .unwrap_or_else(|| PathBuf::from("public"))
    })
}

#[requires(true)]
#[ensures(true)]
fn dioxus_public_dir() -> Option<PathBuf> {
    std::env::var_os(DIOXUS_PUBLIC_PATH_ENV)
        .map(PathBuf::from)
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .as_deref()
                .and_then(Path::parent)
                .map(|parent| parent.join("public"))
        })
}

#[requires(true)]
#[ensures(true)]
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        features: WebFeatureAvailability::default(),
    })
}

#[requires(true)]
#[ensures(true)]
async fn features() -> Json<WebFeatureAvailability> {
    Json(WebFeatureAvailability::default())
}

#[requires(true)]
#[ensures(true)]
async fn gentufa(Json(request): Json<GentufaWebRequest>) -> Json<GentufaWebResult> {
    Json(parse_gentufa_for_web_blocking(request).await)
}

#[requires(true)]
#[ensures(true)]
async fn parse_gentufa_for_web_blocking(request: GentufaWebRequest) -> GentufaWebResult {
    tokio::task::spawn_blocking(move || parse_gentufa_for_web(&request))
        .await
        .unwrap_or_else(gentufa_task_error)
}

#[requires(true)]
#[ensures(matches!(ret, GentufaWebResult::Error(_)))]
fn gentufa_task_error(error: tokio::task::JoinError) -> GentufaWebResult {
    GentufaWebResult::Error(GentufaError {
        phase: None,
        message: format!("gentufa parse task failed: {error}"),
        diagnostics: Vec::new(),
    })
}

#[requires(true)]
#[ensures(true)]
async fn static_or_spa(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    uri: Uri,
) -> Response<Body> {
    let request_path = uri.path();
    if request_path == "/favicon.ico" {
        if let Some(response) = static_dir_response(
            &state.public_dir,
            FAVICON_ASSET_PATH,
            accepts_brotli(&headers),
        )
        .await
        {
            return response;
        }
        return Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Body::empty())
            .expect("favicon response builder is valid");
    }
    if is_api_request_path(request_path, &state.base_path) {
        return plain_response(StatusCode::NOT_FOUND, "not found");
    }
    if request_path == "/" || (state.base_path != "/" && request_path == state.base_path) {
        let location = landing_page_location(&state.base_path);
        return redirect_response(&location);
    }
    if let Some(format) = gentufa_export_format_for_request_path(request_path, &state.base_path) {
        return gentufa_export_response(format, uri.query().unwrap_or_default().to_owned()).await;
    }
    let Some(asset_path) = asset_path_for_request(request_path, &state.base_path) else {
        return plain_response(StatusCode::NOT_FOUND, "not found");
    };
    if asset_path == "/index.html" {
        return spa_index_response(&state, &headers, &uri)
            .await
            .unwrap_or_else(|| plain_response(StatusCode::NOT_FOUND, "not found"));
    }
    if let Some(response) =
        static_dir_response(&state.public_dir, &asset_path, accepts_brotli(&headers)).await
    {
        return response;
    }
    plain_response(StatusCode::NOT_FOUND, "not found")
}

#[requires(true)]
#[ensures(true)]
async fn spa_index_response(
    state: &AppState,
    headers: &HeaderMap,
    uri: &Uri,
) -> Option<Response<Body>> {
    let bytes = load_index_html_bytes(state).await?;
    let html = String::from_utf8_lossy(&bytes);
    let logical_path = strip_base_path(uri.path(), &state.base_path).unwrap_or_else(|| {
        if uri.path().starts_with('/') {
            uri.path().to_owned()
        } else {
            format!("/{}", uri.path())
        }
    });
    let route = parse_web_route(&logical_path, uri.query().unwrap_or_default());
    let meta = build_page_meta_blocking(state.base_path.clone(), route).await?;
    let rendered = apply_spa_head_metadata(&html, request_origin(headers).as_deref(), &meta);
    Some(asset_response(
        StatusCode::OK,
        "/index.html",
        None,
        Body::from(rendered),
    ))
}

#[requires(true)]
#[ensures(true)]
async fn gentufa_export_response(format: GentufaExportFormat, query: String) -> Response<Body> {
    let request = parse_gentufa_web_export_request(&query);
    let task = tokio::task::spawn_blocking(move || {
        render_gentufa_state_web_export(&request.state, request.script, format)
    })
    .await;
    let export = match task {
        Ok(Ok(export)) => export,
        Ok(Err(error)) => return plain_response(StatusCode::BAD_REQUEST, &error.to_string()),
        Err(error) => {
            return plain_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("gentufa export task failed: {error}"),
            );
        }
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, export.content_type)
        .header(CACHE_CONTROL, "no-cache")
        .body(Body::from(export.bytes))
        .expect("gentufa export response builder is valid")
}

#[requires(base_path.starts_with('/'))]
#[ensures(true)]
async fn build_page_meta_blocking(base_path: String, route: WebRoute) -> Option<PageMeta> {
    tokio::task::spawn_blocking(move || build_page_meta(&base_path, &route))
        .await
        .ok()
}

#[requires(true)]
#[ensures(true)]
async fn load_index_html_bytes(state: &AppState) -> Option<Vec<u8>> {
    let index_path = state.public_dir.join("index.html");
    if let Ok(bytes) = tokio::fs::read(index_path).await {
        return Some(bytes);
    }
    None
}

#[requires(path.starts_with('/'))]
#[requires(base_path.starts_with('/'))]
#[ensures(ret.as_ref().is_none_or(|path| path.starts_with('/')))]
fn asset_path_for_request(path: &str, base_path: &str) -> Option<String> {
    if path.starts_with("/assets/") && has_file_extension(path) {
        return Some(path.to_owned());
    }
    let stripped = strip_base_path(path, base_path)?;
    if stripped == "/" || is_spa_route_path(&stripped) || !has_file_extension(&stripped) {
        return Some("/index.html".to_owned());
    }
    Some(stripped)
}

#[requires(path.starts_with('/'))]
#[requires(base_path.starts_with('/'))]
#[ensures(true)]
fn gentufa_export_format_for_request_path(
    path: &str,
    base_path: &str,
) -> Option<GentufaExportFormat> {
    match strip_base_path(path, base_path).as_deref() {
        Some("/gentufa.png") => Some(GentufaExportFormat::Png),
        Some("/gentufa.svg") => Some(GentufaExportFormat::Svg),
        _ => None,
    }
}

#[requires(path.starts_with('/'))]
#[ensures(true)]
fn is_spa_route_path(path: &str) -> bool {
    path == "/gentufa"
        || path.starts_with("/gentufa/")
        || path == "/vlacku"
        || path.starts_with("/vlacku/")
        || path == "/gimfihi"
        || path == "/gimfi'i"
        || path == "/gimfi%27i"
        || path == "/cukta"
        || path.starts_with("/cukta/")
        || path == "/settings"
        || path.starts_with("/settings/")
}

#[requires(path.starts_with('/'))]
#[requires(base_path.starts_with('/'))]
#[ensures(true)]
fn is_api_request_path(path: &str, base_path: &str) -> bool {
    if path == "/mcp" || path == "/discord" || path.starts_with("/api/") {
        return true;
    }
    strip_base_path(path, base_path).is_some_and(|stripped| {
        stripped == "/mcp" || stripped == "/discord" || stripped.starts_with("/api/")
    })
}

#[requires(path.starts_with('/'))]
#[requires(base_path.starts_with('/'))]
#[ensures(ret.as_ref().is_none_or(|path| path.starts_with('/')))]
fn strip_base_path(path: &str, base_path: &str) -> Option<String> {
    if base_path == "/" {
        return Some(path.to_owned());
    }
    if path == base_path {
        return Some("/".to_owned());
    }
    let prefix = format!("{base_path}/");
    path.strip_prefix(&prefix).map(|rest| format!("/{rest}"))
}

#[requires(base_path.starts_with('/'))]
#[ensures(ret.starts_with('/'))]
fn landing_page_location(base_path: &str) -> String {
    web_route_url(base_path, &WebRoute::default())
}

#[requires(path.starts_with('/'))]
#[ensures(true)]
fn has_file_extension(path: &str) -> bool {
    path.rsplit_once('/')
        .map(|(_, file_name)| file_name.contains('.'))
        .unwrap_or(false)
}

#[requires(true)]
#[ensures(true)]
fn accepts_brotli(headers: &HeaderMap) -> bool {
    headers
        .get(ACCEPT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .any(|encoding| encoding.trim().eq_ignore_ascii_case("br"))
        })
}

#[requires(asset_path.starts_with('/'))]
#[ensures(true)]
async fn static_dir_response(
    static_dir: &Path,
    asset_path: &str,
    accepts_brotli: bool,
) -> Option<Response<Body>> {
    let relative = safe_relative_path(asset_path)?;
    let normal_path = static_dir.join(&relative);
    let (path, logical_path, encoding) = if accepts_brotli {
        let br_path = brotli_sidecar_path(&normal_path);
        if is_regular_file(&br_path).await {
            (br_path, asset_path.to_owned(), Some("br"))
        } else {
            (normal_path, asset_path.to_owned(), None)
        }
    } else {
        (normal_path, asset_path.to_owned(), None)
    };
    let bytes = tokio::fs::read(path).await.ok()?;
    Some(asset_response(
        StatusCode::OK,
        &logical_path,
        encoding,
        Body::from(bytes),
    ))
}

#[requires(true)]
#[ensures(true)]
fn brotli_sidecar_path(path: &Path) -> PathBuf {
    let mut sidecar = path.as_os_str().to_os_string();
    sidecar.push(".br");
    PathBuf::from(sidecar)
}

#[requires(true)]
#[ensures(true)]
async fn is_regular_file(path: &Path) -> bool {
    tokio::fs::metadata(path)
        .await
        .is_ok_and(|metadata| metadata.is_file())
}

#[requires(path.starts_with('/'))]
#[ensures(true)]
fn safe_relative_path(path: &str) -> Option<PathBuf> {
    let mut output = PathBuf::new();
    for component in Path::new(path.trim_start_matches('/')).components() {
        match component {
            Component::Normal(part) => output.push(part),
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => return None,
        }
    }
    Some(output)
}

#[requires(path.starts_with('/'))]
#[ensures(true)]
fn asset_response(
    status: StatusCode,
    path: &str,
    content_encoding: Option<&str>,
    body: Body,
) -> Response<Body> {
    let mut response = Response::builder()
        .status(status)
        .header(CONTENT_TYPE, content_type_for_path(path))
        .header(CACHE_CONTROL, cache_control_for_path(path));
    if let Some(encoding) = content_encoding {
        response = response.header(CONTENT_ENCODING, encoding);
    }
    response
        .body(body)
        .expect("asset response builder is valid")
}

#[requires(true)]
#[ensures(true)]
fn apply_spa_head_metadata(html: &str, origin: Option<&str>, meta: &PageMeta) -> String {
    let without_old_block = remove_managed_meta_block(html);
    let (with_title, inserted_title) = replace_title(&without_old_block, &meta.title);
    let block = render_page_head_metadata_block(origin, meta, !inserted_title);
    if let Some(head_end) = with_title.find("</head>") {
        let mut output = String::with_capacity(with_title.len() + block.len() + 1);
        output.push_str(&with_title[..head_end]);
        output.push_str(&block);
        output.push_str(&with_title[head_end..]);
        output
    } else {
        format!("{with_title}{block}")
    }
}

#[requires(true)]
#[ensures(true)]
fn remove_managed_meta_block(html: &str) -> String {
    let Some(start) = html.find(META_BLOCK_START) else {
        return html.to_owned();
    };
    let Some(end) = html[start..].find(META_BLOCK_END) else {
        return html.to_owned();
    };
    let block_end = start + end + META_BLOCK_END.len();
    let mut output = String::with_capacity(html.len().saturating_sub(block_end - start));
    output.push_str(&html[..start]);
    output.push_str(&html[block_end..]);
    output
}

#[requires(true)]
#[ensures(true)]
fn replace_title(html: &str, title: &str) -> (String, bool) {
    let Some(open_start) = html.find("<title") else {
        return (html.to_owned(), false);
    };
    let Some(open_end_offset) = html[open_start..].find('>') else {
        return (html.to_owned(), false);
    };
    let content_start = open_start + open_end_offset + 1;
    let Some(close_offset) = html[content_start..].find("</title>") else {
        return (html.to_owned(), false);
    };
    let content_end = content_start + close_offset;
    let mut output = String::with_capacity(html.len() + title.len());
    output.push_str(&html[..content_start]);
    output.push_str(&escape_html_text(title));
    output.push_str(&html[content_end..]);
    (output, true)
}

#[requires(true)]
#[ensures(true)]
fn request_origin(headers: &HeaderMap) -> Option<String> {
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get(HOST))
        .and_then(|value| value.to_str().ok())?
        .split(',')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| *value == "http" || *value == "https")
        .unwrap_or("http");
    Some(format!("{scheme}://{host}"))
}

#[requires(true)]
#[ensures(true)]
fn escape_html_text(input: &str) -> String {
    let mut output = String::new();
    for ch in input.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            _ => output.push(ch),
        }
    }
    output
}

#[requires(path.starts_with('/'))]
#[ensures(!ret.is_empty())]
fn content_type_for_path(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, extension)| extension) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("svg") => "image/svg+xml",
        Some("json") | Some("webmanifest") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("otf") => "font/otf",
        Some("ttf") => "font/ttf",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("f32") => "application/octet-stream",
        _ => "application/octet-stream",
    }
}

#[requires(path.starts_with('/'))]
#[ensures(!ret.is_empty())]
fn cache_control_for_path(path: &str) -> &'static str {
    if path == "/index.html"
        || path == MANIFEST_ASSET_PATH
        || path == SERVICE_WORKER_ASSET_PATH
        || path == "/assets/embeddings/web/v1/catalog.json"
    {
        "no-cache"
    } else {
        "public, max-age=31536000, immutable"
    }
}

#[requires(!location.is_empty())]
#[ensures(true)]
fn redirect_response(location: &str) -> Response<Body> {
    Response::builder()
        .status(StatusCode::FOUND)
        .header(
            LOCATION,
            HeaderValue::from_str(location).expect("normalized path is valid header text"),
        )
        .body(Body::empty())
        .expect("redirect response builder is valid")
}

#[requires(true)]
#[ensures(true)]
fn plain_response(status: StatusCode, message: &str) -> Response<Body> {
    (status, message.to_owned()).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::{Method, Request};
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};
    use ed25519_dalek::{Signer, SigningKey};
    use std::sync::atomic::{AtomicU64, Ordering};
    use tower::ServiceExt;

    static TEST_STATIC_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
    static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[requires(true)]
    #[ensures(ret.is_dir())]
    fn test_static_dir() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        let ordinal = TEST_STATIC_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "jbotci-server-spa-test-{}-{nanos}-{ordinal}",
            std::process::id(),
        ));
        std::fs::create_dir_all(&dir).expect("create test static dir");
        std::fs::write(
            dir.join("index.html"),
            "<!doctype html><html><head><title>jbotci</title></head><body><div id=\"main\"></div></body></html>",
        )
        .expect("write test index");
        dir
    }

    #[requires(true)]
    #[ensures(ret.base_path.starts_with('/'))]
    fn test_config(public_dir: PathBuf) -> ServerConfig {
        test_config_with_base_path("/jbotci", public_dir)
    }

    #[requires(base_path.starts_with('/'))]
    #[ensures(ret.base_path.starts_with('/'))]
    fn test_config_with_base_path(base_path: &str, public_dir: PathBuf) -> ServerConfig {
        ServerConfig {
            address: "127.0.0.1:0".parse().expect("valid test address"),
            base_path: base_path.to_owned(),
            public_dir,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    async fn response_text(response: Response<Body>) -> String {
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        String::from_utf8(bytes.to_vec()).expect("utf-8 body")
    }

    #[requires(true)]
    #[ensures(true)]
    async fn response_bytes(response: Response<Body>) -> Vec<u8> {
        to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body")
            .to_vec()
    }

    #[requires(uri.starts_with('/'))]
    #[ensures(true)]
    async fn post_json(app: Router, uri: &str, value: serde_json::Value) -> Response<Body> {
        app.oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(value.to_string()))
                .expect("request"),
        )
        .await
        .expect("response")
    }

    #[requires(true)]
    #[ensures(true)]
    async fn response_json(response: Response<Body>) -> serde_json::Value {
        serde_json::from_str(&response_text(response).await).expect("response JSON")
    }

    #[requires(true)]
    #[ensures(true)]
    fn test_discord_signing_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    #[requires(true)]
    #[ensures(true)]
    fn lock_test_env() -> std::sync::MutexGuard<'static, ()> {
        TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[requires(!public_key_hex.is_empty())]
    #[ensures(true)]
    fn configure_discord_test_env(public_key_hex: &str) {
        // SAFETY: Discord route tests hold TEST_ENV_LOCK while mutating process-wide env vars.
        unsafe {
            std::env::set_var("DISCORD_PUBLIC_KEY", public_key_hex);
            std::env::set_var("JBOTCI_DISCORD_DISABLE_FOLLOWUP", "1");
            std::env::set_var("JBOTCI_PUBLIC_BASE_URL", "https://jbotci.app");
        }
    }

    #[requires(true)]
    #[ensures(ret.len() == bytes.len() * 2)]
    fn hex_bytes(bytes: &[u8]) -> String {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    }

    #[requires(!body.is_empty())]
    #[requires(!timestamp.is_empty())]
    #[ensures(!ret.0.is_empty() && !ret.1.is_empty())]
    fn discord_signature_headers(
        key: &SigningKey,
        body: &str,
        timestamp: &str,
    ) -> (String, String) {
        let mut signed = Vec::with_capacity(timestamp.len() + body.len());
        signed.extend_from_slice(timestamp.as_bytes());
        signed.extend_from_slice(body.as_bytes());
        let signature = key.sign(&signed);
        (timestamp.to_owned(), hex_bytes(&signature.to_bytes()))
    }

    #[requires(true)]
    #[ensures(true)]
    async fn post_signed_discord(
        app: Router,
        value: serde_json::Value,
        key: &SigningKey,
    ) -> Response<Body> {
        let body = value.to_string();
        let (timestamp, signature) = discord_signature_headers(key, &body, "1710000000");
        app.oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/discord")
                .header("X-Signature-Timestamp", timestamp)
                .header("X-Signature-Ed25519", signature)
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("response")
    }

    #[requires(!name.is_empty())]
    #[requires(!value.is_empty())]
    #[ensures(ret.is_object())]
    fn discord_string_option(name: &str, value: &str) -> serde_json::Value {
        serde_json::json!({
            "name": name,
            "type": 3,
            "value": value
        })
    }

    #[requires(!subcommand.is_empty())]
    #[ensures(ret.is_object())]
    fn discord_interaction(subcommand: &str, options: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::json!({
            "type": 2,
            "application_id": "123456789",
            "token": "test-token",
            "data": {
                "name": "jbotci",
                "options": [
                    {
                        "name": subcommand,
                        "type": 1,
                        "options": options
                    }
                ]
            }
        })
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn base_path_normalization_keeps_single_root() {
        assert_eq!(normalize_base_path(""), "/");
        assert_eq!(normalize_base_path("/"), "/");
        assert_eq!(normalize_base_path("jbotci/"), "/jbotci");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn embedding_cache_reuses_cached_unavailable_state() {
        let mut cache = EmbeddingSearchCache::Unavailable {
            error: new!(CachedEmbeddingError {
                message: "cached embedding load failed".to_owned(),
            }),
        };
        let output = run_embedding_tool_request(
            EmbeddingToolRequest::Vlacku {
                request: jbotci_cli::ToolVlackuRequest {
                    mode: jbotci_cli::ToolVlackuMode::Meaning,
                    query: "goer".to_owned(),
                    count: Some(1),
                    word_types: Vec::new(),
                    min_votes: None,
                    min_similarity: None,
                    decompose_lujvo: true,
                    show_etymology: false,
                },
            },
            &mut cache,
        )
        .expect("tool output");

        assert_eq!(output.status, jbotci_cli::ToolStatus::InvalidInput);
        assert_eq!(output.stderr, "vlacku: cached embedding load failed\n");
        assert!(matches!(cache, EmbeddingSearchCache::Unavailable { .. }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tool_services_runs_nonsemantic_vlacku_without_embedding_context() {
        let services = ToolServices::new();
        let output = services
            .run_vlacku(jbotci_cli::ToolVlackuRequest {
                mode: jbotci_cli::ToolVlackuMode::Word,
                query: "klama".to_owned(),
                count: Some(1),
                word_types: Vec::new(),
                min_votes: None,
                min_similarity: None,
                decompose_lujvo: true,
                show_etymology: false,
            })
            .expect("tool output");

        assert_eq!(output.status, jbotci_cli::ToolStatus::Success);
        assert!(output.stderr.is_empty());
        assert!(output.stdout_text().expect("text").contains("klama"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn root_absolute_assets_work_with_non_root_base_path() {
        assert_eq!(
            asset_path_for_request("/assets/app.js", "/jbotci").as_deref(),
            Some("/assets/app.js")
        );
        assert_eq!(
            asset_path_for_request("/jbotci/assets/manifest.webmanifest", "/jbotci").as_deref(),
            Some("/assets/manifest.webmanifest")
        );
        assert_eq!(
            asset_path_for_request("/jbotci/service-worker.js", "/jbotci").as_deref(),
            Some("/service-worker.js")
        );
        assert_eq!(
            asset_path_for_request("/jbotci/cukta", "/jbotci").as_deref(),
            Some("/index.html")
        );
        assert_eq!(
            asset_path_for_request("/jbotci/cukta/section/11.9", "/jbotci").as_deref(),
            Some("/index.html")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pwa_root_assets_are_not_cached_as_immutable() {
        assert_eq!(
            cache_control_for_path("/assets/manifest.webmanifest"),
            "no-cache"
        );
        assert_eq!(cache_control_for_path("/service-worker.js"), "no-cache");
        assert_eq!(
            cache_control_for_path("/assets/icons/jbotci-icon-192.png"),
            "public, max-age=31536000, immutable"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn public_dir_uses_dioxus_env_before_exe_adjacent_default() {
        assert_eq!(
            public_dir_from_env_or_exe(
                Some(PathBuf::from("/tmp/jbotci-public")),
                Some(Path::new("/opt/jbotci/server")),
            ),
            PathBuf::from("/tmp/jbotci-public")
        );
        assert_eq!(
            public_dir_from_env_or_exe(None, Some(Path::new("/opt/jbotci/server"))),
            PathBuf::from("/opt/jbotci/public")
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn health_and_features_routes_return_availability() {
        let app = router(test_config(test_static_dir()));
        let health = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("health response");
        assert_eq!(health.status(), StatusCode::OK);
        let health_json: serde_json::Value =
            serde_json::from_str(&response_text(health).await).expect("health JSON");
        assert_eq!(health_json["status"], "ok");
        assert_eq!(health_json["features"]["gentufa"], true);

        let features = app
            .oneshot(
                Request::builder()
                    .uri("/api/features")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("features response");
        assert_eq!(features.status(), StatusCode::OK);
        let features_json: serde_json::Value =
            serde_json::from_str(&response_text(features).await).expect("features JSON");
        assert_eq!(features_json["cukta"], true);
        assert_eq!(features_json["vlacku"], true);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn gentufa_api_matches_direct_parser() {
        let app = router(test_config(test_static_dir()));
        let request = GentufaWebRequest {
            text: "mi klama".to_owned(),
            options: Default::default(),
        };
        let expected = parse_gentufa_for_web(&request);
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/gentufa")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&request).expect("request JSON"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let actual: GentufaWebResult = serde_json::from_slice(&bytes).expect("response JSON");
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn missing_api_route_does_not_fall_back_to_spa() {
        let app = router(test_config(test_static_dir()));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/missing")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/jbotci/api/cukta")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn mcp_initialize_and_tool_listing_follow_protocol_shape() {
        let app = router(test_config(test_static_dir()));
        let initialize = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "clientInfo": {
                        "name": "test",
                        "version": "0"
                    }
                }
            }),
        )
        .await;
        assert_eq!(initialize.status(), StatusCode::OK);
        let initialize_json = response_json(initialize).await;
        assert_eq!(initialize_json["result"]["protocolVersion"], "2025-06-18");
        assert!(initialize_json["result"]["capabilities"]["tools"].is_object());

        let tools = post_json(
            app,
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list"
            }),
        )
        .await;
        assert_eq!(tools.status(), StatusCode::OK);
        let tools_json = response_json(tools).await;
        let names = tools_json["result"]["tools"]
            .as_array()
            .expect("tools array")
            .iter()
            .map(|tool| tool["name"].as_str().expect("tool name"))
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec!["cukta", "vlacku", "jvozba", "vlasei", "gentufa", "gimfi'i"]
        );
        assert!(!names.contains(&"tersmu"));
        for tool in tools_json["result"]["tools"]
            .as_array()
            .expect("tools array")
        {
            assert_eq!(tool["inputSchema"]["additionalProperties"], false);
            assert_eq!(tool["annotations"]["readOnlyHint"], true);
            assert_eq!(tool["annotations"]["destructiveHint"], false);
            assert_eq!(tool["annotations"]["openWorldHint"], false);
        }
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn mcp_calls_return_text_structured_json_images_and_tool_errors() {
        let app = router(test_config(test_static_dir()));
        let text = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "text",
                "method": "tools/call",
                "params": {
                    "name": "gentufa",
                    "arguments": {
                        "text": "mi klama"
                    }
                }
            }),
        )
        .await;
        assert_eq!(text.status(), StatusCode::OK);
        let text_json = response_json(text).await;
        assert_eq!(text_json["result"]["content"][0]["type"], "text");
        assert!(
            text_json["result"]["content"][0]["text"]
                .as_str()
                .expect("text content")
                .trim()
                .len()
                > 0
        );

        for (id, mode, query, expected_text) in [
            ("vlacku-word", "word", "klama", "klama"),
            ("vlacku-regex", "regex", "/^klama$/", "klama"),
            ("vlacku-rafsi-glob", "rafsi-glob", "kla*", "kla"),
            ("vlacku-sound", "sound", "klama", "klama"),
        ] {
            let vlacku = post_json(
                app.clone(),
                "/mcp",
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "method": "tools/call",
                    "params": {
                        "name": "vlacku",
                        "arguments": {
                            "mode": mode,
                            "query": query,
                            "count": 1
                        }
                    }
                }),
            )
            .await;
            assert_eq!(vlacku.status(), StatusCode::OK);
            let vlacku_json = response_json(vlacku).await;
            assert_ne!(vlacku_json["result"]["isError"], true);
            assert!(
                vlacku_json["result"]["content"][0]["text"]
                    .as_str()
                    .expect("vlacku text")
                    .contains(expected_text)
            );
        }

        let structured = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "json",
                "method": "tools/call",
                "params": {
                    "name": "gentufa",
                    "arguments": {
                        "text": "mi klama",
                        "format": "json"
                    }
                }
            }),
        )
        .await;
        assert_eq!(structured.status(), StatusCode::OK);
        let structured_json = response_json(structured).await;
        assert_eq!(structured_json["result"]["content"][0]["type"], "text");
        assert!(structured_json["result"]["structuredContent"].is_object());

        let image = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "png",
                "method": "tools/call",
                "params": {
                    "name": "gentufa",
                    "arguments": {
                        "text": "mi klama",
                        "format": "png"
                    }
                }
            }),
        )
        .await;
        assert_eq!(image.status(), StatusCode::OK);
        let image_json = response_json(image).await;
        assert_eq!(image_json["result"]["content"][0]["type"], "image");
        assert_eq!(image_json["result"]["content"][0]["mimeType"], "image/png");
        assert!(
            image_json["result"]["content"][0]["data"]
                .as_str()
                .expect("image data")
                .len()
                > 100
        );

        let unknown = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "unknown",
                "method": "tools/call",
                "params": {
                    "name": "tersmu",
                    "arguments": {}
                }
            }),
        )
        .await;
        assert_eq!(unknown.status(), StatusCode::OK);
        let unknown_json = response_json(unknown).await;
        assert_eq!(unknown_json["result"]["isError"], true);
        assert!(
            unknown_json["result"]["content"][0]["text"]
                .as_str()
                .expect("error text")
                .contains("Unknown tool")
        );

        let invalid_args = post_json(
            app,
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "invalid",
                "method": "tools/call",
                "params": {
                    "name": "vlacku",
                    "arguments": {
                        "query": "klama",
                        "unexpected": true
                    }
                }
            }),
        )
        .await;
        assert_eq!(invalid_args.status(), StatusCode::OK);
        let invalid_args_json = response_json(invalid_args).await;
        assert_eq!(invalid_args_json["result"]["isError"], true);
        assert!(
            invalid_args_json["result"]["content"][0]["text"]
                .as_str()
                .expect("error text")
                .contains("unknown field")
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn mcp_routes_do_not_fall_back_to_spa() {
        let app = router(test_config(test_static_dir()));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/mcp")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/jbotci/mcp")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/jbotci/discord")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn discord_rejects_missing_and_invalid_signatures() {
        let _env_guard = lock_test_env();
        let key = test_discord_signing_key();
        let public_key = hex_bytes(&key.verifying_key().to_bytes());
        configure_discord_test_env(&public_key);
        let app = router(test_config(test_static_dir()));

        let missing = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/discord")
                    .body(Body::from(serde_json::json!({ "type": 1 }).to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);

        let invalid = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/discord")
                    .header("X-Signature-Timestamp", "1710000000")
                    .header("X-Signature-Ed25519", "00".repeat(64))
                    .body(Body::from(serde_json::json!({ "type": 1 }).to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(invalid.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn discord_accepts_signed_ping_and_subcommands() {
        let _env_guard = lock_test_env();
        let key = test_discord_signing_key();
        let public_key = hex_bytes(&key.verifying_key().to_bytes());
        configure_discord_test_env(&public_key);
        let app = router(test_config(test_static_dir()));

        let ping = post_signed_discord(app.clone(), serde_json::json!({ "type": 1 }), &key).await;
        assert_eq!(ping.status(), StatusCode::OK);
        let ping_json = response_json(ping).await;
        assert_eq!(ping_json["type"], 1);

        for interaction in [
            discord_interaction("gentufa", vec![discord_string_option("text", "mi klama")]),
            discord_interaction("vlasei", vec![discord_string_option("text", "mi klama")]),
            discord_interaction("vlacku", vec![discord_string_option("query", "klama")]),
            discord_interaction(
                "cukta",
                vec![
                    discord_string_option("mode", "word"),
                    discord_string_option("query", "klama"),
                ],
            ),
            discord_interaction(
                "jvozba",
                vec![discord_string_option("parts", "klama bajra")],
            ),
            discord_interaction("gimfi'i", vec![discord_string_option("sources", "en:go")]),
        ] {
            let response = post_signed_discord(app.clone(), interaction, &key).await;
            assert_eq!(response.status(), StatusCode::OK);
            let json = response_json(response).await;
            assert_eq!(json["type"], 5);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn discord_command_registration_payload_matches_public_script_shape() {
        let registration = discord::discord_command_registration();
        assert_eq!(registration.payload["name"], "jbotci");
        let subcommands = registration.payload["options"]
            .as_array()
            .expect("subcommand array")
            .iter()
            .map(|option| option["name"].as_str().expect("subcommand name"))
            .collect::<Vec<_>>();
        assert_eq!(
            subcommands,
            vec!["gentufa", "vlasei", "vlacku", "cukta", "jvozba", "gimfi'i"]
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn embedding_assets_return_404_without_static_dir() {
        let app = router(test_config(test_static_dir()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/embeddings/web/v1/catalog.json")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn embedding_assets_serve_catalog_and_vectors_from_static_dir() {
        let static_dir = test_static_dir();
        std::fs::create_dir_all(static_dir.join(
            "assets/embeddings/web/v1/models/test/spaces/space/packs/pack/corpora/vlacku-en",
        ))
        .expect("create embedding asset dir");
        std::fs::write(
            static_dir.join("assets/embeddings/web/v1/catalog.json"),
            "{\"schema_version\":1,\"models\":[]}\n",
        )
        .expect("write catalog");
        std::fs::write(
            static_dir
                .join("assets/embeddings/web/v1/models/test/spaces/space/packs/pack/corpora/vlacku-en/vectors.f32"),
            [0u8, 0, 0, 0],
        )
        .expect("write vector file");
        let app = router(test_config(static_dir));
        let catalog = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/assets/embeddings/web/v1/catalog.json")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("catalog response");
        assert_eq!(catalog.status(), StatusCode::OK);
        assert_eq!(
            catalog
                .headers()
                .get(CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("no-cache")
        );
        assert_eq!(
            response_text(catalog).await,
            "{\"schema_version\":1,\"models\":[]}\n"
        );

        let shard = app
            .oneshot(
                Request::builder()
                    .uri("/assets/embeddings/web/v1/models/test/spaces/space/packs/pack/corpora/vlacku-en/vectors.f32")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("shard response");
        assert_eq!(shard.status(), StatusCode::OK);
        assert_eq!(
            shard
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("application/octet-stream")
        );
        assert_eq!(response_bytes(shard).await, vec![0, 0, 0, 0]);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn static_assets_prefer_brotli_when_accepted() {
        let static_dir = test_static_dir();
        std::fs::create_dir_all(static_dir.join("assets")).expect("create assets dir");
        std::fs::write(static_dir.join("assets/app.js"), "plain").expect("write asset");
        std::fs::write(static_dir.join("assets/app.js.br"), "brotli").expect("write br asset");
        let app = router(test_config(static_dir));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/app.js")
                    .header(ACCEPT_ENCODING, "gzip, br")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(CONTENT_ENCODING)
                .and_then(|value| value.to_str().ok()),
            Some("br")
        );
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/javascript; charset=utf-8")
        );
        assert_eq!(response_text(response).await, "brotli");
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn static_assets_skip_brotli_without_accept_encoding() {
        let static_dir = test_static_dir();
        std::fs::create_dir_all(static_dir.join("assets")).expect("create assets dir");
        std::fs::write(static_dir.join("assets/app.js"), "plain").expect("write asset");
        std::fs::write(static_dir.join("assets/app.js.br"), "brotli").expect("write br asset");
        let app = router(test_config(static_dir));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/app.js")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().get(CONTENT_ENCODING).is_none());
        assert_eq!(response_text(response).await, "plain");
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn missing_static_asset_returns_404() {
        let app = router(test_config(test_static_dir()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/missing.js")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn embedding_assets_reject_path_traversal() {
        let static_dir = test_static_dir();
        let secret_path = static_dir
            .parent()
            .expect("test root has parent")
            .join("secret.txt");
        std::fs::write(&secret_path, "secret").expect("write secret");
        let app = router(test_config(static_dir));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/embeddings/../secret.txt")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn root_redirects_to_dictionary_route() {
        let app = router(test_config(test_static_dir()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok()),
            Some("/jbotci/vlacku"),
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn root_base_redirects_to_unprefixed_dictionary_route() {
        let app = router(test_config_with_base_path("/", test_static_dir()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok()),
            Some("/vlacku"),
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn root_favicon_serves_v0_png_icon() {
        let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/jbotci-ui");
        let app = router(test_config(static_dir));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/favicon.ico")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("image/png"),
        );
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn gentufa_export_routes_render_png_and_svg() {
        let app = router(test_config(test_static_dir()));
        let png = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/jbotci/gentufa.png?text=mi+klama")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("PNG response");
        assert_eq!(png.status(), StatusCode::OK);
        assert_eq!(
            png.headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("image/png")
        );
        let png_bytes = response_bytes(png).await;
        assert!(png_bytes.starts_with(b"\x89PNG\r\n\x1a\n"));

        let svg = app
            .oneshot(
                Request::builder()
                    .uri("/jbotci/gentufa.svg?text=mi+klama&script=cyrillic")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("SVG response");
        assert_eq!(svg.status(), StatusCode::OK);
        assert_eq!(
            svg.headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("image/svg+xml; charset=utf-8")
        );
        let svg_body = response_text(svg).await;
        assert!(svg_body.contains("<svg"));
        assert!(svg_body.contains("ми"));
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn gentufa_export_route_returns_bad_request_for_parse_error() {
        let app = router(test_config(test_static_dir()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/jbotci/gentufa.png?text=perhaps")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response_text(response).await;
        assert!(body.contains("morphology.invalid-apostrophe"));
        assert!(body.contains("\nreason:"));
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn spa_gentufa_metadata_is_rendered_with_png_social_image() {
        let app = router(test_config(test_static_dir()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/jbotci/gentufa?text=mi+klama")
                    .header(HOST, "example.test")
                    .header("x-forwarded-proto", "https")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_text(response).await;

        assert!(body.contains("<title>mi klama - jbotci gentufa</title>"));
        assert!(body.contains("name=\"description\""));
        assert!(
            body.contains("<link rel=\"manifest\" href=\"/jbotci/assets/manifest.webmanifest\">")
        );
        assert!(body.contains(
            "<link rel=\"apple-touch-icon\" href=\"/jbotci/assets/icons/apple-touch-icon.png\">"
        ));
        assert!(!body.contains("Parse succeeded:"));
        assert!(body.contains(
            "property=\"og:url\" content=\"https://example.test/jbotci/gentufa?text=mi+klama\""
        ));
        assert!(body.contains("name=\"twitter:card\" content=\"summary_large_image\""));
        assert!(body.contains(
            "property=\"og:image\" content=\"https://example.test/jbotci/gentufa.png?text=mi+klama\""
        ));
        assert!(body.contains(
            "name=\"twitter:image\" content=\"https://example.test/jbotci/gentufa.png?text=mi+klama\""
        ));
        assert!(!body.contains("gentufa.svg"));
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn spa_cukta_and_vlacku_metadata_include_canonical_social_tags() {
        let app = router(test_config(test_static_dir()));
        let cukta = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/jbotci/cukta/index")
                    .header(HOST, "example.test")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let cukta_body = response_text(cukta).await;
        assert!(cukta_body.contains("<title>jbotci CLL - CLL index</title>"));
        assert!(
            cukta_body
                .contains("Browse indexed CLL terms and jump directly into the embedded book.")
        );
        assert!(
            cukta_body
                .contains("property=\"og:url\" content=\"http://example.test/jbotci/cukta/index\"")
        );
        assert!(cukta_body.contains("name=\"twitter:title\" content=\"jbotci CLL - CLL index\""));

        let vlacku = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/jbotci/vlacku/klama")
                    .header(HOST, "example.test")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let vlacku_body = response_text(vlacku).await;
        assert!(vlacku_body.contains("<title>klama - jbotci vlacku</title>"));
        assert!(vlacku_body.contains("comes/goes"));
        assert!(
            vlacku_body.contains(
                "property=\"og:url\" content=\"http://example.test/jbotci/vlacku/klama\""
            )
        );

        let gimfihi = app
            .oneshot(
                Request::builder()
                    .uri("/jbotci/gimfihi?preset=1995&source=cmn%3A%3Auan&source=hin%3A%3Arakan&source=eng%3A%3Aekspekt&source=spa%3A%3Aesper&source=rus%3A%3Apredpologa&source=ara%3A%3Amulud&shape=ccvcv&shape=cvccv&letters=source&check-collisions=none&require-free-short-rafsi=false&count=5&highlight=nanpe")
                    .header(HOST, "example.test")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let gimfihi_body = response_text(gimfihi).await;
        assert!(gimfihi_body.contains("<title>nanpe - jbotci gimfi"));
        assert!(gimfihi_body.contains("nanpe = cmn:uan ×347"));
        assert!(
            gimfihi_body
                .contains("property=\"og:url\" content=\"http://example.test/jbotci/gimfihi?")
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn spa_unknown_route_uses_default_dictionary_metadata() {
        let app = router(test_config(test_static_dir()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/jbotci/unknown-route")
                    .header(HOST, "example.test")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_text(response).await;
        assert!(body.contains("<title>jbotci vlacku</title>"));
        assert!(body.contains("property=\"og:url\" content=\"http://example.test/jbotci/vlacku\""));
    }
}
