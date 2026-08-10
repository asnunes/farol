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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_browser_is_told_what_each_kind_of_file_is() {
        // Serving JavaScript as octet-stream makes the browser refuse to run
        // it, and the screen comes up blank with nothing in the log.
        assert_eq!(mime_for("index.html"), "text/html; charset=utf-8");
        assert_eq!(mime_for("assets/app.js"), "text/javascript; charset=utf-8");
        assert_eq!(mime_for("assets/app.mjs"), "text/javascript; charset=utf-8");
        assert_eq!(mime_for("assets/app.css"), "text/css; charset=utf-8");
        assert_eq!(mime_for("data.json"), "application/json");
        assert_eq!(mime_for("icon.svg"), "image/svg+xml");
        assert_eq!(mime_for("font.woff2"), "font/woff2");
    }

    #[test]
    fn anything_unrecognised_is_handed_over_as_bytes() {
        assert_eq!(mime_for("favicon.png"), "application/octet-stream");
        assert_eq!(mime_for("LICENSE"), "application/octet-stream");
    }

    #[test]
    fn the_extension_is_the_last_one_not_the_first() {
        // `app.min.css` is a stylesheet, not something called `min.css`.
        assert_eq!(mime_for("app.min.css"), "text/css; charset=utf-8");
    }

    #[tokio::test]
    async fn a_request_is_answered_either_way() {
        // Whether the page is served or the reviewer is told to build it
        // depends on the state of `web/dist`, which a concurrent `vite build`
        // changes underneath — asserting on which branch ran made this flake
        // three times. The invariant that holds regardless: an answer, and if
        // there is nothing to serve, one that says what to run. The page being
        // served is covered in `tests/server.rs`, against a real process whose
        // build state is settled.
        let res = handler(Uri::from_static("/blocks/core")).await;

        match res.status() {
            StatusCode::OK => {}
            StatusCode::NOT_FOUND => {
                let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
                    .await
                    .unwrap();
                assert!(
                    String::from_utf8_lossy(&bytes).contains("just build"),
                    "an unbuilt frontend has to say what to run"
                );
            }
            other => panic!("unexpected status {other}"),
        }
    }
}
