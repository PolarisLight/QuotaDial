use crate::error::AppError;
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
    sync::{broadcast, oneshot, Mutex},
};

#[derive(Debug, Clone, PartialEq)]
pub struct RpcNotification {
    pub method: String,
    pub params: Value,
}

type PendingSender = oneshot::Sender<Result<Value, AppError>>;

struct RpcPeerInner {
    writer: Mutex<Box<dyn AsyncWrite + Unpin + Send>>,
    pending: Arc<Mutex<HashMap<u64, PendingSender>>>,
    next_id: AtomicU64,
    notifications: broadcast::Sender<RpcNotification>,
}

#[derive(Clone)]
pub struct RpcPeer {
    inner: Arc<RpcPeerInner>,
}

#[derive(Serialize)]
struct ClientInfo<'a> {
    name: &'a str,
    title: &'a str,
    version: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InitializeParams<'a> {
    client_info: ClientInfo<'a>,
}

impl RpcPeer {
    pub fn new<R, W>(reader: R, writer: W) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (notifications, _) = broadcast::channel(64);
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let inner = Arc::new(RpcPeerInner {
            writer: Mutex::new(Box::new(writer)),
            pending: pending.clone(),
            next_id: AtomicU64::new(1),
            notifications: notifications.clone(),
        });

        tokio::spawn(read_messages(reader, pending, notifications));
        Self { inner }
    }

    pub fn from_stream<S>(stream: S) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let (reader, writer) = tokio::io::split(stream);
        Self::new(reader, writer)
    }

    pub async fn initialize(&self) -> Result<(), AppError> {
        let params = serde_json::to_value(InitializeParams {
            client_info: ClientInfo {
                name: "quota_dial",
                title: "QuotaDial",
                version: env!("CARGO_PKG_VERSION"),
            },
        })?;
        self.request("initialize", Some(params)).await?;
        self.send_notification("initialized", serde_json::json!({}))
            .await
    }

    pub async fn request(&self, method: &str, params: Option<Value>) -> Result<Value, AppError> {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let mut message = serde_json::Map::new();
        message.insert("method".into(), Value::String(method.into()));
        message.insert("id".into(), Value::from(id));
        if let Some(params) = params {
            message.insert("params".into(), params);
        }

        let (sender, receiver) = oneshot::channel();
        self.inner.pending.lock().await.insert(id, sender);
        if let Err(error) = self.write(Value::Object(message)).await {
            self.inner.pending.lock().await.remove(&id);
            return Err(error);
        }

        receiver.await.map_err(|_| AppError::Disconnected)?
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RpcNotification> {
        self.inner.notifications.subscribe()
    }

    async fn send_notification(&self, method: &str, params: Value) -> Result<(), AppError> {
        self.write(serde_json::json!({
            "method": method,
            "params": params,
        }))
        .await
    }

    async fn write(&self, message: Value) -> Result<(), AppError> {
        let mut bytes = serde_json::to_vec(&message)?;
        bytes.push(b'\n');
        let mut writer = self.inner.writer.lock().await;
        writer.write_all(&bytes).await?;
        writer.flush().await?;
        Ok(())
    }
}

async fn read_messages<R>(
    reader: R,
    pending: Arc<Mutex<HashMap<u64, PendingSender>>>,
    notifications: broadcast::Sender<RpcNotification>,
) where
    R: AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                let Ok(message) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if let Some(id) = message.get("id").and_then(Value::as_u64) {
                    let Some(sender) = pending.lock().await.remove(&id) else {
                        continue;
                    };
                    let response = match message.get("error") {
                        Some(error) => Err(AppError::Rpc {
                            code: error.get("code").and_then(Value::as_i64),
                            message: error
                                .get("message")
                                .and_then(Value::as_str)
                                .unwrap_or("unknown app-server error")
                                .to_owned(),
                        }),
                        None => Ok(message.get("result").cloned().unwrap_or(Value::Null)),
                    };
                    let _ = sender.send(response);
                    continue;
                }

                if let Some(method) = message.get("method").and_then(Value::as_str) {
                    let _ = notifications.send(RpcNotification {
                        method: method.to_owned(),
                        params: message.get("params").cloned().unwrap_or(Value::Null),
                    });
                }
            }
            Ok(None) | Err(_) => {
                let mut requests = pending.lock().await;
                for (_, sender) in requests.drain() {
                    let _ = sender.send(Err(AppError::Disconnected));
                }
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    #[tokio::test]
    async fn sends_initialize_before_account_calls() {
        let (client, server) = tokio::io::duplex(4096);
        let (server_read, mut server_write) = tokio::io::split(server);
        let peer = RpcPeer::from_stream(client);
        let server_task = tokio::spawn(async move {
            let mut lines = BufReader::new(server_read).lines();

            let initialize: serde_json::Value =
                serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();
            assert_eq!(
                initialize,
                serde_json::json!({
                    "method": "initialize",
                    "id": 1,
                    "params": {
                        "clientInfo": {
                            "name": "quota_dial",
                            "title": "QuotaDial",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    }
                })
            );
            server_write
                .write_all(b"{\"id\":1,\"result\":{}}\n")
                .await
                .unwrap();

            let initialized: serde_json::Value =
                serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();
            assert_eq!(
                initialized,
                serde_json::json!({"method": "initialized", "params": {}})
            );

            let request: serde_json::Value =
                serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();
            assert_eq!(
                request,
                serde_json::json!({"method": "account/rateLimits/read", "id": 2})
            );
            server_write
                .write_all(b"{\"id\":2,\"result\":{\"rateLimits\":{\"limitId\":\"codex\"}}}\n")
                .await
                .unwrap();
        });

        peer.initialize().await.unwrap();
        let value = peer.request("account/rateLimits/read", None).await.unwrap();

        assert_eq!(value["rateLimits"]["limitId"], "codex");
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn routes_server_notifications_to_subscribers() {
        let (client, mut server) = tokio::io::duplex(1024);
        let peer = RpcPeer::from_stream(client);
        let mut notifications = peer.subscribe();

        server
            .write_all(
                b"{\"method\":\"account/rateLimits/updated\",\"params\":{\"source\":\"push\"}}\n",
            )
            .await
            .unwrap();

        let notification = notifications.recv().await.unwrap();
        assert_eq!(notification.method, "account/rateLimits/updated");
        assert_eq!(notification.params["source"], "push");
    }

    #[tokio::test]
    async fn returns_rpc_errors_to_the_matching_request() {
        let (client, server) = tokio::io::duplex(1024);
        let (server_read, mut server_write) = tokio::io::split(server);
        let peer = RpcPeer::from_stream(client);
        let server_task = tokio::spawn(async move {
            let mut lines = BufReader::new(server_read).lines();
            let _request = lines.next_line().await.unwrap().unwrap();
            server_write
                .write_all(
                    b"{\"id\":1,\"error\":{\"code\":-32601,\"message\":\"unknown method\"}}\n",
                )
                .await
                .unwrap();
        });

        let error = peer.request("missing/method", None).await.unwrap_err();
        assert!(error.to_string().contains("unknown method"));
        server_task.await.unwrap();
    }
}
