use actix_web::HttpResponse;
use yew::html::BaseComponent;
use yew::ServerRenderer;

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

/// Render a Yew view to send out in an Actix Response for a Turbo Stream
pub async fn render_turbo_stream<V, VM, E>(args: VM) -> Result<HttpResponse, E>
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
    let path: String = path.into();
    Ok(HttpResponse::SeeOther()
        .insert_header(("Location", path))
        .finish())
}
