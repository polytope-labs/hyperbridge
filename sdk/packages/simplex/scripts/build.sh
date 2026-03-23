#!/usr/bin/env bash
# @hyperbridge/simplex: submodule protos → protoc → src/proto, then tsup (unless --codegen-only).
#
# Requires: git (optional), protoc, pnpm install (ts-proto in node_modules).
#
# Usage:
#   ./scripts/build.sh              # full build (codegen + tsup)
#   ./scripts/build.sh --codegen-only   # protoc only (for test/lint/cli before tsup)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PKG_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PKG_ROOT"

SUBMODULE_PATH="sdk/packages/simplex/proto/mpcvaultapis"

if REPO_ROOT="$(git -C "$PKG_ROOT" rev-parse --show-toplevel 2>/dev/null)"; then
	if [ ! -f "$PKG_ROOT/proto/mpcvaultapis/mpcvault/platform/v1/api.proto" ]; then
		echo "Initializing mpcvaultapis submodule ($SUBMODULE_PATH)..."
		git -C "$REPO_ROOT" submodule update --init --recursive "$SUBMODULE_PATH"
	fi
else
	echo "Warning: not inside a git repository; skipping submodule init."
fi

./scripts/generate-proto.sh

if [ "${1:-}" = "--codegen-only" ]; then
	exit 0
fi

echo "Running tsup..."
pnpm exec tsup
