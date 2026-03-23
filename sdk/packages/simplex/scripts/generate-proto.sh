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

echo "Cleaning $OUT_DIR..."
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

echo "Generating TypeScript from proto files..."
protoc \
	--plugin="protoc-gen-ts_proto=$PLUGIN" \
	--ts_proto_out="$OUT_DIR" \
	--ts_proto_opt=outputServices=grpc-js \
	--ts_proto_opt=esModuleInterop=true \
	--ts_proto_opt=env=node \
	--ts_proto_opt=useExactTypes=false \
	--ts_proto_opt=forceLong=string \
	-I="$PROTO_DIR" \
	"$PROTO_DIR"/mpcvault/platform/v1/api.proto \
	"$PROTO_DIR"/mpcvault/platform/v1/error.proto

echo "Done. Generated files:"
find "$OUT_DIR" -name "*.ts" | sort | sed "s|^|  |"
