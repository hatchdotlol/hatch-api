use std::time::Duration;

use redis::{AsyncCommands, RedisError};
use rmp_serde::Serializer;
use rocket::futures::StreamExt;
use serde::{Deserialize, Serialize};
use webhook::client::WebhookClient;

use crate::{
    config::{config, USER_DELETION},
    db::{db, REDIS},
};

#[derive(Debug, Serialize, Deserialize)]
pub enum AuditCategory {
    Mod,
    User,
    // Project,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditLog {
    pub culprit: u32,
    pub category: u8,
    pub description: String,
}

fn get_username(id: u32) -> String {
    let cur = db().lock().unwrap();
    cur.user_by_id(id).unwrap().username
}

pub async fn audit_queue() -> Result<(), RedisError> {
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut pubsub_conn = client.get_async_pubsub().await?;

    let _ = pubsub_conn.subscribe("audits").await?;
    let mut msgs = pubsub_conn.on_message();

    let webhook = &config().logging_webhook.as_ref().map(|url| WebhookClient::new(url));

    loop {
        while let Some(msg) = msgs.next().await {
            let payload = msg.get_payload::<Vec<u8>>().unwrap();
            let audit: AuditLog = rmp_serde::from_slice(&payload).unwrap();

            if let Some(client) = webhook.as_ref() {
                let username = get_username(audit.culprit);
                let user_url = format!("https://dev.hatch.lol/user?u={username}");
                let title = username
                    + match audit.category {
                        0 => " did a mod action",
                        1 => " did a user action",
                        _ => unreachable!(),
                    };

                client
                    .send(move |message| {
                        message.embed(|embed| {
                            embed
                                .title(&title)
                                .url(&user_url)
                                .description(&audit.description)
                        })
                    })
                    .await
                    .unwrap();
            }
        }
    }
}

pub fn send_audit(audit: AuditLog) {
    let mut buf = vec![];
    audit.serialize(&mut Serializer::new(&mut buf)).unwrap();

    tokio::spawn(async move {
        let redis = REDIS.get().unwrap();
        let mut redis = redis.lock().await;
        let _: () = redis.publish("audits", buf).await.unwrap();
    });
}

pub fn schedule_deletion(audit: AuditLog) {
    let mut buf = vec![];
    audit.serialize(&mut Serializer::new(&mut buf)).unwrap();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(USER_DELETION)).await;
        let redis = REDIS.get().unwrap();
        let mut redis = redis.lock().await;
        let _: () = redis.publish("audits", buf).await.unwrap();
    });
}
