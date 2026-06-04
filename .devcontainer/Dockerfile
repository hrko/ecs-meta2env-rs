FROM mcr.microsoft.com/devcontainers/base:bullseye

# install mise via apt repository
RUN apt-get update -y && \
    apt-get install -y --no-install-recommends curl ca-certificates && \
    install -dm 755 /etc/apt/keyrings && \
    curl -fSs https://mise.en.dev/gpg-key.pub -o /etc/apt/keyrings/mise-archive-keyring.asc && \
    echo "deb [signed-by=/etc/apt/keyrings/mise-archive-keyring.asc] https://mise.en.dev/deb stable main" > /etc/apt/sources.list.d/mise.list && \
    apt-get update -y && \
    apt-get install -y --no-install-recommends mise && \
    rm -rf /var/lib/apt/lists/*
