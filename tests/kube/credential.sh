#!/bin/sh
set -eu

token=$(cat "$(dirname "$0")/token")

printf '{"apiVersion":"client.authentication.k8s.io/v1","kind":"ExecCredential","status":{"token":"%s"}}\n' "$token"
