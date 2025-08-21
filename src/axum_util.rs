use argon2::{Argon2, PasswordHash, PasswordVerifier, password_hash::Error};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
    middleware::Next,
    response::{Html, IntoResponse, Response},
};
use base64::prelude::{BASE64_STANDARD, Engine};
use headers::{HeaderMap, HeaderValue};

fn verify_password(hash: &str, password: &str) -> Result<bool, Error> {
    let parsed_hash = PasswordHash::new(hash)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

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

pub async fn basic_auth(
    req: Request<Body>,
    next: Next,
    username: String,
    password_argon2: String,
) -> Result<Response, Response> {
    let headers: &HeaderMap = req.headers();

    if let Some(auth_header) = headers.get(header::AUTHORIZATION)
        && let Ok(auth_str) = auth_header.to_str()
        && let Some(b64) = auth_str.strip_prefix("Basic ")
        && let Ok(decoded) = BASE64_STANDARD.decode(b64)
        && let Ok(creds) = String::from_utf8(decoded)
    {
        let mut split = creds.splitn(2, ':');
        if let (Some(user), Some(pass)) = (split.next(), split.next())
            && user == username
            && verify_password(&password_argon2, &pass).expect("Valid hash expected")
        {
            return Ok(next.run(req).await);
        }
    }

    let mut resp = StatusCode::UNAUTHORIZED.into_response();
    resp.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        HeaderValue::from_static("Basic realm=\"Restricted\""),
    );
    Err(resp)
}
