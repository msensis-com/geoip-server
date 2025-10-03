FROM rust:1-alpine3.22 AS builder
RUN apk add --no-cache musl-dev pkgconf git

WORKDIR /build
COPY . /build
RUN cargo build --bins --release

FROM scratch
ARG MMDB="countries.mmdb"

COPY --from=builder /build/target/release/geoip-server /
COPY --from=builder /build/$MMDB /geoip.mmdb
CMD ["/geoip-server", "/geoip.mmdb"]
