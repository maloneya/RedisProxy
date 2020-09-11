FROM rust:latest

WORKDIR /usr/src/redisproxy

COPY . .

# Rocket depends on nihgtly :( 
RUN rustup default nightly

# RUN cargo build --release 

RUN cargo install --path . 


CMD ["redis_proxy"]