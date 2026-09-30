#!/usr/bin/env bash
# @hyperbridge/simplex: submodule protos → protoc → src/proto, then tsup (unless --codegen-only).
#
# Requires: git (optional), protoc, pnpm install (ts-proto in node_modules).
# For DTS, @hyperbridge/sdk must be built first so dist/node/index.d.ts includes latest
# ChainConfigService (run from repo sdk root: pnpm --filter=@hyperbridge/sdk build).
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

# The store imports `node:sqlite`, which — unlike `fs` or `path` — has no unprefixed
# alias. tsup 8 strips `node:` prefixes unless told not to (see KEEP_NODE_PROTOCOL in
# tsup.config.ts), and a stripped import is an ERR_MODULE_NOT_FOUND the moment the
# binary starts. No test catches it: the suite runs from source, where nothing rewrites
# imports. So the bundle itself is checked.
echo "Verifying the node: protocol survived bundling..."
if grep -rnE '(from|require\()[[:space:]]*"sqlite"' dist --include='*.js' --include='*.cjs'; then
	echo "ERROR: the bundle imports bare \"sqlite\" — the node: prefix was stripped." >&2
	echo "       Set removeNodeProtocol: false for that entry in tsup.config.ts." >&2
	exit 1
fi
if ! grep -rqE '"node:sqlite"' dist --include='*.js' --include='*.cjs'; then
	echo "ERROR: no node:sqlite import in dist — the persistent store cannot work." >&2
	exit 1
fi

# The web UI must build after tsup: tsup's clean:true wipes dist/, including dist/ui.
echo "Building web UI (vite)..."
pnpm exec vite build --config ui/vite.config.ts
