# TODO: base image + build for adguard. Mirror jellyfin/Dockerfile conventions.
FROM debian:12-slim
LABEL org.opencontainers.image.source="https://github.com/argyle-labs/adguard"
EXPOSE 3000
