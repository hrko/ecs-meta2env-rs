FROM mcr.microsoft.com/devcontainers/base:bullseye

ARG ZIG_VERSION=0.13.0

# enable Taskfile completion
RUN wget https://github.com/go-task/task/raw/a1b3bb03ed7f7339ba7a03db0bbf8e5ca35eef95/completion/bash/task.bash \
        -O /etc/bash_completion.d/task && \
    chmod 755 /etc/bash_completion.d/task

# install zig for cargo-zigbuild
RUN wget https://ziglang.org/download/${ZIG_VERSION}/zig-linux-x86_64-${ZIG_VERSION}.tar.xz -O - | tar -xJf - -C /usr/local
ENV PATH=/usr/local/zig-linux-x86_64-${ZIG_VERSION}:$PATH
