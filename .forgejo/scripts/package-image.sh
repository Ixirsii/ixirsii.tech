#!/usr/bin/env bash
set -euo pipefail
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
# Debian Bookworm slim, linux/amd64 manifest.
base=docker.io/library/debian@sha256:c67e92c8099ba22853ab4cfda7cfb494d253d6012ca82dee98ba8976772af018
install -Dm755 "$1" "$work/root/app/ixirsii-tech"
install -Dm644 /etc/ssl/certs/ca-certificates.crt "$work/root/etc/ssl/certs/ca-certificates.crt"
# The telemetry exporter links native TLS even when remote export is disabled.
for lib in libssl.so.3 libcrypto.so.3; do
	install -Dm644 "/usr/lib/x86_64-linux-gnu/$lib" "$work/root/usr/lib/x86_64-linux-gnu/$lib"
done
chmod 755 "$work/root"/{app,etc,etc/ssl,etc/ssl/certs}
tar --owner=0 --group=0 -C "$work/root" -cf "$work/layer.tar" app etc usr
crane mutate "$base" --append "$work/layer.tar" --entrypoint /app/ixirsii-tech \
	--user 65534:65534 --env AUTH_CLIENT_IP_MODE=railway \
	--label "org.opencontainers.image.revision=$(git rev-parse HEAD)" --tag "$2"
