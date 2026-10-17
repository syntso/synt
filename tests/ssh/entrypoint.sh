set -eu

install -m 600 /run/synt/host_ed25519_key /etc/ssh/ssh_host_ed25519_key
install -d -m 700 -o synt -g synt /home/synt/.ssh
install -m 600 -o synt -g synt /run/synt/authorized_keys /home/synt/.ssh/authorized_keys

socat TCP-LISTEN:7000,fork,reuseaddr EXEC:cat &

exec /usr/sbin/sshd -D -e
