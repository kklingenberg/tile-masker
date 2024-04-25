FROM busybox AS download
ARG REPO=https://github.com/kklingenberg/tile-masker
ARG VERSION
RUN test -n "${VERSION}" && \
    wget "${REPO}/releases/download/${VERSION}/tile-masker" -O /tile-masker && \
    chmod +x /tile-masker

FROM scratch
COPY --from=download /tile-masker /usr/bin/tile-masker
ENTRYPOINT ["/usr/bin/tile-masker"]
