use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use super::AppState;

/// Loopback binding alone does not prevent a browser from addressing this
/// server through an unrelated hostname or originating from another site.
pub(super) async fn local_request(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let port = state.session.identity().port;
    if !allowed(request.headers(), port) {
        return (
            StatusCode::FORBIDDEN,
            format!("this server only accepts requests from its local review page\nOpen http://127.0.0.1:{port} to read the review."),
        ).into_response();
    }
    next.run(request).await
}

fn allowed(headers: &HeaderMap, port: u16) -> bool {
    let Some(host) = single(headers, "host") else {
        return false;
    };
    if host != format!("127.0.0.1:{port}") && host != format!("localhost:{port}") {
        return false;
    }
    if headers.contains_key(header::ORIGIN)
        && single(headers, "origin") != Some(format!("http://{host}").as_str())
    {
        return false;
    }
    // CLI clients have no browser origin. Fetch metadata distinguishes them
    // from cross-site resource requests, which may also omit Origin.
    if headers.contains_key("sec-fetch-site")
        && !matches!(
            single(headers, "sec-fetch-site"),
            Some("same-origin" | "none")
        )
    {
        return false;
    }
    true
}

fn single<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?.to_str().ok()?;
    if values.next().is_some() {
        return None;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_listening_address_and_port_are_accepted() {
        for host in ["127.0.0.1:4600", "localhost:4600"] {
            assert!(allowed(&headers(&[("host", host)]), 4600));
        }
        for host in [
            "localhost",
            "127.0.0.1",
            "localhost:4601",
            "127.0.0.1:4601",
            "example.test:4600",
            "localhost.example.test:4600",
            "127.0.0.1.example.test:4600",
            "localhost:4600@example.test",
            "localhost:4600,example.test",
        ] {
            assert!(!allowed(&headers(&[("host", host)]), 4600), "{host}");
        }
        assert!(!allowed(&HeaderMap::new(), 4600));
    }

    #[test]
    fn a_browser_origin_must_match_the_requested_address_exactly() {
        for host in ["127.0.0.1:4600", "localhost:4600"] {
            assert!(allowed(
                &headers(&[("host", host), ("origin", &format!("http://{host}"))]),
                4600
            ));
        }
        for origin in [
            "null",
            "https://127.0.0.1:4600",
            "http://127.0.0.1:4601",
            "http://localhost:4600",
            "http://example.test",
            "http://127.0.0.1:4600.example.test",
            "http://127.0.0.1:4600/",
            "http://127.0.0.1:4600 http://example.test",
        ] {
            assert!(
                !allowed(
                    &headers(&[("host", "127.0.0.1:4600"), ("origin", origin)]),
                    4600
                ),
                "{origin}"
            );
        }
    }

    #[test]
    fn cross_site_browser_requests_are_refused_even_without_an_origin() {
        for site in ["cross-site", "same-site", "unexpected"] {
            assert!(!allowed(
                &headers(&[("host", "127.0.0.1:4600"), ("sec-fetch-site", site)]),
                4600
            ));
        }
        for site in ["same-origin", "none"] {
            assert!(allowed(
                &headers(&[("host", "127.0.0.1:4600"), ("sec-fetch-site", site)]),
                4600
            ));
        }
    }

    #[test]
    fn repeated_security_headers_are_not_ambiguous_authorization() {
        for name in ["host", "origin", "sec-fetch-site"] {
            let mut headers = headers(&[
                ("host", "127.0.0.1:4600"),
                ("origin", "http://127.0.0.1:4600"),
                ("sec-fetch-site", "same-origin"),
            ]);
            let value = headers.get(name).unwrap().clone();
            headers.append(
                axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                value,
            );
            assert!(!allowed(&headers, 4600), "{name}");
        }
    }

    fn headers(values: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in values {
            headers.insert(
                axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                value.parse().unwrap(),
            );
        }
        headers
    }
}
