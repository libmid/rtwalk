FROM rust:1.86

WORKDIR /rtwalk

COPY . .

RUN cargo build --release

EXPOSE 4001

CMD ["./target/release/rtwalk"]
