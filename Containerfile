FROM rust:1.96-trixie AS build

ENV PROTOC_VERSION=35.1

RUN curl -LO "https://github.com/protocolbuffers/protobuf/releases/download/v${PROTOC_VERSION}/protoc-${PROTOC_VERSION}-linux-x86_64.zip" \
    && unzip -o "protoc-${PROTOC_VERSION}-linux-x86_64.zip" -d /usr/local bin/protoc \
    && unzip -o "protoc-${PROTOC_VERSION}-linux-x86_64.zip" -d /usr/local 'include/*' \
    && rm "protoc-${PROTOC_VERSION}-linux-x86_64.zip"

RUN cargo new --bin /app/

COPY Cargo.toml Cargo.lock /app/

WORKDIR /app/

RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release

COPY . /app/

RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release

CMD ["/app/target/release/awuba-iot"]

FROM gcr.io/distroless/cc-debian13 AS app
COPY --from=build /app/target/release/awuba-iot /awuba-iot
CMD ["/awuba-iot"]
