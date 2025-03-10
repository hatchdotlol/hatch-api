use redis::{RedisError, AsyncCommands};
use rmp_serde::Serializer;
use rocket::futures::StreamExt;
use serde::{Deserialize, Serialize};
use webhook::client::WebhookClient;

use crate::{config::report_webhook, data::NumOrStr, db::REDIS};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ReportLog {
    pub reportee: u32,
    pub reason: String,
    pub resource_id: NumOrStr,
    pub location: u8
}

pub async fn report_queue() -> Result<(), RedisError> {
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut pubsub_conn = client.get_async_pubsub().await?;

    let _ = pubsub_conn.subscribe("reports").await?;
    let mut msgs = pubsub_conn.on_message();

    let webhook = report_webhook().map(|url| WebhookClient::new(url));

    loop {
        while let Some(msg) = msgs.next().await {
            let payload = msg.get_payload::<Vec<u8>>().unwrap();
            let _report: ReportLog = rmp_serde::from_slice(&payload).unwrap();

            if let Some(client) = webhook.as_ref() {
                let title = "hello world";
                let description = "foo bar";

                client
                    .send(move |message| {
                        message.embed(|embed| {
                            embed
                                .title(title)
                                .description(description)
                        })
                    })
                    .await
                    .unwrap();
            }
        }
    }
}

pub fn send_report(report: ReportLog) {
    let mut buf = vec![];
    report.serialize(&mut Serializer::new(&mut buf)).unwrap();

    tokio::spawn(async move {
        let redis = REDIS.get().unwrap();
        let mut redis = redis.lock().await;
        let _: () = redis.publish("reports", buf).await.unwrap();
    });
}