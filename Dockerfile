FROM scratch

ARG TARGETPLATFORM
COPY $TARGETPLATFORM/ecs-meta2env-rs /ecs-meta2env-rs
