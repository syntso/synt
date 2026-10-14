#!/bin/sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
run="$here/run"
compose="docker compose --file $here/compose.yaml"
kubectl="$compose exec -T cluster kubectl"

rm -rf "$run"
mkdir -p "$run/bastion" "$run/cluster"

ssh-keygen -q -t ed25519 -N '' -C synt-kube-tests -f "$run/client_ed25519"
ssh-keygen -q -t ed25519 -N '' -C bastion -f "$run/bastion/host_ed25519_key"
cp "$run/client_ed25519.pub" "$run/bastion/authorized_keys"

$compose up --build --detach --force-recreate --wait

for attempt in $(seq 1 120); do
  if [ -s "$run/cluster/k3s.yaml" ] && $kubectl get --raw /readyz > /dev/null 2>&1; then
    break
  fi
  sleep 1
done

$kubectl wait --for=condition=Ready node --all --timeout=120s
for attempt in $(seq 1 60); do
  if $kubectl --namespace default get serviceaccount default > /dev/null 2>&1; then
    break
  fi
  sleep 1
done

$kubectl create serviceaccount synt-plugin --namespace kube-system
$kubectl create clusterrolebinding synt-plugin --clusterrole cluster-admin \
  --serviceaccount kube-system:synt-plugin
$kubectl create token synt-plugin --namespace kube-system --duration 24h > "$run/token"
cp "$here/credential.sh" "$run/credential.sh"
chmod +x "$run/credential.sh"

api_port=$($compose port cluster 6443 | sed 's/.*://')
ssh_port=$($compose port bastion 22 | sed 's/.*://')
k3s="$run/cluster/k3s.yaml"
authority=$(sed -n 's/^ *certificate-authority-data: //p' "$k3s")
certificate=$(sed -n 's/^ *client-certificate-data: //p' "$k3s")
key=$(sed -n 's/^ *client-key-data: //p' "$k3s")

cat > "$run/kubeconfig" <<CONFIG
apiVersion: v1
kind: Config
current-context: direct
clusters:
- name: direct
  cluster:
    server: https://127.0.0.1:$api_port
    certificate-authority-data: $authority
- name: inside
  cluster:
    server: https://cluster:6443
    certificate-authority-data: $authority
users:
- name: admin
  user:
    client-certificate-data: $certificate
    client-key-data: $key
- name: plugin
  user:
    exec:
      apiVersion: client.authentication.k8s.io/v1
      command: $run/credential.sh
      interactiveMode: Never
contexts:
- name: direct
  context:
    cluster: direct
    user: admin
- name: inside
  context:
    cluster: inside
    user: admin
- name: plugin
  context:
    cluster: direct
    user: plugin
CONFIG

for attempt in $(seq 1 50); do
  if ssh-keyscan -T 2 -t ed25519 -p "$ssh_port" 127.0.0.1 > /dev/null 2>&1; then
    printf '%s\n' "$ssh_port" > "$run/bastion_port"
    echo "kube test cluster is up: api on 127.0.0.1:$api_port, bastion on 127.0.0.1:$ssh_port"
    exit 0
  fi
  sleep 0.2
done

$compose logs
echo "the bastion did not answer on 127.0.0.1:$ssh_port" >&2
exit 1
