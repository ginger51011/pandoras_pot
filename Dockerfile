FROM rust:latest as build
COPY . .

# Ensure config file exists
ARG CONFIG="config.toml"
RUN [ -z "$CONFIG" ] || touch "./$CONFIG"
RUN mv "./$CONFIG" /config

# Uncomment to use data sources for things like Markov chain generator; also uncomment in image below
# This requires that generator `data` setting points to "./data"
# ARG DATA="data.txt"
# RUN [ -z "$DATA" ] || touch "./$DATA"
# RUN mv "./$DATA" /data

RUN cargo build --release
RUN mv ./target/release/pandoras_pot /pandoras_pot

# We create a user with no root access that cannot log in
# that can run the script later
RUN adduser \
    --disabled-password \
    --gecos '' \
    --shell /sbin/nologin \
    --no-create-home \
    --home /iamadirandidontexist \
    "satan"

FROM debian:bookworm-slim

# Make our build stage user available
COPY --from=build /etc/passwd /etc/passwd
COPY --from=build /etc/group /etc/group

# Create dir for log etc.
RUN mkdir /hell
RUN chown -R satan:satan /hell
WORKDIR /hell

COPY --from=build --chown=satan:satan /pandoras_pot ./pandoras_pot
COPY --from=build --chown=satan:satan /config ./config
# COPY --from=build --chown=satan:satan /data ./data

# Run binary as non-root user; make sure to define group, otherwise
# it will be put in the root group
USER satan:satan

EXPOSE 8080
ENTRYPOINT ["./pandoras_pot", "./config"]
