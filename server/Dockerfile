FROM rust:buster

WORKDIR /usr/src/magmaserver
COPY . .

RUN cargo install --path .

CMD ["magmaserver"]
