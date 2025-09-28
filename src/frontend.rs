use axum::{
    body::Body,
    extract::Path,
    http::header,
    response::{IntoResponse, Response},
};
use mime_guess::from_path;
use rust_embed::Embed;
use std::hash::{Hash, Hasher};

#[derive(Embed)]
#[folder = "frontend/dist/"]
struct Frontend;

#[tracing::instrument(level = "trace")]
pub async fn serve_frontend(
    path: Option<Path<String>>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let path = path.unwrap_or(Path("".to_string()));
    let path = path.as_str();

    let default_path = "index.html";

    let path_candidate = if path.is_empty() { default_path } else { &path };

    let actual_path_and_content = Frontend::get(path_candidate)
        .map(|content| (path_candidate, content))
        .or_else(|| Frontend::get(default_path).map(|content| (default_path, content)));

    if let Some((actual_path, content)) = actual_path_and_content {
        let body = Body::from(content.data.into_owned());
        let mime = from_path(actual_path).first_or_octet_stream();
        tracing::debug!("Serving {} as {}", actual_path, path);
        let response = Response::builder()
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(body);
        match response {
            Ok(response) => Ok(response),
            Err(_) => Err(crate::axum_util::not_found().await),
        }
    } else {
        tracing::debug!("Failed to serve {}", path);
        Err(crate::axum_util::not_found().await)
    }
}

pub fn frontend_hash() -> String {
    let mut files: Vec<_> = Frontend::iter().collect();
    files.sort();
    let shas: Vec<_> = files
        .iter()
        .map(|f| {
            Frontend::get(f)
                .expect("File expected")
                .metadata
                .sha256_hash()
        })
        .collect();

    let mut hasher = std::hash::DefaultHasher::new();
    shas.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
