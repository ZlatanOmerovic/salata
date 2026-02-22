# Salata — main build + all runtimes
#
# Builds salata binaries from source and installs all supported runtimes.
# Runtime paths match config.toml defaults.

FROM rust:1.88-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY config.toml ./
COPY errors/ errors/

RUN cargo build --release

# ---------------------------------------------------------------------------
# Runtime image
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim

# System deps
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Python 3
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 \
    python3-minimal \
    && ln -sf /usr/bin/python3 /usr/bin/python \
    && rm -rf /var/lib/apt/lists/*

# Ruby
RUN apt-get update && apt-get install -y --no-install-recommends \
    ruby \
    && rm -rf /var/lib/apt/lists/*

# Node.js (via NodeSource for a recent version)
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

# TypeScript (ts-node + tsx)
RUN npm install -g typescript ts-node tsx

# PHP + php-cgi
RUN apt-get update && apt-get install -y --no-install-recommends \
    php-cgi \
    php-cli \
    && rm -rf /var/lib/apt/lists/*

# Shells: bash is already present, install dash (sh is usually dash on Debian)
RUN apt-get update && apt-get install -y --no-install-recommends \
    dash \
    && rm -rf /var/lib/apt/lists/*

# Verify runtime paths match config.toml
RUN python3 --version \
    && ruby --version \
    && node --version \
    && ts-node --version \
    && php-cgi --version \
    && bash --version | head -1 \
    && dash -c 'echo dash ok'

# Ensure ts-node is accessible at /usr/bin/ts-node (config.toml path)
RUN TS_NODE_PATH="$(which ts-node)" && \
    if [ "$TS_NODE_PATH" != "/usr/bin/ts-node" ]; then \
        ln -sf "$TS_NODE_PATH" /usr/bin/ts-node; \
    fi

# Copy salata binaries
COPY --from=builder /build/target/release/salata /usr/local/bin/salata
COPY --from=builder /build/target/release/salata-cgi /usr/local/bin/salata-cgi
COPY --from=builder /build/target/release/salata-fastcgi /usr/local/bin/salata-fastcgi
COPY --from=builder /build/target/release/salata-server /usr/local/bin/salata-server

# Copy default config and error pages
COPY config.toml /usr/local/bin/config.toml
COPY errors/ /usr/local/bin/errors/

# Create logs directory
RUN mkdir -p /usr/local/bin/logs

WORKDIR /srv/salata

EXPOSE 3000

CMD ["salata-server", "--config", "/usr/local/bin/config.toml", "--port", "3000", "/srv/salata"]
