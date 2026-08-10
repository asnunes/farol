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

    async fn body_of(res: Response) -> String {
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    #[tokio::test]
    async fn an_unknown_path_falls_back_to_the_page_or_says_it_is_not_built() {
        // Both are correct, and which one depends on whether `just build` has
        // run — so assert the pair rather than the state of this checkout.
        let res = handler(Uri::from_static("/blocks/core")).await;

        if Assets::get("index.html").is_some() {
            assert_eq!(
                res.status(),
                StatusCode::OK,
                "a deep link is the screen's own routing, not a missing file"
            );
            assert!(body_of(res).await.contains("<"), "it should be the page");
        } else {
            assert_eq!(res.status(), StatusCode::NOT_FOUND);
            assert!(
                body_of(res).await.contains("just build"),
                "the message has to say what to run"
            );
        }
    }

    #[tokio::test]
    async fn the_root_serves_the_page_itself() {
        let res = handler(Uri::from_static("/")).await;

        if Assets::get("index.html").is_some() {
            assert_eq!(res.status(), StatusCode::OK);
            let kind = res
                .headers()
                .get(header::CONTENT_TYPE)
                .map(|v| v.to_str().unwrap().to_string());
            assert_eq!(kind.as_deref(), Some("text/html; charset=utf-8"));
        } else {
            assert_eq!(res.status(), StatusCode::NOT_FOUND);
        }
    }
}
