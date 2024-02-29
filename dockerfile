FROM rust:1.74.1-alpine

WORKDIR ~/learn/rust/rb-start/
COPY . .

RUN cargo install --path .

CMD ["rb-start"]