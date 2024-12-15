use crate::errors::Result;
use actix_web::web::Bytes;
use std::sync::Arc;
use tokio::sync::broadcast::{Receiver, Sender};
use yew::html::BaseComponent;
use yew::ServerRenderer;

#[derive(Debug)]
pub struct TurboMessage {
    pub channel: String,
    pub html: String,
}

/// A wrapper around tokio::sync::broadcast
/// It is used to Send out Render Events to anyone who wants them
#[derive(Debug, Clone)]
pub struct TurboStream(Sender<Arc<TurboMessage>>);

impl Default for TurboStream {
    fn default() -> Self {
        let (tx, _rx) = tokio::sync::broadcast::channel(100);
        TurboStream(tx)
    }
}

impl TurboStream {
    /// Render a Yew view Into the TurboStream Pipeline.
    /// Its is expected that the HTML contains a TurboStream message.
    ///
    /// ```html
    /// <turbo-stream action=*** target=*** >
    ///   <template>
    ///   </template>
    /// </turbo-stream>
    /// ```
    /// This message can then be send out using a SSE action
    /// and the helper function `turbo_sse_stream`
    pub async fn render<V, VM>(&self, channel: impl Into<String>, args: VM) -> Result<()>
    where
        V: BaseComponent,
        V: BaseComponent<Properties = VM>,
        VM: Send + 'static,
    {
        let renderer = ServerRenderer::<V>::with_props(|| args);
        let html = renderer.render().await;
        self.stream(channel, html);
        Ok(())
    }

    pub fn stream(&self, channel: impl Into<String>, html: impl Into<String>) {
        let msg = Arc::new(TurboMessage {
            channel: channel.into(),
            html: html.into(),
        });
        match self.0.send(msg) {
            Ok(_) => (),
            Err(err) => log::warn!("TurboStream Error: {:?}", err),
        }
    }

    pub fn watch(&self, channel: impl Into<String>) -> TurboMessageStream {
        TurboMessageStream {
            channel: channel.into(),
            rx: self.0.subscribe(),
        }
    }
}

pub struct TurboMessageStream {
    channel: String,
    rx: Receiver<Arc<TurboMessage>>,
}

impl TurboMessageStream {
    /// return the next TurboMessage for your channel
    pub async fn next(&mut self) -> Option<Arc<TurboMessage>> {
        loop {
            let msg = self.rx.recv().await.ok()?;
            if msg.channel == self.channel {
                return Some(msg);
            }
        }
    }
}

/// The inner logic for a futures::unfold()
/// Used to stream turbo SSE to the frontend
///
/// In your controller action your will need to build a futures::stream
/// to send out turbo changes.
///
/// ```
/// use gumbo_lib::turbo::{TurboStream, turbo_sse_stream};
/// use actix_web::web::Data;
/// use actix_web::HttpResponse;
/// use futures::stream::unfold;
///
/// // actix endpoint
/// pub(crate) async fn stream(turbo: Data<TurboStream>) -> Result<HttpResponse, ()> {
///   let sub = turbo.watch("dogs/create");
///   let body = unfold(sub, turbo_sse_stream);
///   Ok(HttpResponse::Ok().content_type("text/event-stream").streaming(body))
/// }
/// ```
///
pub async fn turbo_sse_stream(
    mut state: TurboMessageStream,
) -> Option<(
    std::result::Result<Bytes, actix_web::Error>,
    TurboMessageStream,
)> {
    let msg = state.next().await?;
    // make sure there are no newlines in the HTML
    let html = &msg.html.replace("\n", " ");
    let msg = format!("data: {}\n\n", html);
    let bytes = Bytes::copy_from_slice(msg.as_bytes());
    Some((Ok::<_, actix_web::Error>(bytes), state))
}
