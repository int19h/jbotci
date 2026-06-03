//! HTTP server for the jbotci web app and API integrations.

use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
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
use bityzba::{ensures, invariant, requires};
use dioxus::server::{DioxusRouterExt, FullstackState};
use jbotci_web_core::{
    FAVICON_ASSET_PATH, GentufaError, GentufaWebRequest, GentufaWebResult, MANIFEST_ASSET_PATH,
    META_BLOCK_END, META_BLOCK_START, PageMeta, WebFeatureAvailability, WebRoute, build_page_meta,
    parse_gentufa_for_web, parse_web_route, render_page_head_metadata_block,
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
struct AppState {
    base_path: String,
    public_dir: PathBuf,
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
    });
    let use_dioxus_static_assets = dioxus_public_dir()
        .as_ref()
        .is_some_and(|public_dir| public_dir.is_dir() && public_dir == &state.public_dir);
    let router = Router::<FullstackState>::new()
        .route("/api/health", get(health))
        .route("/api/features", get(features))
        .route("/api/gentufa", post(gentufa))
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
        let location = gentufa_location(&state.base_path);
        return redirect_response(&location);
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
#[ensures(true)]
fn is_spa_route_path(path: &str) -> bool {
    path == "/gentufa"
        || path.starts_with("/gentufa/")
        || path == "/vlacku"
        || path.starts_with("/vlacku/")
        || path == "/cukta"
        || path.starts_with("/cukta/")
        || path == "/settings"
        || path.starts_with("/settings/")
}

#[requires(path.starts_with('/'))]
#[requires(base_path.starts_with('/'))]
#[ensures(true)]
fn is_api_request_path(path: &str, base_path: &str) -> bool {
    if path.starts_with("/api/") {
        return true;
    }
    strip_base_path(path, base_path).is_some_and(|stripped| stripped.starts_with("/api/"))
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
fn gentufa_location(base_path: &str) -> String {
    if base_path == "/" {
        "/gentufa".to_owned()
    } else {
        format!("{base_path}/gentufa")
    }
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
    use tower::ServiceExt;

    #[requires(true)]
    #[ensures(ret.is_dir())]
    fn test_static_dir() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "jbotci-server-spa-test-{}-{nanos}",
            std::process::id()
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
    fn root_absolute_assets_work_with_non_root_base_path() {
        assert_eq!(
            asset_path_for_request("/assets/app.js", "/jbotci").as_deref(),
            Some("/assets/app.js")
        );
        assert_eq!(
            asset_path_for_request("/jbotci/manifest.webmanifest", "/jbotci").as_deref(),
            Some("/manifest.webmanifest")
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
        assert_eq!(cache_control_for_path("/manifest.webmanifest"), "no-cache");
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
    async fn root_redirects_to_gentufa_route() {
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
            Some("/jbotci/gentufa"),
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn root_base_redirects_to_unprefixed_gentufa_route() {
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
            Some("/gentufa"),
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn root_favicon_serves_v0_png_icon() {
        let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../jbotci-web");
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
    async fn spa_gentufa_metadata_is_rendered_without_social_image() {
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
        assert!(body.contains("<link rel=\"manifest\" href=\"/jbotci/manifest.webmanifest\">"));
        assert!(body.contains(
            "<link rel=\"apple-touch-icon\" href=\"/jbotci/assets/icons/apple-touch-icon.png\">"
        ));
        assert!(body.contains("Parse succeeded:"));
        assert!(body.contains(
            "property=\"og:url\" content=\"https://example.test/jbotci/gentufa?text=mi+klama\""
        ));
        assert!(body.contains("name=\"twitter:card\" content=\"summary\""));
        assert!(!body.contains("property=\"og:image\""));
        assert!(!body.contains("name=\"twitter:image\""));
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
        assert!(vlacku_body.contains("Dictionary lookup for “klama”."));
        assert!(
            vlacku_body.contains(
                "property=\"og:url\" content=\"http://example.test/jbotci/vlacku/klama\""
            )
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn spa_unknown_route_uses_default_gentufa_metadata() {
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
        assert!(body.contains("<title>jbotci gentufa</title>"));
        assert!(
            body.contains("property=\"og:url\" content=\"http://example.test/jbotci/gentufa\"")
        );
    }
}
