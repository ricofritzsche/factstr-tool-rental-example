use axum::{
    http::{HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Response},
};

const INDEX_HTML: &str = include_str!("../../static/index.html");
const APP_JS: &str = include_str!("../../static/app.js");
const STYLES_CSS: &str = include_str!("../../static/styles.css");

pub async fn index_handler() -> Html<&'static str> {
    Html(INDEX_HTML)
}

pub async fn app_js_handler() -> Response {
    static_asset_response(APP_JS, "application/javascript; charset=utf-8")
}

pub async fn styles_css_handler() -> Response {
    static_asset_response(STYLES_CSS, "text/css; charset=utf-8")
}

fn static_asset_response(body: &'static str, content_type: &'static str) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, HeaderValue::from_static(content_type))],
        body,
    )
        .into_response()
}
