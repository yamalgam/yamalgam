set shell := ["bash", "-c"]
set dotenv-load := true
toolchain := `taplo get -f rust-toolchain.toml toolchain.channel | tr -d '"'`
msrv := "1.88.0"

default:
  @just --list

# First-time project setup after cloning
bootstrap:
    #!/usr/bin/env bash
    set -euo pipefail

    # Find bootstrap hook by walking up from current directory to /
    # See: docs/tasks.md for hook documentation
    find_bootstrap_hook() {
        local dir="$PWD"
        while [[ "$dir" != "/" ]]; do
            if [[ -f "$dir/.claylo-rs.bootstrap.sh" ]]; then
                echo "$dir/.claylo-rs.bootstrap.sh"
                return 0
            fi
            dir="$(dirname "$dir")"
        done
        # Check root as well
        if [[ -f "/.claylo-rs.bootstrap.sh" ]]; then
            echo "/.claylo-rs.bootstrap.sh"
            return 0
        fi
        return 1
    }

    # Source custom bootstrap hook if found
    SKIP_HOOK_INSTALL=false
    post_bootstrap() { :; }  # no-op default
    if bootstrap_hook="$(find_bootstrap_hook)"; then
        echo "📎 Loading $bootstrap_hook..."
        source "$bootstrap_hook"
    fi

    echo "🔧 Bootstrapping yamalgam..."
    echo ""
    # Check Rust version
    installed=$(rustc --version | awk '{print $2}')
    echo "📦 Rust version: $installed (MSRV: {{msrv}})"
    if [[ "$(printf '%s\n' "{{msrv}}" "$installed" | sort -V | head -n1)" != "{{msrv}}" ]]; then
        echo "⚠️  Warning: Installed Rust $installed is older than MSRV {{msrv}}"
        echo "   Run: rustup update stable"
    fi
    echo ""

    # No git hooks configured (hook_system=none)
    echo "ℹ️  No git hooks configured"

    echo ""
    # Format (Jinja templating can produce non-canonical import ordering)
    echo "🔨 Formatting source..."
    cargo fmt --all
    echo ""
    # Build
    echo "🔨 Building project..."
    cargo build --workspace
    echo ""

    # Generate completions and man pages
    echo "📝 Generating shell completions..."
    cargo xtask completions
    echo ""
    echo "📖 Generating man pages..."
    cargo xtask man
    echo ""


    # Install site dependencies
    echo "📦 Installing site dependencies (npm)..."
    (cd site && npm install)
    echo ""


    # Configure repository settings via gh-coda
    if command -v gh &>/dev/null && gh extension list 2>/dev/null | grep -q coda; then
        echo "⚙️  Applying repository settings (gh coda)..."
        gh coda setup 2>/dev/null || echo "   (gh coda setup failed, run manually: gh coda setup)"
    else
        echo "ℹ️  gh-coda not installed. Install with: gh extension install lovelesslabs/gh-coda"
    fi
    echo ""

    # Run custom post-bootstrap hook if defined
    post_bootstrap

    echo "✅ Bootstrap complete!"
    echo ""
    echo "Next steps:"
    echo "  just check     - Run all the things (fmt, clippy, deny, test, doc-test)"
    echo "  just test      - Run tests only"
    echo ""
    echo "Try it out:"
    echo "  target/debug/yamalgam --help"

fmt:
  cargo fmt --all

clippy:
  cargo +{{toolchain}} clippy --all-targets --all-features --message-format=short -- -D warnings

fix:
  echo "Using toolchain {{toolchain}}"
  cargo +{{toolchain}} clippy --fix --allow-dirty --allow-staged -- -W clippy::all

# Check dependencies for security advisories and license compliance
deny:
  cargo deny check

test:
  cargo nextest run

test-ci:
  cargo nextest run --profile ci

doc-test:
  cargo test --doc

cov:
  @cargo llvm-cov clean --workspace
  cargo llvm-cov nextest --no-report
  @cargo llvm-cov report --html
  @cargo llvm-cov report --summary-only --json --output-path target/llvm-cov/summary.json

# Update vendored YAML Test Suite snapshot
update-test-suite:
    scripts/update-test-suite

# Generate C baseline cache (2 process spawns instead of ~700)
c-baseline:
    cd tools/fyaml-tokenize && make -q || make
    cargo run -p yamalgam-compare --bin generate_baseline

# Run YAML Test Suite compliance tests (uses cached C baseline if available)
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
    if command -v rumdl &>/dev/null; then
        rumdl ${files:-.}
    elif command -v markdownlint &>/dev/null; then
        markdownlint ${files:-"**/*.md"}
    else
        echo "Install rumdl (cargo bininstall rumdl) or markdownlint (npm i -g markdownlint-cli)"
        exit 1
    fi

# Fix markdown files
mdfix *files='':
    #!/usr/bin/env bash
    if command -v rumdl &>/dev/null; then
        rumdl --fix ${files:-.}
    elif command -v markdownlint &>/dev/null; then
        markdownlint --fix ${files:-"**/*.md"}
    else
        echo "Install rumdl (cargo bininstall rumdl) or markdownlint (npm i -g markdownlint-cli)"
        exit 1
    fi



# Run all benchmarks
bench:
  cargo xtask bench

# Run divan (wall-clock) benchmarks only
bench-divan:
  cargo bench --bench divan_benchmarks

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
  cd site && npm install

# Start site dev server
site-dev:
  cd site && npm run dev

# Build site for production
site-build:
  cd site && npm run build

# Preview production build
site-preview:
  cd site && npm run preview

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
    cargo deny check
    echo "✅ Dependencies OK"
    # Build release
    echo ""
    echo "🔨 Building release..."
    cargo build -p yamalgam --release
    echo "✅ Release build succeeded"
    echo ""
    echo "✅ All pre-release checks passed!"

# Build release binary
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

# Upgrade Cargo.toml to latest compatible versions
upgrade:
    cargo upgrade
    cargo update --workspace

# The nuclear option: upgrade to latest incompatible versions (breaking changes)
upgrade-breaking:
    cargo upgrade --incompatible
    cargo update --workspace

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

