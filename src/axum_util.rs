use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};

pub async fn not_found() -> impl IntoResponse {
    tracing::debug!("Not found");
    (
        StatusCode::NOT_FOUND,
        Html(
            r#"
        <!DOCTYPE html>
        <html lang="de">
        <head>
            <meta charset="UTF-8">
            <title>Not Found</title>
        </head>
        <body>
            <h1>404 - Not found</h1>
        </body>
        </html>
    "#,
        ),
    )
}

pub async fn empty_response() -> impl IntoResponse {
    "".into_response()
}
