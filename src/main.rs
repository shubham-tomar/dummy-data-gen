use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rand::Rng;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use tokio::task;
use serde_json::json;
use std::sync::Arc;

#[warn(unused_imports)]

const TOPIC: &str = "test-topic";
const BROKER: &str = "localhost:9092";
const THREADS: usize = 10;
const MESSAGE_INTERVAL: Duration = Duration::from_millis(1);

fn generate_name() -> &'static str {
    let names = ["Alice", "Bob", "Charlie", "David", "Eve", "Frank", "Grace", "Heidi"];
    let index = rand::thread_rng().gen_range(0..names.len());
    return names[index];
}

fn generate_id() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

fn generate_clicks() -> u16 {
    rand::thread_rng().gen_range(1..=40)
}

async fn produce_logs(producer: Arc<FutureProducer>) {
    loop {
        let id = generate_id();
        let name = generate_name();
        let clicks = generate_clicks();
        let created_at = chrono::Utc::now().to_rfc3339();

        let message = json!({
            "id": id,
            "name": name,
            "clicks": clicks,
            "created_at": created_at
        }).to_string();

        let id_str = id.to_string(); // Store the key separately

        let producer_clone = Arc::clone(&producer);
        tokio::spawn(async move {
            let record = FutureRecord::to(TOPIC)
                .payload(message.as_str())
                .key(id_str.as_str());
            match producer_clone.send(record, Duration::from_secs(0)).await {
                Ok(_) => println!("Message sent: {}", message),
                Err((e, _)) => eprintln!("Failed to send message: {}", e),
            }
        });

        sleep(MESSAGE_INTERVAL).await;
    }
}


#[tokio::main]

async fn main() {
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", BROKER)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Producer creation error");

    let producer = Arc::new(producer);
    let mut tasks = Vec::new();

    for _ in 0..THREADS {
        let producer_clone = Arc::clone(&producer);
        tasks.push(task::spawn(produce_logs(producer_clone)));
    }

    futures::future::join_all(tasks).await;
}
