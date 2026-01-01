FROM lukemathwalker/cargo-chef:latest-rust-1.91.0 AS chef
WORKDIR /app
RUN apt update && apt install lld clang protobuf-compiler -y

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json -p web-server -p mmo-server
COPY . ./
ENV SQLX_OFFLINE true
RUN cargo build --release -p web-server -p mmo-server

FROM debian:trixie-slim AS runtime-base
WORKDIR /app
RUN apt-get update -y \
	&& apt-get install -y --no-install-recommends openssl ca-certificates \
	&& apt-get autoremove -y \
	&& apt-get clean -y \
	&& rm -rf /var/lib/apt/lists/*

FROM runtime-base AS web-server
COPY --from=builder /app/target/release/web-server web-server
COPY web-server/configuration configuration
CMD ["./web-server"]
LABEL service=web-server

FROM runtime-base AS mmo-server
COPY --from=builder /app/target/release/mmo-server mmo-server
COPY mmo-server/configuration configuration
# TODO: Probably we want to improve the way we handle assets in prod
COPY mmo-server/assets assets
CMD ["./mmo-server"]
LABEL service=mmo-server
