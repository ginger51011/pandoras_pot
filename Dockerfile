FROM rust:1.75 as build
COPY . .

# Ensure config file exists
ARG CONFIG="config.toml"
RUN [ -z "$CONFIG" ] || touch "./$CONFIG"
RUN mv "./$CONFIG" /config

RUN cargo build --release
RUN mv ./target/release/pandoras_pot /pandoras_pot

FROM debian:bookworm

COPY --from=build /pandoras_pot /pandoras_pot
COPY --from=build /config /config

EXPOSE 8080
ENTRYPOINT ["/pandoras_pot", "/config"]
