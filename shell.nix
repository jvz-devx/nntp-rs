{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "nntp-rs-dev";

  buildInputs = with pkgs; [
    # Node.js
    nodejs_24
    nodePackages.npm

    # Rust toolchain (stable from nixpkgs)
    rustc
    cargo
    rust-analyzer
    clippy
    rustfmt

    # Build essentials
    pkg-config
    bash

    # For faster linking
    mold

    # Cargo extensions
    cargo-watch      # Auto-rebuild: cargo watch -x check
    cargo-edit       # cargo add/rm/upgrade
    cargo-nextest    # Faster test runner
    cargo-deny       # Audit dependencies
    cargo-outdated   # Check for outdated deps
    cargo-flamegraph # Performance profiling
    cargo-expand     # Expand macros

    # Debugging
    gdb
  ];

  # Environment variables
  RUST_BACKTRACE = "1";
  RUST_LOG = "debug";

  # Use mold for faster linking
  RUSTFLAGS = "-C link-arg=-fuse-ld=mold";

  # For rust-analyzer
  RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

  shellHook = ''
    # Ensure npm global installs go to a local directory
    export NPM_CONFIG_PREFIX="$HOME/.npm-global"
    export PATH="$NPM_CONFIG_PREFIX/bin:$PATH"
    mkdir -p "$NPM_CONFIG_PREFIX"

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo " nntp-rs development environment"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "Rust: $(rustc --version)"
    echo "Node: $(node --version)"
    echo "npm:  $(npm --version)"
    echo ""
    echo "Commands:"
    echo "  cargo build             Build the library"
    echo "  cargo test              Run tests"
    echo "  cargo nextest run       Run tests (faster)"
    echo "  cargo watch -x check    Auto-check on save"
    echo "  cargo clippy            Run lints"
    echo "  cargo fmt               Format code"
    echo "  cargo doc --open        Build and view docs"
    echo ""
    echo "Live tests (set NNTP_* env vars first):"
    echo "  cargo test --features live-tests"
    echo ""
  '';
}
