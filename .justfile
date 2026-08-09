set shell := ["bash", "-c"]
set dotenv-load := true
toolchain := `taplo get -f rust-toolchain.toml toolchain.channel | tr -d '"'`
msrv := "1.89.0"

default:
  @just --list


fmt:
  cargo fmt --all -- --config-path .config/rustfmt.toml

clippy:
  cargo +{{toolchain}} clippy --all-targets --all-features --message-format=short -- -D warnings

fix:
  echo "Using toolchain {{toolchain}}"
  cargo +{{toolchain}} clippy --fix --allow-dirty --allow-staged -- -W clippy::all

# Check dependencies for security advisories and license compliance
deny:
  cargo deny --all-features --locked --config .config/deny.toml check
  cargo deny --manifest-path fuzz/Cargo.toml --all-features --locked --config .config/deny.toml check

# Compiler-derived unsafe census for the shipped yg dependency graph.
unsafe-census:
  #!/usr/bin/env bash
  set -euo pipefail
  CARGO_TARGET_DIR=target/unsafe-census \
    CARGO_BUILD_JOBS=1 \
    RUSTC_WRAPPER= \
    RUSTFLAGS="--force-warn unsafe-code" \
    cargo check -p yamalgam --locked --message-format=short

test:
  cargo nextest run --workspace --all-features --cargo-quiet

test-ci:
  cargo nextest run --workspace --all-features --profile ci

doc-test:
  cargo test --doc --all-features

# Run executable examples embedded in the yg user guides.
docs-examples:
  scripts/check-yg-doc-examples

# Run executable, prose, Markdown, and offline-link checks for yg docs.
docs-check: docs-examples
  #!/usr/bin/env bash
  set -euo pipefail
  docs=(README.md docs/README.md docs/yg/*.md)
  rumdl check "${docs[@]}"
  lychee --offline "${docs[@]}"
  for doc in docs/README.md docs/yg/*.md; do
    bito readability "$doc" --max-grade 8
  done

cov:
  @cargo llvm-cov clean --workspace

  cargo llvm-cov nextest --no-report
  @cargo llvm-cov report --html
  @cargo llvm-cov report --summary-only --json --output-path target/llvm-cov/summary.json

# Update vendored YAML Test Suite snapshot
update-test-suite:
    scripts/update-test-suite

# Run YAML Test Suite compliance tests
test-compliance:
    cargo nextest run -p yamalgam-compare --test compliance

check: fmt clippy deny test doc-test

# Watch for changes and run tests (requires cargo-watch)
watch *args='':
  cargo watch -x 'nextest run {{args}}'

# Watch and run clippy on changes
watch-clippy:
  cargo watch -x 'clippy --all-targets --all-features -- -D warnings'

# Lint markdown files
mdlint *files='':
    #!/usr/bin/env bash
    files="{{files}}"
    if command -v rumdl &>/dev/null; then
        rumdl check ${files:-.}
    elif command -v markdownlint &>/dev/null; then
        markdownlint ${files:-"**/*.md"}
    else
        echo "Install rumdl (cargo bininstall rumdl) or markdownlint (npm i -g markdownlint-cli)"
        exit 1
    fi

# Fix markdown files
mdfix *files='':
    #!/usr/bin/env bash
    files="{{files}}"
    if command -v rumdl &>/dev/null; then
        rumdl check --fix ${files:-.}
    elif command -v markdownlint &>/dev/null; then
        markdownlint --fix ${files:-"**/*.md"}
    else
        echo "Install rumdl (cargo bininstall rumdl) or markdownlint (npm i -g markdownlint-cli)"
        exit 1
    fi

# Run all benchmarks
bench:
  cargo xtask bench

# Run CLI (hyperfine) benchmarks only
bench-cli:
  ./scripts/bench-cli.sh

# Run comparative benchmarks (yamalgam vs peers)
bench-comparative:
  cargo bench -p yamalgam-bench

# Run scanner self-benchmarks
bench-scanner:
  cargo bench -p yamalgam-scanner

# Run parser self-benchmarks
bench-parser:
  cargo bench -p yamalgam-parser


# Run a single fuzz target (default: scanner bytes, 60s)
fuzz target='fuzz_scanner_bytes' duration='60':
    cargo +nightly fuzz run {{target}} -- -max_total_time={{duration}}

# Run all fuzz targets (default: 60s each)
fuzz-all duration='60':
    #!/usr/bin/env bash
    set -euo pipefail
    for target in fuzz_scanner_bytes fuzz_parser_bytes fuzz_scanner_structured \
                  fuzz_parser_structured fuzz_limits fuzz_differential; do
        echo "=== Fuzzing $target for {{duration}}s ==="
        cargo +nightly fuzz run "$target" -- -max_total_time={{duration}} || true
    done

# Run all fuzz targets for 1 hour each
fuzz-long:
    just fuzz-all 3600

# Minimize fuzz corpus (remove redundant inputs)
fuzz-corpus-min:
    cargo +nightly fuzz cmin fuzz_scanner_bytes
    cargo +nightly fuzz cmin fuzz_parser_bytes

# Seed fuzz corpus from YAML Test Suite
fuzz-seed:
    scripts/seed-fuzz-corpus

# Install site dependencies
site-install:
  cd site && pnpm install

# Start site dev server
site-dev:
  cd site && pnpm run dev

# Build site for production
site-build:
  cd site && pnpm run build

# Preview production build
site-preview:
  cd site && pnpm run preview

# Add a new crate to the workspace
add-crate *ARGS:
    scripts/add-crate {{ARGS}}


# Pre-release validation
release-check:
    #!/usr/bin/env zsh
    set -euo pipefail
    echo "🔍 Running pre-release checks..."
    echo ""
    # Check for uncommitted changes
    if ! git diff --quiet HEAD 2>/dev/null; then
        echo "❌ Uncommitted changes detected"
        git status --short
        exit 1
    fi
    echo "✅ Working directory clean"
    # Run full test suite
    echo ""
    echo "🧪 Running tests..."
    cargo nextest run
    echo "✅ Tests passed"
    # Clippy
    echo ""
    echo "📎 Running clippy..."
    cargo clippy --all-targets --all-features -- -D warnings
    echo "✅ Clippy clean"
    # Security/license check
    echo ""
    echo "🔐 Running cargo-deny..."
    just deny
    echo "✅ Dependencies OK"
    # Build release
    echo ""
    echo "🔨 Building release..."
    cargo build -p yamalgam --release
    echo "✅ Release build succeeded"
    echo ""
    echo "✅ All pre-release checks passed!"

# Build release
build-release:
  cargo build -p yamalgam --release

zip:
  git archive --format=zip --output=../yamalgam-{{datetime('-%Y-%m-%d_%H%M')}}.zip HEAD

# Check for outdated dependencies (root only, no transitive noise)
outdated:
    cargo outdated --workspace --root-deps-only

# Safe update: respects semver constraints, only touches Cargo.lock
update:
    cargo update --workspace --verbose
    cargo update -p serde-saphyr --precise 1.0.0
    cargo update -p granit-parser --precise 1.0.0

# Upgrade Cargo.toml to latest compatible versions
upgrade:
    cargo upgrade
    just update

# The nuclear option: upgrade to latest incompatible versions (breaking changes)
upgrade-breaking:
    cargo upgrade --incompatible
    just update

# See what WOULD update without doing it
check-updates:
    cargo update --workspace --dry-run

# Full refresh: update, test, clippy
refresh: update
    cargo test --workspace

    cargo clippy --workspace -- -D warnings

# Monthly maintenance: upgrade, test everything
monthly: upgrade
    cargo test --workspace

    cargo clippy --workspace -- -D warnings
    cargo build --workspace --release

# Show why a specific package version is stuck
[private]
why pkg:
    cargo tree -i {{pkg}}

# Update system-level cargo tools
system-update:
    cargo install-update -al


# =============================================================================
# Release Management
# =============================================================================

# Generate CHANGELOG.md using git-cliff
changelog:
    git cliff --output CHANGELOG.md

# Preview what the next changelog entry will look like
changelog-preview:
    git cliff --unreleased --strip all

# Bump version (requires git-cliff with bump feature)
bump *args='':
    #!/usr/bin/env bash
    set -euo pipefail
    # Get the next version from git-cliff
    next=$(git cliff --bumped-version {{args}})
    echo "Next version: $next"
    # Update Cargo.toml versions

    cargo set-version --workspace "${next#v}"
    # Generate changelog
    git cliff --tag "$next" --output CHANGELOG.md
    echo "Updated CHANGELOG.md and Cargo.toml"
    echo ""
    echo "Next steps:"
    echo "  git add -A && git commit -m 'chore(release): prepare $next'"
    echo "  git tag $next"
    echo "  git push && git push --tags"

# Create a release tag (runs pre-release checks first)
release version:
    #!/usr/bin/env bash
    set -euo pipefail
    version="{{version}}"
    if [[ ! "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
        echo "❌ Invalid version format: $version"
        echo "   Expected: v1.2.3 or v1.2.3-beta.1"
        exit 1
    fi
    echo "🔍 Running pre-release checks..."
    just release-check
    echo ""
    echo "📝 Updating changelog..."
    git cliff --tag "$version" --output CHANGELOG.md
    # Update Cargo.toml
    cargo set-version --workspace "${version#v}"
    echo ""
    echo "🏷️  Creating release commit and tag..."
    git add -A
    git commit -m "chore(release): prepare $version"
    git tag -a "$version" -m "Release $version"
    echo ""
    echo "✅ Release $version prepared!"
    echo ""
    echo "To publish:"
    echo "  git push && git push --tags"
