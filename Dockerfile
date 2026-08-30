# =============================================================================
# Finkit — true one-click build image
# =============================================================================
#
# Pre-installs every toolchain required to build the 7-language usage
# packages from a clean checkout. Lets `docker run` produce a complete
# dist/ tree with one command (no host setup besides Docker).
#
# Toolchains:
#   * cargo / rustc (stable)            — Rust core
#   * maturin + python3 + pip           — Python wheel
#   * node 20 + npm                     — Node tgz
#   * openjdk-17 + maven                — Java jar
#   * go 1.22                           — Go package
#   * cmake + gcc + pkg-config          — C/C++ install
#   * dotnet-sdk-8.0                    — .NET nupkg
#   * wasm-pack + rustup target wasm32  — WASM
#
# Build:
#   docker build -t finkit/builder:latest .
#
# Use (mounts host dist/ so artifacts survive container exit):
#   docker run --rm -v "$(pwd)/dist:/work/dist" finkit/builder:latest
#   docker run --rm -v "$(pwd)/dist:/work/dist" finkit/builder:latest --bench-talib
# =============================================================================

FROM rust:1.82-bookworm

ENV DEBIAN_FRONTEND=noninteractive
ENV LANG=C.UTF-8
ENV LC_ALL=C.UTF-8
ENV PATH="/root/.cargo/bin:${PATH}"

# ---- system packages -------------------------------------------------------
RUN apt-get update && apt-get install -y --no-install-recommends \
        cmake gcc g++ pkg-config libssl-dev zip unzip \
        openjdk-17-jdk maven \
        golang-go \
        python3 python3-pip python3-venv \
        ca-certificates curl git xz-utils \
    && rm -rf /var/lib/apt/lists/*

# ---- Node.js 20 ------------------------------------------------------------
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/* \
    && node --version \
    && npm --version

# ---- .NET 8 SDK ------------------------------------------------------------
RUN curl -fsSL https://dot.net/v1/dotnet-install.sh | bash -s -- \
        --channel 8.0 --install-dir /usr/share/dotnet \
    && ln -s /usr/share/dotnet/dotnet /usr/local/bin/dotnet \
    && dotnet --version

# ---- TA-Lib C library (needed for --bench-talib) ---------------------------
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential wget \
    && wget -q https://github.com/ta-lib/ta-lib/releases/download/v0.6.4/ta-lib-0.6.4-src.tar.gz \
       -O /tmp/ta-lib.tar.gz \
    && mkdir -p /opt/ta-lib && tar -xzf /tmp/ta-lib.tar.gz -C /opt/ta-lib --strip-components=1 \
    && (cd /opt/ta-lib && ./configure --prefix=/usr/local && make -j"$(nproc)" && make install) \
    && ldconfig \
    && rm -rf /tmp/ta-lib.tar.gz \
    && rm -rf /var/lib/apt/lists/* \
    && pkg-config --modversion ta-lib

# ---- WASM toolchain --------------------------------------------------------
RUN cargo install wasm-pack --locked \
    && rustup target add wasm32-unknown-unknown

# ---- Python build deps ----------------------------------------------------
RUN pip3 install --no-cache-dir --break-system-packages \
        maturin pytest numpy

# ---- project --------------------------------------------------------------
WORKDIR /work
COPY . /work

# Pre-build the core crate to warm the cargo cache; subsequent
# `docker run --rm -v ./dist:/work/dist …` invocations reuse the layer.
RUN cargo build --release -p finkit 2>&1 | tail -5 || true

# ---- entrypoint -----------------------------------------------------------
# `./build-usage.sh` (or `.ps1`) lives at /work, exposed as the entrypoint.
RUN chmod +x /work/build-usage.sh
ENTRYPOINT ["/work/build-usage.sh"]
CMD ["--no-bundle"]
