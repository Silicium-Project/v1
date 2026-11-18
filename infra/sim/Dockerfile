# Nœud Silicium de simulation : le binaire Linux (dist/linux-amd64/silicium-node) est le seul contexte.
# la configuration est montée au démarrage. Construction depuis la racine du dépôt :
#   docker build -f infra/sim/Dockerfile -t silicium-node:sim ../../dist/linux-amd64
FROM ubuntu:24.04

RUN useradd --system --home-dir /data --shell /usr/sbin/nologin silicium \
 && mkdir -p /data && chown silicium:silicium /data

COPY silicium-node /usr/local/bin/silicium-node
COPY sil-raytracer-block sil-sandbox-launch sil-pdf-block.wasm /usr/local/bin/
# Runtime ONNX (embedding) : cherché dans ../lib/Silicium à côté du binaire (cli.rs, runtime_library).
RUN mkdir -p /usr/local/lib/Silicium
COPY libonnxruntime.so /usr/local/lib/Silicium/libonnxruntime.so

USER silicium
VOLUME ["/data"]
ENTRYPOINT ["/usr/local/bin/silicium-node"]
