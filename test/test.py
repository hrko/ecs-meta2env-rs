#!/usr/bin/env python3

import http.server
import json
import threading
import subprocess
import os


class RequestHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/":
            self.send_response(200)
            self.send_header("Content-type", "application/json")
            self.end_headers()
            with open("container.json") as f:
                container_data = json.load(f)
            self.wfile.write(json.dumps(container_data).encode("utf-8"))
        elif self.path == "/task":
            self.send_response(200)
            self.send_header("Content-type", "application/json")
            self.end_headers()
            with open("task.json") as f:
                task_data = json.load(f)
            self.wfile.write(json.dumps(task_data).encode("utf-8"))
        else:
            self.send_response(404)
            self.end_headers()


def run(server_class=http.server.HTTPServer, handler_class=RequestHandler):
    server_address = ("", 5000)
    httpd = server_class(server_address, handler_class)
    print("Starting httpd server on port 5000...")
    httpd.serve_forever()


def test_ecs_meta2env_rs(input_env: dict, expected_env: dict):
    # run ecs-meta2env-rs
    env = os.environ.copy()
    env = {**env, **input_env}
    cp = subprocess.run(
        ["../target/x86_64-unknown-linux-musl/release/ecs-meta2env-rs", "printenv"],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    # parse output
    output_env = {}
    for line in cp.stdout.decode("utf-8").split("\n"):
        if line == "":
            continue
        key, value = line.split("=", 1)
        output_env[key] = value
    # ensure expected environment variables are set
    if cp.returncode != 0:
        print("=== stdout ===")
        print(cp.stdout.decode("utf-8"))
        print("=== stderr ===")
        print(cp.stderr.decode("utf-8"))
        raise Exception("Failed to run ecs-meta2env-rs")
    for key, value in expected_env.items():
        if output_env[key] != value:
            raise Exception(f"Expected {key} to be {value}, got {output_env[key]}")
        else:
            print(f"OK: {key}={value}")
    return


if __name__ == "__main__":
    # start mock container metadata endpoint
    server_thread = threading.Thread(target=run)
    server_thread.daemon = True
    server_thread.start()

    # test case 1
    print("Test case 1 - metadata from endpoint")
    input_env = {"ECS_CONTAINER_METADATA_URI_V4": "http://127.0.0.1:5000"}
    expected_env = {
        "X_ECS_CLUSTER": "default",
        "X_ECS_TASK_ARN": "arn:aws:ecs:us-west-2:111122223333:task/default/158d1c8083dd49d6b527399fd6414f5c",
        "X_ECS_FAMILY": "curltest",
        "X_ECS_REVISION": "26",
        "X_ECS_SERVICE_NAME": "MyService",
        "X_ECS_CONTAINER_NAME": "curl",
        "X_ECS_CONTAINER_DOCKER_NAME": "ecs-curltest-24-curl-cca48e8dcadd97805600",
        "X_ECS_CONTAINER_ARN": "arn:aws:ecs:us-west-2:111122223333:container/0206b271-b33f-47ab-86c6-a0ba208a70a9",
        "X_ECS_CONTAINER_INSTANCE_ARN": "",
    }
    test_ecs_meta2env_rs(input_env, expected_env)
    print("Test case 1 passed!")

    # test case 2
    print("Test case 2 - metadata from endpoint and file")
    input_env = {
        "ECS_CONTAINER_METADATA_URI_V4": "http://127.0.0.1:5000",
        "ECS_CONTAINER_METADATA_FILE": "container_file.json",
        "META2ENV_USE_FILE": "true",
    }
    expected_env = {
        "X_ECS_CLUSTER": "default",
        "X_ECS_TASK_ARN": "arn:aws:ecs:us-west-2:111122223333:task/default/158d1c8083dd49d6b527399fd6414f5c",
        "X_ECS_FAMILY": "curltest",
        "X_ECS_REVISION": "26",
        "X_ECS_SERVICE_NAME": "MyService",
        "X_ECS_CONTAINER_NAME": "curl",
        "X_ECS_CONTAINER_DOCKER_NAME": "ecs-curltest-24-curl-cca48e8dcadd97805600",
        "X_ECS_CONTAINER_ARN": "arn:aws:ecs:us-west-2:111122223333:container/0206b271-b33f-47ab-86c6-a0ba208a70a9",
        "X_ECS_CONTAINER_INSTANCE_ARN": "arn:aws:ecs:us-west-2:012345678910:container-instance/default/1f73d099-b914-411c-a9ff-81633b7741dd",
    }
    test_ecs_meta2env_rs(input_env, expected_env)
    print("Test case 2 passed!")
