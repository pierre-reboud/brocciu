FROM rust:latest

WORKDIR /myapp
COPY . .

# RUN cargo install --path .
RUN cargo build --release

CMD cargo run --release