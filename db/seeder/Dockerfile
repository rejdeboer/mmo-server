FROM lukemathwalker/cargo-chef:latest-rust-1.91.0 AS chef
WORKDIR /app
RUN apt update && apt install lld clang -y

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . ./
ENV SQLX_OFFLINE true
RUN cargo build --release -p db-seeder 

FROM debian:trixie-slim AS db-seeder
WORKDIR /app
RUN apt-get update -y \
	&& apt-get install -y --no-install-recommends openssl ca-certificates \
	&& apt-get autoremove -y \
	&& apt-get clean -y \
	&& rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/db-seeder db-seeder
CMD ["./db-seeder", "serve"]

