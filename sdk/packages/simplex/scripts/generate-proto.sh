#!/usr/bin/env bash
# Generate TypeScript gRPC client stubs from mpcvaultapis protos.
#
# Prerequisites:
#   - protoc (Protocol Buffers compiler) installed
#   - npm install (ts-proto, @grpc/grpc-js, @bufbuild/protobuf)
#
# Proto sources: proto/mpcvaultapis (submodule or clone — see src/services/wallet/migration.md)
#
# Usage (from this package root):
#   ./scripts/generate-proto.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

PROTO_DIR="$ROOT_DIR/proto/mpcvaultapis"
OUT_DIR="$ROOT_DIR/src/proto"
PLUGIN="$ROOT_DIR/node_modules/.bin/protoc-gen-ts_proto"

if [ ! -d "$PROTO_DIR" ]; then
	echo "Error: Proto source directory not found at $PROTO_DIR"
	echo "Run: git submodule update --init  (or: git clone --depth 1 https://github.com/mpcvault/mpcvaultapis.git \"$PROTO_DIR\")"
	exit 1
fi

if [ ! -f "$PLUGIN" ]; then
	echo "Error: ts-proto plugin not found at $PLUGIN. Run: npm install"
	exit 1
fi

# pull_request_target runs workflow YAML from the base branch, so PR-only "apt install protobuf-compiler"
# steps may not run while this script (from the PR) still executes. Bootstrap protoc on CI when missing.
ensure_protoc() {
	if command -v protoc >/dev/null 2>&1; then
		return 0
	fi
	if [ "${CI:-}" != "true" ]; then
		echo "Error: protoc not found. Install it (e.g. apt install protobuf-compiler, brew install protobuf)."
		exit 1
	fi
	local ver="${PROTOC_CI_VERSION:-25.3}"
	local dest="$ROOT_DIR/.protoc-ci"
	local zip
	case "$(uname -s)/$(uname -m)" in
		Linux/x86_64) zip="protoc-${ver}-linux-x86_64.zip" ;;
		Linux/aarch64 | Linux/arm64) zip="protoc-${ver}-linux-aarch_64.zip" ;;
		Darwin/x86_64) zip="protoc-${ver}-osx-x86_64.zip" ;;
		Darwin/arm64) zip="protoc-${ver}-osx-aarch_64.zip" ;;
		*)
			echo "Error: CI protoc bootstrap not supported for $(uname -s)/$(uname -m)"
			exit 1
			;;
	esac
	if [ ! -x "$dest/bin/protoc" ]; then
		echo "CI: downloading protoc ${ver} (${zip})..."
		mkdir -p "$dest"
		curl -fsSL "https://github.com/protocolbuffers/protobuf/releases/download/v${ver}/${zip}" -o /tmp/protoc-ci.zip
		unzip -q -o /tmp/protoc-ci.zip -d "$dest"
		rm -f /tmp/protoc-ci.zip
	fi
	export PATH="$dest/bin:$PATH"
}
ensure_protoc

echo "Cleaning $OUT_DIR..."
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

echo "Generating TypeScript from proto files..."
# Locate well-known proto includes (wrappers.proto, descriptor.proto, etc.)
WELL_KNOWN_PROTOS=""
for candidate in "$(dirname "$(command -v protoc)" 2>/dev/null)/../include" /usr/include /usr/local/include; do
	if [ -f "$candidate/google/protobuf/wrappers.proto" ]; then
		WELL_KNOWN_PROTOS="$candidate"
		break
	fi
done
if [ -z "$WELL_KNOWN_PROTOS" ]; then
	echo "Error: Could not find Google well-known proto includes (google/protobuf/wrappers.proto)"
	exit 1
fi

protoc \
	--plugin="protoc-gen-ts_proto=$PLUGIN" \
	--ts_proto_out="$OUT_DIR" \
	--ts_proto_opt=outputServices=grpc-js \
	--ts_proto_opt=esModuleInterop=true \
	--ts_proto_opt=env=node \
	--ts_proto_opt=useExactTypes=false \
	--ts_proto_opt=forceLong=string \
	-I="$PROTO_DIR" \
	-I="$WELL_KNOWN_PROTOS" \
	"$PROTO_DIR"/mpcvault/platform/v1/api.proto \
	"$PROTO_DIR"/mpcvault/platform/v1/error.proto

echo "Done. Generated files:"
find "$OUT_DIR" -name "*.ts" | sort | sed "s|^|  |"
