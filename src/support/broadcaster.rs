use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::RwLock;

pub type Tx = UnboundedSender<String>;

pub struct ClientSession {
    pub id: u64,
    pub tx: Tx,
}

#[derive(Default)]
pub struct BroadcasterState {
    pub channels: RwLock<HashMap<String, Vec<ClientSession>>>,
    pub next_id: std::sync::atomic::AtomicU64,
}

impl BroadcasterState {
    pub fn next_conn_id(&self) -> u64 {
        self.next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    pub async fn subscribe(&self, channel_name: &str, session: ClientSession) {
        let mut channels = self.channels.write().await;
        let entry = channels.entry(channel_name.to_string()).or_default();
        entry.retain(|c| c.id != session.id);
        entry.push(session);
    }

    pub async fn unsubscribe(&self, channel_name: &str, session_id: u64) {
        let mut channels = self.channels.write().await;
        if let Some(sessions) = channels.get_mut(channel_name) {
            sessions.retain(|c| c.id != session_id);
        }
    }

    pub async fn remove_session(&self, session_id: u64) {
        let mut channels = self.channels.write().await;
        for sessions in channels.values_mut() {
            sessions.retain(|c| c.id != session_id);
        }
    }
}

pub struct Broadcaster;

static BROADCASTER_STATE: std::sync::OnceLock<Arc<BroadcasterState>> = std::sync::OnceLock::new();

impl Broadcaster {
    pub fn state() -> &'static Arc<BroadcasterState> {
        BROADCASTER_STATE.get_or_init(|| Arc::new(BroadcasterState::default()))
    }

    pub fn to(channel: &str) -> ChannelBroadcaster {
        ChannelBroadcaster {
            channel: channel.to_string(),
        }
    }
}

pub struct ChannelBroadcaster {
    channel: String,
}

impl ChannelBroadcaster {
    pub async fn emit<T: serde::Serialize>(&self, event: &str, payload: T) {
        let state = Broadcaster::state();
        let msg = serde_json::json!({
            "event": event,
            "channel": self.channel,
            "data": payload
        });
        let msg_str = match serde_json::to_string(&msg) {
            Ok(s) => s,
            Err(_) => return,
        };

        let channels = state.channels.read().await;
        if let Some(sessions) = channels.get(&self.channel) {
            for session in sessions {
                let _ = session.tx.send(msg_str.clone());
            }
        }
    }
}
