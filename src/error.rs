use askama::Template;
use askama_axum::IntoResponse;
use axum::http::StatusCode;

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    message: String,
}

pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> askama_axum::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorTemplate {
                message: self.0.to_string(),
            }
            .into_response(),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
