FROM debian:12-slim AS jemalloc

RUN apt-get update && apt-get install -y --no-install-recommends libjemalloc2

FROM gcr.io/distroless/cc-debian12 AS runtime
COPY --from=jemalloc /usr/lib /usr/lib
ARG BINARY
WORKDIR /app
COPY ./target/release/${BINARY} /app/${BINARY}
COPY ./config /app/config
ENV LD_PRELOAD=/usr/lib/x86_64-linux-gnu/libjemalloc.so.2
CMD ["/app/${BINARY}"]