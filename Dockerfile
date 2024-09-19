# docker build --platform=linux/amd64 -t waithook . ; say done

## Builder
FROM rust:alpine AS builder

RUN apk add pkgconf openssl-dev openssl-libs-static musl-dev

WORKDIR /app

COPY ./ .

ENV RUST_BACKTRACE=1
RUN cargo build --release


## Final image
FROM alpine:latest

WORKDIR /app

# Copy our build
COPY --from=builder /app/public ./public
COPY --from=builder /app/target/release/waithook ./

CMD ["/app/waithook"]
