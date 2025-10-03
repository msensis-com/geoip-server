FROM rust:1-alpine3.22 AS builder
RUN apk add --no-cache musl-dev pkgconf git wget unzip

WORKDIR /build
COPY . /build
RUN cargo build --bins --release

RUN mkdir /mmdb && cd /mmdb && \
    wget "https://download.ip2location.com/lite/IP2LOCATION-LITE-DB1.MMDB.ZIP" -O geoip.mmdb.zip && \
    unzip geoip.mmdb.zip && rm geoip.mmdb.zip && \
    find . -type f -name '*.mmdb' -or -name '*.MMDB' -exec mv {} /geoip.mmdb ';' && \
    cd / && rm -rf /mmdb

FROM scratch

COPY --from=builder /build/target/release/geoip-server /
COPY --from=builder /geoip.mmdb /geoip.mmdb

EXPOSE 3000
CMD ["/geoip-server", "--bind", "0.0.0.0:3000", "/geoip.mmdb"]
