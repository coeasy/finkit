# =============================================================================
# AlphaTA — bench-vs-talib dedicated image
# =============================================================================
#
# Slimmer variant of Dockerfile that pre-installs only the toolchains needed
# for `cargo bench --bench talib_c_comparison --features talib-c` and the
# precision-parity Python check. Useful in CI to skip the .NET / WASM install
# steps when only the benchmark artifact is required.
#
# Build:
#   docker build -f scripts/bench-vs-talib.dockerfile -t AlphaTA/bench:latest .
#
# Use:
#   docker run --rm -v "$(pwd)/dist:/work/dist" AlphaTA/bench:latest
# =============================================================================

FROM rust:1.82-bookworm

ENV DEBIAN_FRONTEND=noninteractive
ENV LANG=C.UTF-8
ENV LC_ALL=C.UTF-8
ENV PATH="/root/.cargo/bin:${PATH}"

# ---- system packages -------------------------------------------------------
RUN apt-get update && apt-get install -y --no-install-recommends \
        cmake gcc g++ pkg-config libssl-dev \
        python3 python3-pip python3-venv \
        ca-certificates curl git \
    && rm -rf /var/lib/apt/lists/*

# ---- TA-Lib C library (the only extra dep vs the standard image) ----------
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

# ---- Python deps (precision parity needs numpy + AlphaTA + talib) ----------
RUN pip3 install --no-cache-dir --break-system-packages \
        numpy matplotlib

# ---- project --------------------------------------------------------------
WORKDIR /work
COPY . /work

# Warm cargo cache for the bench target only
RUN cargo build --release -p alpha-ta-core --features talib-c 2>&1 | tail -5 || true

ENTRYPOINT ["/work/scripts/bench-vs-talib.sh"]
CMD ["--precision"]
