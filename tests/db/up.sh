#!/bin/sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
run="$here/run"
compose="docker compose --file $here/compose.yaml"

rm -rf "$run"
mkdir -p "$run/tls"

cat > "$run/tls/openssl.cnf" <<'CONFIG'
[req]
distinguished_name = name
x509_extensions = server
prompt = no

[name]
CN = synt-test-db

[server]
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = DNS:localhost, IP:127.0.0.1
CONFIG

openssl req -x509 -newkey rsa:2048 -nodes -days 2 -config "$run/tls/openssl.cnf" \
  -keyout "$run/tls/server.key" -out "$run/tls/server.crt" 2> /dev/null
chmod 644 "$run/tls/server.key"

$compose up --detach --force-recreate --wait

ssh_network=synt-ssh-tests_inside

if docker network inspect "$ssh_network" > /dev/null 2>&1; then
  docker network connect --alias db-postgres "$ssh_network" "$($compose ps --quiet postgres)"
  echo "postgres also answers as db-postgres:5432 inside the ssh test network"
fi

port() {
  $compose port "$1" "$2" | sed 's/.*://'
}

{
  echo "postgres=$(port postgres 5432)"
  echo "mysql=$(port mysql 3306)"
  echo "mariadb=$(port mariadb 3306)"
  echo "redis=$(port redis 6379)"
  echo "redis_tls=$(port redis 6380)"
} > "$run/ports"

echo "database test servers are up:"
cat "$run/ports"
