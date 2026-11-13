//! The web client's files.
//!
//! Two roots:
//!
//! - the web root: the Vite build of `web/`.
//! - the assets root: art, CSS, fonts, scripts and documents under `/art`, `/css`, `/fonts`,
//!   `/js`, `/docs`, the fonts at the top level and the icon.
//!
//! A path that names no file and has no extension in its last segment gets `index.html`, so
//! client routes such as `/scene/alliance/Atreides` load the app. TLS, redirects and HSTS are
//! the reverse proxy's job.

use std::path::{Component, Path, PathBuf};

use axum::body::Body;
use axum::http::{header, Method, StatusCode, Uri};
use axum::response::Response;

#[derive(Clone, Debug)]
pub struct StaticRoots {
    pub web: PathBuf,
    pub shared: PathBuf,
}

/// Whether a path is one of the shared assets.
fn is_shared(path: &str) -> bool {
    ["/art/", "/css/", "/fonts/", "/js/", "/docs/"].iter().any(|p| path.starts_with(p))
        || path == "/favicon.ico"
        || (path.ends_with(".ttf") && path[1..].find('/').is_none())
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("ttf") => "font/ttf",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("mp3") => "audio/mpeg",
        Some("pdf") => "application/pdf",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// `%XX` decoding of a URL path. `None` for a malformed escape or invalid UTF-8.
fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// The file under `root` for a URL path, refusing anything that leaves `root`.
fn resolve(root: &Path, path: &str) -> Option<PathBuf> {
    let relative = Path::new(path.trim_start_matches('/'));
    if relative.components().any(|c| !matches!(c, Component::Normal(_))) {
        return None;
    }
    Some(root.join(relative))
}

async fn file(path: &Path, head: bool) -> Option<Response> {
    let meta = tokio::fs::metadata(path).await.ok()?;
    if !meta.is_file() {
        return None;
    }
    let body = if head { Body::empty() } else { Body::from(tokio::fs::read(path).await.ok()?) };
    Response::builder()
        .header(header::CONTENT_TYPE, content_type(path))
        .header(header::CONTENT_LENGTH, meta.len())
        .body(body)
        .ok()
}

fn status(code: StatusCode) -> Response {
    Response::builder().status(code).body(Body::empty()).expect("a bare status response")
}

/// The fallback route.
pub async fn serve(roots: &StaticRoots, method: &Method, uri: &Uri) -> Response {
    if method != Method::GET && method != Method::HEAD {
        return status(StatusCode::METHOD_NOT_ALLOWED);
    }
    let head = method == Method::HEAD;
    let Some(path) = percent_decode(uri.path()) else { return status(StatusCode::BAD_REQUEST) };
    let root = if is_shared(&path) { &roots.shared } else { &roots.web };
    if let Some(p) = resolve(root, &path) {
        if let Some(r) = file(&p, head).await {
            return r;
        }
    }
    let last = path.rsplit('/').next().unwrap_or_default();
    if !last.contains('.') {
        if let Some(r) = file(&roots.web.join("index.html"), head).await {
            return r;
        }
    }
    status(StatusCode::NOT_FOUND)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths() {
        assert!(is_shared("/css/treachery.css"));
        assert!(is_shared("/Advokat.ttf"));
        assert!(!is_shared("/assets/index.js"));
        assert!(!is_shared("/scene/a.ttf"));
        let root = Path::new("/r");
        assert_eq!(resolve(root, "/a/b.png"), Some(PathBuf::from("/r/a/b.png")));
        assert_eq!(resolve(root, "/a/../../etc/passwd"), None);
        assert_eq!(percent_decode("/a%20b"), Some("/a b".into()));
        assert_eq!(percent_decode("/a%2"), None);
    }
}
