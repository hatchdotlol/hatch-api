use redis::RedisError;

use crate::db::redis;

pub async fn report_queue() -> Result<(), RedisError> {
    let mut client = redis().lock().unwrap();
    let mut pubsub = client.as_pubsub();

    pubsub.subscribe("reports")?;

    loop {
        println!("ping");
        let msg = pubsub.get_message().unwrap();
        let payload: String = msg.get_payload().unwrap();

        println!("{}", payload)
    }
}
