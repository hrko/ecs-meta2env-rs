use std::env;
use std::fs;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::time::Duration;

use aws_config::meta::region::RegionProviderChain;
use aws_config::Region;
use aws_sdk_ecs as ecs;
use aws_sdk_ssm as ssm;
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
    // ServiceName is not present on Fargate tasks at the time of writing
    // ref. https://docs.aws.amazon.com/ja_jp/AmazonECS/latest/developerguide/task-metadata-endpoint-v4-fargate-examples.html
    #[serde(rename = "ServiceName")]
    service_name: Option<String>,
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

#[derive(Deserialize)]
struct ContainerMetadataFile {
    #[serde(rename = "ContainerInstanceARN")]
    container_instance_arn: String,
}

#[::tokio::main]
async fn main() {
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

    // get container instance ARN from metadata file if `META2ENV_USE_FILE` is set
    let container_instance_arn = if env::var("META2ENV_USE_FILE").is_ok() {
        let metadata_file_path = env::var("ECS_CONTAINER_METADATA_FILE").expect("ECS_CONTAINER_METADATA_FILE environment variable not set");
        let metadata_file_content = fs::read_to_string(metadata_file_path).expect("Error reading metadata file");
        let file_metadata: ContainerMetadataFile = serde_json::from_str(&metadata_file_content).expect("Error parsing metadata file");
        file_metadata.container_instance_arn
    } else {
        // otherwise, CONTAINER_INSTANCE_ARN will be empty
        String::new()
    };

    // get container instance hostname by calling ecs:DescribeContainerInstances and
    // ssm:DescribeInstanceInformation if both `META2ENV_USE_FILE` and `META2ENV_FETCH_HOSTNAME` are set.
    let container_instance_hostname = if env::var("META2ENV_USE_FILE").is_ok() && env::var("META2ENV_FETCH_HOSTNAME").is_ok() {
        let parsed_region = extract_region_from_arn(&container_instance_arn).expect("Failed to extract region from container_instance_arn");
        let region_provider = RegionProviderChain::default_provider().or_else(Region::new(parsed_region.clone()));
        let config = aws_config::from_env().region(region_provider).load().await;

        let ecs_client = ecs::Client::new(&config);
        let container_instance = ecs_client
            .describe_container_instances()
            .cluster(&task_metadata.cluster)
            .container_instances(&container_instance_arn)
            .send()
            .await
            .expect("Error fetching container instance");
        let container_instances = container_instance.container_instances.unwrap();
        let instance_id = container_instances[0].ec2_instance_id.as_ref().expect("Instance ID not found");

        let ssm_client = ssm::Client::new(&config);
        let instance_info = ssm_client
            .describe_instance_information()
            .filters(
                ssm::types::InstanceInformationStringFilter::builder()
                    .key("InstanceIds")
                    .values(instance_id)
                    .build()
                    .unwrap(),
            )
            .send()
            .await
            .expect("Error fetching instance information");
        let computer_name = instance_info.instance_information_list.unwrap()[0].computer_name.clone();
        let hostname = computer_name.as_ref().expect("Computer name not found");
        hostname.clone()
    } else {
        // otherwise, CONTAINER_INSTANCE_HOSTNAME will be empty
        String::new()
    };

    if let Some(command) = std::env::args().nth(1) {
        let error = Command::new(command)
            .args(std::env::args().skip(2))
            .env(format!("{}CLUSTER", PREFIX), task_metadata.cluster)
            .env(format!("{}TASK_ARN", PREFIX), task_metadata.task_arn)
            .env(format!("{}FAMILY", PREFIX), task_metadata.family)
            .env(format!("{}REVISION", PREFIX), task_metadata.revision)
            .env(format!("{}SERVICE_NAME", PREFIX), task_metadata.service_name.unwrap_or_default())
            .env(format!("{}CONTAINER_NAME", PREFIX), container_metadata.name)
            .env(format!("{}CONTAINER_DOCKER_NAME", PREFIX), container_metadata.docker_name)
            .env(format!("{}CONTAINER_ARN", PREFIX), container_metadata.container_arn)
            .env(format!("{}CONTAINER_INSTANCE_ARN", PREFIX), container_instance_arn)
            .env(format!("{}CONTAINER_INSTANCE_HOSTNAME", PREFIX), container_instance_hostname)
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

fn extract_region_from_arn(arn: &str) -> Option<String> {
    // arn:aws:ecs:ap-northeast-1:123456789012:container-instance/xxx
    let parts: Vec<&str> = arn.split(':').collect();
    if parts.len() > 3 {
        Some(parts[3].to_string())
    } else {
        None
    }
}
