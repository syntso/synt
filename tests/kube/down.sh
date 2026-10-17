#!/bin/sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)

docker compose --file "$here/compose.yaml" down --volumes --remove-orphans
rm -rf "$here/run"
