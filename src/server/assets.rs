use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};

/// In a release build the whole frontend rides inside the binary, so shipping
/// farol is copying one file. In debug it is proxied from Vite instead, because
/// developing the UI without hot reload is not worth the purity.
#[derive(rust_embed::Embed)]
#[folder = "web/dist"]
struct Assets;

pub async fn handler(uri: Uri) -> Response {
    #[cfg(debug_assertions)]
    {
        if let Some(res) = proxy_to_vite(&uri).await {
            return res;
        }
    }

    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match Assets::get(path).or_else(|| Assets::get("index.html")) {
        Some(file) => {
            let mime = mime_for(path);
            ([(header::CONTENT_TYPE, mime)], file.data.into_owned()).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            "The frontend is not built. Run `just build` (or `npm run build` in web/).",
        )
            .into_response(),
    }
}

#[cfg(debug_assertions)]
async fn proxy_to_vite(uri: &Uri) -> Option<Response> {
    let target = format!("http://127.0.0.1:5173{}", uri.path());
    let body = tokio::process::Command::new("curl")
        .args(["-sf", &target])
        .output()
        .await
        .ok()?;
    if !body.status.success() {
        return None;
    }
    let mime = mime_for(uri.path());
    Some(([(header::CONTENT_TYPE, mime)], body.stdout).into_response())
}

fn mime_for(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}
