FROM alpine:3.22@sha256:5291449c3df73caf6ed85e649dec1b9e818b39a5d8c871e97afc13e9cd5e8fa8

RUN apk add --no-cache openssh-server socat \
 && adduser -D -s /bin/sh synt \
 && echo 'synt:*' | chpasswd -e

COPY sshd_config /etc/ssh/sshd_config
COPY entrypoint.sh /entrypoint.sh

EXPOSE 22

ENTRYPOINT ["/bin/sh", "/entrypoint.sh"]
