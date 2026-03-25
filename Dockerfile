# ──────────────────────────────────────────────────────────────
# OptiFlow — Multi-stage Dockerfile
# Stage 1: Compile the Rust web server binary
# Stage 2: Build the React frontend into static files
# Stage 3: Tiny runtime image with just the binary + static files
#
# Source code only exists in build stages — the final image
# contains ONLY compiled binaries and minified JS.
# ──────────────────────────────────────────────────────────────

# ── Stage 1: Rust builder ────────────────────────────────────
FROM rust:1-bookworm AS rust-builder

# cmake is needed by the HiGHS solver crate to compile its C++ core
# clang + libclang-dev are needed by bindgen to generate FFI bindings
RUN apt-get update && apt-get install -y cmake clang libclang-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy Cargo manifest + lockfile first (dependency caching layer)
COPY src-tauri/Cargo.toml src-tauri/Cargo.lock* ./

# Create minimal stubs so cargo can resolve the dependency tree
# This lets Docker cache the expensive dependency download/compile step
RUN mkdir -p src && echo "pub mod models; pub mod solver; pub mod state; pub mod validator;" > src/lib.rs \
    && mkdir -p src/models && touch src/models/mod.rs \
    && touch src/solver.rs src/state.rs src/validator.rs \
    && echo "fn main() {}" > src/web_server.rs \
    && echo "fn main() {}" > build.rs

# Pre-build dependencies (cached unless Cargo.toml changes)
RUN cargo build --release --bin optiflow-web --no-default-features 2>/dev/null || true

# Now copy the real source code
COPY src-tauri/src ./src
COPY src-tauri/build.rs ./build.rs

# Build the web server binary (--no-default-features skips Tauri)
RUN cargo build --release --bin optiflow-web --no-default-features

# ── Stage 2: Node builder ────────────────────────────────────
FROM node:22-slim AS node-builder

WORKDIR /app

# Copy package files first (dependency caching layer)
COPY package.json package-lock.json* ./
RUN npm ci

# Copy frontend source and build
COPY index.html tsconfig.json tsconfig.node.json vite.config.ts ./
COPY src/ ./src/
COPY public/ ./public/

# Build produces the dist/ folder with minified JS/CSS/HTML
RUN npx tsc && npx vite build

# ── Stage 3: Runtime (tiny final image) ─────────────────────
FROM debian:bookworm-slim AS runtime

# libgomp is needed at runtime by HiGHS solver
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libgomp1 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy ONLY the compiled binary — no source code!
COPY --from=rust-builder /build/target/release/optiflow-web .

# Copy ONLY the minified frontend bundle — no source code!
COPY --from=node-builder /app/dist ./dist

# Railway/Render inject PORT env var; default to 3000
ENV PORT=3000
EXPOSE 3000

CMD ["./optiflow-web"]
