FROM scratch
WORKDIR /
# plugin.wasm must be present in the Docker build context
COPY plugin.wasm /plugin.wasm
