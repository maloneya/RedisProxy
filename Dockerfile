FROM rust:latest

WORKDIR /usr/src/redisproxy

ENV REDIS_ADDR="redis://127.0.0.1"

COPY . .

# Rocket depends on nightly :( 
RUN rustup default nightly

RUN cargo build --release 
RUN cargo test
RUN cargo install --path . 


CMD redis_proxy --redis_addr ${REDIS_ADDR}