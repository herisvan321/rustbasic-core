#[cfg(feature = "websocket")]
#[tokio::test]
async fn test_broadcaster_subscription_and_emission() {
    use rustbasic_core::support::broadcaster::{Broadcaster, ClientSession};
    use tokio::sync::mpsc;

    let state = Broadcaster::state();
    let conn_id = 999;
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // 1. Subscribe to a channel
    let session = ClientSession {
        id: conn_id,
        tx: tx.clone(),
    };
    state.subscribe("orders", session).await;

    // Verify channel has the subscription
    {
        let channels = state.channels.read().await;
        let sessions = channels.get("orders").unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, conn_id);
    }

    // 2. Broadcast an event
    Broadcaster::to("orders").emit("updated", serde_json::json!({ "id": 123, "status": "shipped" })).await;

    // 3. Verify message is received by the channel receiver
    let received = rx.recv().await.unwrap();
    let val: serde_json::Value = serde_json::from_str(&received).unwrap();
    assert_eq!(val["event"], "updated");
    assert_eq!(val["channel"], "orders");
    assert_eq!(val["data"]["id"], 123);
    assert_eq!(val["data"]["status"], "shipped");

    // 4. Unsubscribe
    state.unsubscribe("orders", conn_id).await;
    {
        let channels = state.channels.read().await;
        let sessions = channels.get("orders").unwrap();
        assert!(sessions.is_empty());
    }
}
