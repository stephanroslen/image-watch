use axum::{
    body::Body,
    extract::Path,
    http::header,
    response::{IntoResponse, Response},
};
use mime_guess::from_path;
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "frontend/dist/"]
struct Frontend;

#[tracing::instrument]
pub async fn serve_frontend(
    path: Option<Path<String>>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let path = path.unwrap_or(Path("".to_string()));
    let path = path.as_str();

    let path = if path.is_empty() { "index.html" } else { &path };

    if let Some(content) = Frontend::get(path) {
        let body = Body::from(content.data.into_owned());
        let mime = from_path(path).first_or_octet_stream();
        tracing::debug!("Serving {}", path);
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
