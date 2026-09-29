#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$SCRIPT_DIR/../../.."
BUILD_DIR="$SCRIPT_DIR/build"
GEM_DIRECTORY="$BUILD_DIR/gems"
RUBY_INTERPRETER=""
YJIT=false
UNIFFI_DIR=""
INCLUDE=""
PUBLISH=false
RUNNER_ARGS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --ruby)
            RUBY_INTERPRETER="${2:-}"
            shift 2
            ;;
        --yjit)
            YJIT=true
            shift
            ;;
        --uniffi-dir)
            UNIFFI_DIR="${2:-}"
            shift 2
            ;;
        --include)
            INCLUDE="${2:-}"
            shift 2
            ;;
        --publish)
            PUBLISH=true
            shift
            ;;
        *)
            RUNNER_ARGS+=("$1")
            shift
            ;;
    esac
done

SELECTED_RUBY="${RUBY_INTERPRETER:-ruby}"
if ! command -v "$SELECTED_RUBY" >/dev/null 2>&1; then
    echo "Missing ruby interpreter: $SELECTED_RUBY" >&2
    exit 1
fi

MODE=interpreter
if [[ "$YJIT" == true ]]; then
    MODE=yjit
fi
RESULTS_DIR="$BUILD_DIR/results/ruby/$MODE"
mkdir -p "$RESULTS_DIR"
rm -f "$RESULTS_DIR/results.json" "$RESULTS_DIR/benchmark_run.json"

"$ROOT_DIR/benchmarks/generated/boltffi/build-ruby.sh"

# Only the Ruby install's own gems and the benchmark gem are visible, so gems
# in the user's gem directory cannot change the result.
rm -rf "$GEM_DIRECTORY"
export GEM_HOME="$GEM_DIRECTORY"
GEM_PATH="$GEM_HOME:$("$SELECTED_RUBY" -e 'print Gem.default_dir')"
export GEM_PATH

GEM_SOURCE="$ROOT_DIR/benchmarks/generated/boltffi/dist/ruby"
(cd "$GEM_SOURCE" && "$SELECTED_RUBY" -S gem build demo.gemspec --output "$BUILD_DIR/demo.gem")
"$SELECTED_RUBY" -S gem install --local --no-document "$BUILD_DIR/demo.gem"

if [[ -n "$UNIFFI_DIR" ]] && ! "$SELECTED_RUBY" -e 'require "ffi"' >/dev/null 2>&1; then
    "$SELECTED_RUBY" -S gem install --no-document ffi
fi

RUNNER_COMMAND=("$SELECTED_RUBY")
if [[ "$YJIT" == true ]]; then
    RUNNER_COMMAND+=(--yjit)
fi
RUNNER_COMMAND+=("$SCRIPT_DIR/bench.rb" --output "$RESULTS_DIR/results.json")
if [[ -n "$UNIFFI_DIR" ]]; then
    RUNNER_COMMAND+=(--uniffi-dir "$UNIFFI_DIR")
fi
if [[ -n "$INCLUDE" ]]; then
    RUNNER_COMMAND+=(--include "$INCLUDE")
fi
if (( ${#RUNNER_ARGS[@]} > 0 )); then
    RUNNER_COMMAND+=("${RUNNER_ARGS[@]}")
fi

"${RUNNER_COMMAND[@]}"

python3 "$ROOT_DIR/benchmarks/scripts/ruby_bench_to_run.py" \
    --results "$RESULTS_DIR/results.json" \
    --output "$RESULTS_DIR/benchmark_run.json" \
    --profile release \
    --runner-command "${RUNNER_COMMAND[*]}"

if [[ "$PUBLISH" == true ]]; then
    "$ROOT_DIR/benchmarks/scripts/publish-benchmark-runs.sh" "$RESULTS_DIR/benchmark_run.json"
fi
