use std::fmt::Debug;
use std::sync::Arc;

use crate::asynchronous::transport::AsyncTransport;
use crate::error::Result;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::StreamExt;
use http::HeaderMap;
use native_tls::TlsConnector;
use tokio::sync::RwLock;
use tokio_tungstenite::connect_async_tls_with_config;
use tokio_tungstenite::Connector;
use url::Url;

use super::websocket_general::AsyncWebsocketGeneralTransport;

/// An asynchronous websocket transport type.
/// This type only allows for secure websocket
/// connections ("wss://").
pub struct WebsocketSecureTransport {
    inner: AsyncWebsocketGeneralTransport,
    base_url: Arc<RwLock<Url>>,
}

impl WebsocketSecureTransport {
    /// Creates a new instance over a request that might hold additional headers, a possible
    /// Tls connector and an URL.
    pub(crate) async fn new(
        base_url: Url,
        tls_config: Option<TlsConnector>,
        headers: Option<HeaderMap>,
    ) -> Result<Self> {
        let mut url = base_url;
        url.query_pairs_mut().append_pair("transport", "websocket");
        url.set_scheme("wss").unwrap();

        let mut req = http::Request::builder().uri(url.clone().as_str());
        if let Some(map) = headers {
            // SAFETY: this unwrap never panics as the underlying request is just initialized and in proper state
            req.headers_mut().unwrap().extend(map);
        }

        let (ws_stream, _) = connect_async_tls_with_config(
            req.body(())?,
            None,
            tls_config.map(Connector::NativeTls),
        )
        .await?;

        let (sen, rec) = ws_stream.split();
        let inner = AsyncWebsocketGeneralTransport::new(sen, rec).await;

        Ok(WebsocketSecureTransport {
            inner,
            base_url: Arc::new(RwLock::new(url)),
        })
    }

    /// Sends probe packet to ensure connection is valid, then sends upgrade
    /// request
    pub(crate) async fn upgrade(&self) -> Result<()> {
        self.inner.upgrade().await
    }
}

#[async_trait]
impl AsyncTransport for WebsocketSecureTransport {
    async fn emit(&self, data: Bytes, is_binary_att: bool) -> Result<()> {
        self.inner.emit(data, is_binary_att).await
    }

    async fn poll(&self) -> Result<Bytes> {
        self.inner.poll().await
    }

    async fn base_url(&self) -> Result<Url> {
        Ok(self.base_url.read().await.clone())
    }

    async fn set_base_url(&self, base_url: Url) -> Result<()> {
        let mut url = base_url;
        if !url
            .query_pairs()
            .any(|(k, v)| k == "transport" && v == "websocket")
        {
            url.query_pairs_mut().append_pair("transport", "websocket");
        }
        url.set_scheme("wss").unwrap();
        *self.base_url.write().await = url;
        Ok(())
    }
}

impl Debug for WebsocketSecureTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncWebsocketSecureTransport")
            .field("base_url", &self.base_url)
            .finish()
    }
}
