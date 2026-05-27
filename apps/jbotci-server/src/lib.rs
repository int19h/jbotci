//! HTTP server for the jbotci web app and API integrations.

use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::body::Body;
use axum::extract::State;
use axum::http::header::{
    ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_TYPE, HeaderMap, HeaderValue,
    LOCATION,
};
use axum::http::{Response, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use clap::Parser;
use jbotci_web_core::{
    GentufaWebRequest, GentufaWebResult, WebFeatureAvailability, parse_gentufa_for_web,
};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(name = "jbotci-server")]
#[command(about = "Server application for jbotci web and HTTP integrations")]
#[invariant(true)]
pub struct Cli {
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    #[arg(long, default_value_t = 8080)]
    pub port: u16,
    #[arg(long, default_value = "/jbotci")]
    pub base_path: String,
    #[arg(long)]
    pub static_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
#[invariant(true)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub base_path: String,
    pub static_dir: Option<PathBuf>,
}

impl From<Cli> for ServerConfig {
    #[requires(true)]
    #[ensures(true)]
    fn from(cli: Cli) -> Self {
        Self {
            host: cli.host,
            port: cli.port,
            base_path: normalize_base_path(&cli.base_path),
            static_dir: cli.static_dir,
        }
    }
}

#[derive(Debug, Clone)]
#[invariant(true)]
struct AppState {
    base_path: String,
    static_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
#[invariant(true)]
struct HealthResponse {
    status: &'static str,
    features: WebFeatureAvailability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub struct EmbeddedAsset {
    pub request_path: &'static str,
    pub content_encoding: Option<&'static str>,
    pub bytes: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/embedded_assets.rs"));

#[requires(true)]
#[ensures(ret.base_path.starts_with('/'))]
pub fn config_from_cli() -> ServerConfig {
    Cli::parse().into()
}

#[requires(!config.host.is_empty())]
#[requires(config.base_path.starts_with('/'))]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub async fn run_server(config: ServerConfig) -> Result<()> {
    let address: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .with_context(|| format!("invalid listen address `{}:{}`", config.host, config.port))?;
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("failed to bind `{address}`"))?;
    axum::serve(listener, router(config))
        .await
        .context("server failed")?;
    Ok(())
}

#[requires(config.base_path.starts_with('/'))]
#[ensures(true)]
pub fn router(config: ServerConfig) -> Router {
    let state = Arc::new(AppState {
        base_path: normalize_base_path(&config.base_path),
        static_dir: config.static_dir,
    });
    Router::new()
        .route("/api/health", get(health))
        .route("/api/features", get(features))
        .route("/api/gentufa", post(gentufa))
        .fallback(static_or_spa)
        .with_state(state)
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
    Json(parse_gentufa_for_web(&request))
}

#[requires(true)]
#[ensures(true)]
async fn static_or_spa(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    uri: Uri,
) -> Response<Body> {
    let request_path = uri.path();
    if request_path == "/favicon.ico" {
        return Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Body::empty())
            .expect("favicon response builder is valid");
    }
    if request_path.starts_with("/api/") {
        return plain_response(StatusCode::NOT_FOUND, "not found");
    }
    if request_path == "/" || (state.base_path != "/" && request_path == state.base_path) {
        let location = gentufa_location(&state.base_path);
        return redirect_response(&location);
    }
    let Some(asset_path) = asset_path_for_request(request_path, &state.base_path) else {
        return plain_response(StatusCode::NOT_FOUND, "not found");
    };
    if let Some(static_dir) = &state.static_dir
        && let Some(response) =
            static_dir_response(static_dir, &asset_path, accepts_brotli(&headers)).await
    {
        return response;
    }
    embedded_asset_response(&asset_path, accepts_brotli(&headers))
        .unwrap_or_else(|| plain_response(StatusCode::NOT_FOUND, "not found"))
}

#[requires(path.starts_with('/'))]
#[requires(base_path.starts_with('/'))]
#[ensures(ret.as_ref().is_none_or(|path| path.starts_with('/')))]
fn asset_path_for_request(path: &str, base_path: &str) -> Option<String> {
    let stripped = strip_base_path(path, base_path)?;
    if stripped == "/" || !has_file_extension(&stripped) {
        return Some("/index.html".to_owned());
    }
    Some(stripped)
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
        let br_path = PathBuf::from(format!("{}.br", normal_path.display()));
        if br_path.is_file() {
            (br_path, asset_path.to_owned(), Some("br"))
        } else {
            (normal_path, asset_path.to_owned(), None)
        }
    } else {
        (normal_path, asset_path.to_owned(), None)
    };
    let bytes = std::fs::read(path).ok()?;
    Some(asset_response(
        StatusCode::OK,
        &logical_path,
        encoding,
        Body::from(bytes),
    ))
}

#[requires(asset_path.starts_with('/'))]
#[ensures(true)]
fn embedded_asset_response(asset_path: &str, accepts_brotli: bool) -> Option<Response<Body>> {
    let asset = select_embedded_asset(EMBEDDED_ASSETS, asset_path, accepts_brotli)?;
    Some(asset_response(
        StatusCode::OK,
        asset.request_path,
        asset.content_encoding,
        Body::from(asset.bytes),
    ))
}

#[requires(path.starts_with('/'))]
#[ensures(true)]
fn select_embedded_asset<'a>(
    assets: &'a [EmbeddedAsset],
    path: &str,
    accepts_brotli: bool,
) -> Option<&'a EmbeddedAsset> {
    if accepts_brotli
        && let Some(asset) = assets
            .iter()
            .find(|asset| asset.request_path == path && asset.content_encoding == Some("br"))
    {
        return Some(asset);
    }
    assets
        .iter()
        .find(|asset| asset.request_path == path && asset.content_encoding.is_none())
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
        _ => "application/octet-stream",
    }
}

#[requires(path.starts_with('/'))]
#[ensures(!ret.is_empty())]
fn cache_control_for_path(path: &str) -> &'static str {
    if path == "/index.html" {
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
    fn embedded_asset_selection_prefers_brotli_when_accepted() {
        static NORMAL: &[u8] = b"normal";
        static BROTLI: &[u8] = b"br";
        let assets = [
            EmbeddedAsset {
                request_path: "/app.js",
                content_encoding: None,
                bytes: NORMAL,
            },
            EmbeddedAsset {
                request_path: "/app.js",
                content_encoding: Some("br"),
                bytes: BROTLI,
            },
        ];
        assert_eq!(
            select_embedded_asset(&assets, "/app.js", false)
                .unwrap()
                .bytes,
            NORMAL
        );
        assert_eq!(
            select_embedded_asset(&assets, "/app.js", true)
                .unwrap()
                .bytes,
            BROTLI
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn gentufa_api_matches_direct_parser() {
        let app = router(ServerConfig {
            host: "127.0.0.1".to_owned(),
            port: 0,
            base_path: "/jbotci".to_owned(),
            static_dir: None,
        });
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
        let app = router(ServerConfig {
            host: "127.0.0.1".to_owned(),
            port: 0,
            base_path: "/jbotci".to_owned(),
            static_dir: None,
        });
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/missing")
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
        let app = router(ServerConfig {
            host: "127.0.0.1".to_owned(),
            port: 0,
            base_path: "/jbotci".to_owned(),
            static_dir: None,
        });
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
}
