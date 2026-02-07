use actix_web::HttpResponse;
use regex::Regex;
use std::fmt::Display;
use yew::html::BaseComponent;
use yew::ServerRenderer;

/// Returns the web facing path to a resource in your application
/// Will be prefixed with the app_root
pub fn app_path(src: impl Into<String>) -> String {
    let src: String = src.into();
    if let Some(tail) = src.strip_prefix("/") {
        let root = crate::app_root();
        return format!("{root}{tail}");
    }
    src
}

/// Returns the web facing path to a resource in your application
/// Will be prefixed with the app_root
/// takes two args useful when creating a path with an Id
/// output: `/app_root/src/tail`
pub fn app_path2(src: impl Into<String>, tail: impl Display) -> String {
    let src = app_path(src);
    format!("{src}/{tail}")
}

/// Render a Yew view to send out in an Actix Response
pub async fn render<V, VM, E>(args: VM) -> Result<HttpResponse, E>
where
    V: BaseComponent,
    V: BaseComponent<Properties = VM>,
    VM: Send + 'static,
{
    let renderer = ServerRenderer::<V>::with_props(|| args);
    let html = renderer.render().await;
    // add the doctype markup. Yew doesn't like to render this.
    let html = format!("<!DOCTYPE html>\n{html}");
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

/// Render a Yew view to send out in an Actix Response
/// Strips out HTML comments
pub async fn render_min<V, VM, E>(args: VM) -> Result<HttpResponse, E>
where
    V: BaseComponent,
    V: BaseComponent<Properties = VM>,
    VM: Send + 'static,
{
    let renderer = ServerRenderer::<V>::with_props(|| args);
    let html = renderer.render().await;
    let html = strip_html_comments(&html);
    // add the doctype markup. Yew doesn't like to render this.
    let html = format!("<!DOCTYPE html>\n{html}");
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html))
}

/// Render a Yew view to send out in an Actix Response for a Turbo Stream
pub async fn render_turbo_stream<V, VM, E>(args: VM) -> Result<HttpResponse, E>
where
    V: BaseComponent,
    V: BaseComponent<Properties = VM>,
    VM: Send + 'static,
{
    let renderer = ServerRenderer::<V>::with_props(|| args);
    let html = renderer.render().await;
    let html = strip_html_comments(&html);
    Ok(HttpResponse::Ok()
        .content_type("text/vnd.turbo-stream.html")
        .body(html))
}

/// Render a Yew view to send out in an Actix Response for a Turbo Stream
/// return the full contents from YEW without stripping comments
pub async fn render_turbo_stream_full<V, VM, E>(args: VM) -> Result<HttpResponse, E>
where
    V: BaseComponent,
    V: BaseComponent<Properties = VM>,
    VM: Send + 'static,
{
    let renderer = ServerRenderer::<V>::with_props(|| args);
    let html = renderer.render().await;
    Ok(HttpResponse::Ok()
        .content_type("text/vnd.turbo-stream.html")
        .body(html))
}

/// Render a Yew view to send out in an Actix Response
/// Used when a form is not valid
pub fn redirect<E>(path: impl Into<String>) -> Result<HttpResponse, E> {
    let path: String = app_path(path);
    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", path))
        .finish())
}

/// Simple little function to remove HTML comments for the YEW render
fn strip_html_comments(input: &str) -> String {
    // (?s) enables "dot matches newline"
    let re = Regex::new(r"(?s)<!--.*?-->").unwrap();
    re.replace_all(input, "").into_owned()
}
