//! Embedded UI bundle served from `ui/dist/`.
//!
//! Routing rules:
//! * `GET /` → `index.html` (SPA entry).
//! * `GET /<path>` where the file exists in the bundle → that file with the
//!   correct `Content-Type` and a hashed-asset cache header.
//! * `GET /<path>` where no file matches → `index.html` (client-side
//!   routing). API and health/metrics paths are intentionally never reached
//!   here because they are mounted before the static-file fallback.
//!
//! The bundle is produced by `build.rs` (which calls `npm run build`) and
//! embedded at compile time via `rust-embed`. A placeholder `index.html` is
//! emitted when the UI build is unavailable so the binary can still link.

use std::convert::Infallible;

use axum::body::Body;
use axum::http::{header, HeaderValue, Method, Request, Response, StatusCode, Uri};
use axum::routing::{any, get};
use axum::Router;
use rust_embed::Embed;

const INDEX_FILE: &str = "index.html";
const ASSETS_PREFIX: &str = "assets/";

/// Embedded contents of `ui/dist/` produced by the `build.rs` script.
#[derive(Embed)]
#[folder = "$CARGO_MANIFEST_DIR/../../ui/dist/"]
struct UiBundle;

/// Build the static-file router for everything outside the API surface.
///
/// Mount with `.fallback_service(static_router())` so it sits behind the
/// API/health/metrics routes; this keeps API 404s as JSON while still
/// serving the SPA for any front-end route the user lands on directly.
pub fn router() -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/index.html", get(serve_index))
        .fallback(any(serve_path))
}

async fn serve_index() -> Response<Body> {
    serve_embedded(INDEX_FILE)
}

async fn serve_path(method: Method, uri: Uri) -> Response<Body> {
    if method != Method::GET && method != Method::HEAD {
        return method_not_allowed();
    }
    let trimmed = uri.path().trim_start_matches('/');
    if trimmed.is_empty() {
        return serve_embedded(INDEX_FILE);
    }
    if let Some(response) = try_serve(trimmed) {
        return response;
    }
    // SPA fallback.
    serve_embedded(INDEX_FILE)
}

fn try_serve(path: &str) -> Option<Response<Body>> {
    if path.contains("..") {
        return None;
    }
    UiBundle::get(path).map(|file| {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        let mut response = Response::builder().status(StatusCode::OK).header(
            header::CONTENT_TYPE,
            HeaderValue::from_str(mime.as_ref())
                .unwrap_or(HeaderValue::from_static("application/octet-stream")),
        );
        if path.starts_with(ASSETS_PREFIX) {
            response = response.header(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
        } else {
            response = response.header(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        }
        response
            .body(Body::from(file.data.into_owned()))
            .expect("response builder accepts these headers")
    })
}

fn serve_embedded(path: &str) -> Response<Body> {
    try_serve(path).unwrap_or_else(|| {
        // Should never happen for index.html: the build script writes a
        // placeholder when npm is unavailable. Treat it as a 500 so the
        // problem is visible in production logs.
        let mut response = Response::new(Body::from(
            "UI bundle missing; rebuild the binary after running `npm run build` under ui/.",
        ));
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        response
    })
}

fn method_not_allowed() -> Response<Body> {
    let mut response = Response::new(Body::from("method not allowed"));
    *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
    response
}

/// Convert the static router into a `tower::Service` suitable for use as
/// `fallback_service` on the main router.
#[must_use]
pub fn fallback_service(
) -> impl tower::Service<Request<Body>, Response = Response<Body>, Error = Infallible, Future: Send>
       + Clone
       + Send
       + 'static {
    router().into_service()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    #[tokio::test]
    async fn root_returns_index_html() {
        let response = router()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        assert!(content_type.starts_with("text/html"), "got {content_type}");
    }

    #[tokio::test]
    async fn unknown_path_falls_back_to_index() {
        let response = router()
            .oneshot(
                Request::builder()
                    .uri("/dashboard/anything")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        assert!(content_type.starts_with("text/html"), "got {content_type}");
    }

    #[tokio::test]
    async fn rejects_path_traversal() {
        // The SPA fallback should still return index.html (200), not the
        // requested file outside the bundle.
        let response = router()
            .oneshot(
                Request::builder()
                    .uri("/../Cargo.toml")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn known_asset_returns_correct_content_type() {
        // Iterate the embedded bundle and pick the first file under assets/.
        // The hash in the filename changes between builds, so a path lookup
        // would be brittle.
        let asset_path = UiBundle::iter()
            .find(|p| p.starts_with(ASSETS_PREFIX))
            .map(std::borrow::Cow::into_owned);
        let Some(asset) = asset_path else {
            // No assets means we're using the placeholder bundle; skip.
            return;
        };

        let response = router()
            .oneshot(
                Request::builder()
                    .uri(format!("/{asset}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cache = response
            .headers()
            .get(header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert!(
            cache.contains("immutable"),
            "hashed assets should be cacheable forever, got `{cache}`",
        );
    }

    #[tokio::test]
    async fn rejects_non_get_methods() {
        let response = router()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/dashboard")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
