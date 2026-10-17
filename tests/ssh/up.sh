#!/bin/sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
run="$here/run"
compose="docker compose --file $here/compose.yaml"

rm -rf "$run"
mkdir -p "$run/bastion" "$run/target"

ssh-keygen -q -t ed25519 -N '' -C synt-tests -f "$run/client_ed25519"
ssh-keygen -q -t ed25519 -N '' -C synt-vault -f "$run/vault_ed25519"

for host in bastion target; do
  ssh-keygen -q -t ed25519 -N '' -C "$host" -f "$run/$host/host_ed25519_key"
  cat "$run/client_ed25519.pub" "$run/vault_ed25519.pub" > "$run/$host/authorized_keys"
done

$compose up --build --detach --force-recreate --wait

port=$($compose port bastion 22 | sed 's/.*://')

for attempt in $(seq 1 50); do
  if ssh-keyscan -T 2 -t ed25519 -p "$port" 127.0.0.1 > /dev/null 2>&1; then
    printf '%s\n' "$port" > "$run/bastion_port"
    echo "ssh test servers are up: bastion on 127.0.0.1:$port, target behind it"
    exit 0
  fi
  sleep 0.2
done

$compose logs
echo "the bastion did not answer on 127.0.0.1:$port" >&2
exit 1
