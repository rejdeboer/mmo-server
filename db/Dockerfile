FROM rust:1.87 as migrator

RUN cargo install sqlx-cli --version="~0.8"

WORKDIR /app
COPY ./db/migrations ./migrations
COPY ./.sqlx ./.sqlx

CMD ["sqlx", "migrate", "run"]
