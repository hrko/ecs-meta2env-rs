use std::env;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::time::Duration;

use reqwest::blocking::Client;
use serde::de::DeserializeOwned;
use serde::Deserialize;

const PREFIX: &str = "X_ECS_";
const MAX_RETRIES: u32 = 3;
const RETRY_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Deserialize)]
struct TaskMetadata {
    #[serde(rename = "Cluster")]
    cluster: String,
    #[serde(rename = "TaskARN")]
    task_arn: String,
    #[serde(rename = "Family")]
    family: String,
    #[serde(rename = "Revision")]
    revision: String,
    #[serde(rename = "ServiceName")]
    service_name: String,
}

#[derive(Deserialize)]
struct ContainerMetadata {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "DockerName")]
    docker_name: String,
    #[serde(rename = "ContainerARN")]
    container_arn: String,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <command> [args...]", args[0]);
        std::process::exit(1);
    }

    let metadata_uri = env::var("ECS_CONTAINER_METADATA_URI_V4").expect("ECS_CONTAINER_METADATA_URI_V4 environment variable not set");

    let task_metadata: TaskMetadata =
        fetch_metadata_with_retry(format!("{}/task", metadata_uri), MAX_RETRIES, RETRY_INTERVAL).expect("Error fetching task metadata");

    let container_metadata: ContainerMetadata =
        fetch_metadata_with_retry(metadata_uri, MAX_RETRIES, RETRY_INTERVAL).expect("Error fetching container metadata");

    if let Some(command) = std::env::args().nth(1) {
        let error = Command::new(command)
            .args(std::env::args().skip(2))
            .env(format!("{}CLUSTER", PREFIX), task_metadata.cluster)
            .env(format!("{}TASK_ARN", PREFIX), task_metadata.task_arn)
            .env(format!("{}FAMILY", PREFIX), task_metadata.family)
            .env(format!("{}REVISION", PREFIX), task_metadata.revision)
            .env(format!("{}SERVICE_NAME", PREFIX), task_metadata.service_name)
            .env(format!("{}CONTAINER_NAME", PREFIX), container_metadata.name)
            .env(format!("{}CONTAINER_DOCKER_NAME", PREFIX), container_metadata.docker_name)
            .env(format!("{}CONTAINER_ARN", PREFIX), container_metadata.container_arn)
            .exec();
        eprintln!("Failed to execute command: {}", error);
    } else {
        eprintln!("No command provided");
    }
}

fn fetch_metadata_with_retry<T: DeserializeOwned>(url: String, max_retries: u32, retry_interval: Duration) -> Result<T, reqwest::Error> {
    let client = Client::new();
    let mut last_error = None;
    for _ in 0..max_retries {
        match fetch_metadata(&client, &url) {
            Ok(data) => return Ok(data),
            Err(err) => {
                eprintln!("Error fetching metadata: {}", err);
                std::thread::sleep(retry_interval);
                last_error = Some(err);
            }
        }
    }
    Err(last_error.unwrap())
}

fn fetch_metadata<T: DeserializeOwned>(client: &Client, url: &str) -> Result<T, reqwest::Error> {
    let resp = client.get(url).send()?;
    resp.json()
}
