FROM rust:1.95-bookworm AS chef

WORKDIR /usr/src/app
RUN cargo install --locked cargo-chef

FROM chef AS planner

COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM chef AS builder

# Build dependencies
COPY --from=planner /usr/src/app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
# Build application
RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /usr/src/app

COPY --from=builder /usr/src/app/target/release/gge-fortress-timer /usr/local/bin/gge-fortress-timer
CMD ["gge-fortress-timer"]
