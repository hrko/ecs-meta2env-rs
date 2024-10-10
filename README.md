# ecs-meta2env-rs
`ecs-meta2env-rs` is a tool designed to export values from ECS container metadata endpoints to environment variables. This is particularly useful for passing ECS metadata to applications like Fluent Bit to add metadata to logs.

## Download

You can download the latest release from the [releases page](https://github.com/hrko/ecs-meta2env-rs/releases/latest).

## Usage

`ecs-meta2env-rs` is intended to be used as an entrypoint for a container. It fetches metadata from the ECS container metadata endpoint and exports the values to environment variables.

### Example Dockerfile

Below is an example of how to use `ecs-meta2env-rs` in a Dockerfile:

```Dockerfile
FROM debian:bookworm-slim AS ecs-meta2env-rs-downloader
RUN apt-get update && apt-get install -y curl
RUN if [ "$(uname -m)" = "x86_64" ]; then ARCH="amd64"; else ARCH="arm64"; fi && \
    curl -L -o /meta2env https://github.com/hrko/ecs-meta2env-rs/releases/download/v1.0.0/ecs-meta2env-rs-$ARCH && \
    chmod +x /meta2env

FROM <original-image>
COPY --from=ecs-meta2env-rs-downloader /meta2env /meta2env
ENTRYPOINT ["/meta2env", "<original-entrypoint...>"]
```

## Environment Variables

`ecs-meta2env-rs` will export the following environment variables:

* `X_ECS_CLUSTER`
* `X_ECS_TASK_ARN`
* `X_ECS_FAMILY`
* `X_ECS_REVISION`
* `X_ECS_SERVICE_NAME`
* `X_ECS_CONTAINER_NAME`
* `X_ECS_CONTAINER_DOCKER_NAME`
* `X_ECS_CONTAINER_ARN`
* `X_ECS_CONTAINER_INSTANCE_ARN` (only when `META2ENV_USE_FILE` is set, see below for more information)

Because the ECS metadata endpoint lacks `ContainerInstanceARN`, it needs to be read from the [container metadata file](https://docs.aws.amazon.com/AmazonECS/latest/developerguide/container-metadata.html). To do this, set the `META2ENV_USE_FILE` environment variable to any value. Note that this requires the ECS agent to be [configured to write the container metadata file](https://docs.aws.amazon.com/AmazonECS/latest/developerguide/enable-metadata.html). This feature is only available when the launch type is `EC2` or `EXTERNAL`. If `META2ENV_USE_FILE` is not set, `X_ECS_CONTAINER_INSTANCE_ARN` will be empty string, but still exported.

## Development

### Prerequisites

* Dev Container: The project is set up to be used with a dev container. If you are using VS Code, you can open the project in a dev container by selecting the `Reopen in Container` option.

### Building

To build the project, run the following command:

```sh
task build
```

This will create the binaries in the `./target` directory.

### Testing

To run the tests, run the following command:

```sh
task test
```

## References

* [Original Idea](https://github.com/aws/aws-for-fluent-bit/issues/62#issuecomment-925702432): The idea of inserting a shell script at the entry point is suggested here, but `ecs-meta2env-rs` was created to achieve the same for containers with no shell or limited built-in commands, such as the *distroless* container.
* [Amazon ECS task metadata endpoint version 4](https://docs.aws.amazon.com/AmazonECS/latest/developerguide/task-metadata-endpoint-v4.html)