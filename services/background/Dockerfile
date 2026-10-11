FROM rust:1-bookworm AS build
WORKDIR /source
COPY . .
RUN cargo build -p compose-background-service --release --locked

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /source/target/release/compose-background-service /compose-background-service
ENV COMPOSE_SERVICE_BIND=0.0.0.0:8787
EXPOSE 8787
ENTRYPOINT ["/compose-background-service"]
