FROM node:24.15.0-bookworm-slim AS web
WORKDIR /web
COPY samples/crm/web/package*.json ./
COPY samples/crm/web/vendor ./vendor
RUN npm ci
COPY samples/crm/web/ ./
RUN npm run build

FROM rust:1.90.0-bookworm AS backend
WORKDIR /source
ARG CARGO_BUILD_JOBS=4
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates
COPY samples/crm/server ./samples/crm/server
COPY samples/crm/modules ./samples/crm/modules
COPY contracts ./contracts
# Keep the source workspace self-contained in the build stage.
COPY dev ./dev
COPY docs ./docs
COPY LICENSE README.md .env.example .gitignore ./
COPY samples/crm/web ./samples/crm/web
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/source/target \
    cargo +1.90.0 build --jobs "$CARGO_BUILD_JOBS" --locked --release -p bqatlas-server && cp target/release/bqatlas-server /usr/local/bin/bqatlas-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/* && useradd --system --uid 10001 atlas
WORKDIR /app
COPY --from=backend /usr/local/bin/bqatlas-server /app/bqatlas-server
COPY --from=web /web/dist/crm/browser /app/web
COPY LICENSE samples/crm/web/SOURCE-NOTICE.md docs/DEPENDENCY-LICENSES.json /app/licenses/
COPY --from=web /web/dist/crm/3rdpartylicenses.txt /app/licenses/frontend-thirdparty.txt
USER atlas
ENV BQATLAS_ENVIRONMENT=Production BQATLAS_LISTEN=0.0.0.0:5201 BQATLAS_WEB_ROOT=/app/web
EXPOSE 5201
ENTRYPOINT ["/app/bqatlas-server"]
CMD ["serve"]
