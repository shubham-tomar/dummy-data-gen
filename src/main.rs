use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rand::{Rng, seq::SliceRandom};
use rand::distributions::Alphanumeric;
use serde_json::{json, Value};
use std::fs;
use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::task;
use tokio::time::sleep;
use std::io::{self, Write};
use clap::Parser;

const BROKER: &str = "localhost:9092";
const THREADS: usize = 1;
const MESSAGE_INTERVAL: Duration = Duration::from_micros(100000);

#[derive(Parser)]
#[command(name = "dummy-data-gen")]
#[command(about = "Kafka data generator for testing")]
struct Args {
    #[arg(short, long, default_value = "src_2")]
    topic: String,
}

/// Load JSON schema from a file
fn load_schema(file_path: &str) -> Value {
    let schema_str = fs::read_to_string(file_path).expect("Failed to read schema file");
    serde_json::from_str(&schema_str).expect("Invalid JSON format")
}

/// Generate a random value based on schema type
fn generate_value(value_type: &str) -> Value {
    let mut rng = rand::thread_rng();

    match value_type {
        "string" => {
            let random_str: String = (0..10)
                .map(|_| rng.sample(Alphanumeric) as char)
                .collect();
            json!(random_str)
        }
        "int" => json!(rng.gen_range(1..=1000)),
        "bool" => json!(rng.gen_bool(0.5)),
        "status" => {
            let statuses = ["SUCCESS", "FAIL", "PENDING"];
            json!(statuses.choose(&mut rng).unwrap())
        }
        "date" => {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs();
            json!(timestamp)
        }
        _ => json!(null), // If unknown type, return null
    }
}

/// Generate a JSON log based on schema
fn generate_log(schema: &Value) -> String {
    let mut log = serde_json::Map::new();

    for (key, value_type) in schema.as_object().unwrap() {
        log.insert(key.clone(), generate_value(value_type.as_str().unwrap()));
    }

    serde_json::to_string(&log).unwrap()
}

async fn produce_logs(producer: Arc<FutureProducer>, counter: Arc<AtomicUsize>, schema: Arc<Value>, topic: String) {
    let mut interval = Instant::now();

    loop {
        let message = generate_log(&schema);
        let id_str = "log_key".to_string(); // Generic key

        let producer_clone = Arc::clone(&producer);
        let topic_clone = topic.clone();
        tokio::spawn(async move {
            let record = FutureRecord::to(&topic_clone)
                .payload(message.as_str())
                .key(id_str.as_str());

            match producer_clone.send(record, Duration::from_secs(0)).await {
                Ok(_) => {},
                Err((e, _)) => eprintln!("Failed to send message: {}", e),
            }
        });

        counter.fetch_add(1, Ordering::Relaxed);

        if interval.elapsed() >= Duration::from_secs(1) {
            print!("\rLogs Rate: {}/sec", counter.swap(0, Ordering::Relaxed));
            io::stdout().flush().unwrap();
            interval = Instant::now();
        }

        sleep(MESSAGE_INTERVAL).await;
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", BROKER)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Producer creation failed");

    let producer = Arc::new(producer);
    let counter = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::new();
    let schema = Arc::new(load_schema("./src/schema.json"));

    for _ in 0..THREADS {
        let producer_clone = Arc::clone(&producer);
        let counter_clone = Arc::clone(&counter);
        let schema_clone = Arc::clone(&schema);
        let topic_clone = args.topic.clone();
        tasks.push(task::spawn(produce_logs(producer_clone, counter_clone, schema_clone, topic_clone)));
    }

    futures::future::join_all(tasks).await;
}
