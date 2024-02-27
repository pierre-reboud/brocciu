FROM rust:1.76

EXPOSE 80 80
EXPOSE 443 443

WORKDIR /myapp
COPY . .

RUN cargo install --path .

CMD ["cargo run --release"]