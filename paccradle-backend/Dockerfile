# syntax=docker/dockerfile:1
# Backend build image with OpenCV + clang from day 1 (CLAUDE.md §14, §5.3).
#
#   toolchain  Rust + OpenCV dev libraries + libclang. Also used for local dev:
#              scripts/in-docker.sh runs any cargo command inside it.
#   build      Compiles the whole workspace in release mode.
#   runtime    Small Debian image with only the API binary.

FROM rust:1.99-trixie AS toolchain
RUN apt-get update \
    && apt-get install -y --no-install-recommends clang libclang-dev libopencv-dev pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && rustup component add rustfmt clippy
WORKDIR /app

FROM toolchain AS build
# Queries are checked against the committed .sqlx/ cache: building needs no database.
ENV SQLX_OFFLINE=true
COPY . .
RUN cargo build --release --workspace --locked

FROM debian:trixie-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
# The OpenCV runtime libraries get added here once the API runs capture jobs with the
# vision crate (M1-D5). Debian's libopencv-imgcodecs410 pulls in GDAL, OpenEXR and TIFF:
# weigh a slimmer OpenCV build in M1-G1.
COPY --from=build /app/target/release/paccradle-backend /usr/local/bin/paccradle-backend
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/paccradle-backend"]
