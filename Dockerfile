FROM rust:1.91.0-alpine3.22 AS chef
WORKDIR /app

RUN apk add --no-cache musl-dev cargo-chef


FROM chef AS planner

COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM chef AS builder 

COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release


FROM scratch

COPY --from=builder /app/target/release/ironfoil /bin/ironfoil

WORKDIR /app
USER 1000:1000

ENTRYPOINT ["ironfoil"]
