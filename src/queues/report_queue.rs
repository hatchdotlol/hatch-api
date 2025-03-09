use redis::{RedisError, AsyncCommands};
use rocket::futures::StreamExt;

use crate::db::REDIS;

pub async fn report_queue() -> Result<(), RedisError> {
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut pubsub_conn = client.get_async_pubsub().await?;

    let _ = pubsub_conn.subscribe("reports").await?;
    let mut msgs = pubsub_conn.on_message();

    loop {
        while let Some(msg) = msgs.next().await {
            println!("{}", msg.get_payload::<String>().unwrap());
        }
    }
}

pub fn send_report(report: String) {
    tokio::spawn(async move {
        let redis = REDIS.get().unwrap();
        let mut redis = redis.lock().await;
        let _: () = redis.publish("reports", report).await.unwrap();
    });
}