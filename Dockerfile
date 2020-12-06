FROM getsentry/sentry-cli:1 AS sentry-cli
FROM rust:1.48 as builder
WORKDIR /usr/src/oxideoverflow

ARG SENTRY_AUTH_TOKEN
ENV SENTRY_AUTH_TOKEN=$SENTRY_AUTH_TOKEN
ARG SENTRY_ORG
ENV SENTRY_ORG=$SENTRY_ORG
ARG SENTRY_PROJECT
ENV SENTRY_PROJECT=$SENTRY_PROJECT

COPY --from=sentry-cli /bin/sentry-cli /bin/sentry-cli
COPY . .
RUN cargo install --path .

RUN sentry-cli --version \
    && sentry-cli \
       upload-dif \
       --include-sources \
       .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y openssl && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/oxideoverflow /usr/local/bin/oxideoverflow

ARG ENVIRONMENT
ENV SENTRY_ENVIRONMENT=$ENVIRONMENT
RUN echo Environment: $ENVIRONMENT

ARG VERSION
ENV SENTRY_VERSION=$VERSION
RUN echo Version: $VERSION

CMD ["oxideoverflow"]