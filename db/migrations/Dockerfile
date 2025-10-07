FROM rust:1.89 as builder

RUN cargo install sqlx-cli --version="~0.8" --locked

FROM debian:bookworm-slim as migrator

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/sqlx /usr/local/bin/sqlx
WORKDIR /app

COPY ./db/migrations ./migrations
COPY ./.sqlx ./.sqlx

CMD ["sqlx", "migrate", "run"]
