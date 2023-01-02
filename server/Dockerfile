FROM rust:buster

WORKDIR /usr/src/rserver
COPY . .

RUN cargo install --path .

CMD ["rserver"]
