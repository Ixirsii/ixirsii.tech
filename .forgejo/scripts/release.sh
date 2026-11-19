#!/usr/bin/env bash
# Runs only on the trusted Linux release runner, never for pull requests.
set -euo pipefail
umask 077
: "${RAILWAY_PROJECT_ID:?}" "${RAILWAY_SERVICE:?}" "${RAILWAY_TOKEN:?}"
: "${TUNNEL_API_TOKEN:?}" "${TUNNEL_SSH_KEY:?}" "${REGISTRY_USER:?}" "${REGISTRY_TOKEN:?}"
case "$RAILWAY_ENVIRONMENT" in
staging) db=Postgres-staging ;;
production) db=Postgres ;;
*) exit 1 ;;
esac
if ! rustc -vV | grep -Fx 'host: x86_64-unknown-linux-gnu' >/dev/null; then
	echo "Release jobs need a Linux amd64 GNU toolchain" >&2
	exit 1
fi
registry=${FORGEJO_API_URL#https://}
registry=${registry%%/*}
repo="$registry/${FORGEJO_REPOSITORY,,}"
sha=$(git rev-parse HEAD)
work=$(mktemp -d)
tunnel=
cleanup() {
	if [[ -n $tunnel ]]; then kill -- "-$tunnel" 2>/dev/null || true; fi
	if [[ -n ${SSH_AGENT_PID:-} ]]; then ssh-agent -k >/dev/null; fi
	rm -rf "$work"
}
unset SSH_AGENT_PID SSH_AUTH_SOCK
trap cleanup EXIT
trap 'exit 1' TERM INT
export DOCKER_CONFIG="$work/docker" CARGO_TARGET_DIR="$work/target"
mkdir -p "$DOCKER_CONFIG"
printf '%s' "$REGISTRY_TOKEN" | crane auth login "$registry" -u "$REGISTRY_USER" --password-stdin
if [[ $RAILWAY_ENVIRONMENT == production ]]; then
	run=$(node .forgejo/scripts/deploy.mjs verify-staging)
else
	run=$RELEASE_RUN_ID
fi
digest=$(node .forgejo/scripts/receipt.mjs read "$run" "$sha")
build_image=false
if [[ -n $digest ]]; then
	[[ $(crane digest "$repo@$digest") == "$digest" ]]
elif [[ $RAILWAY_ENVIRONMENT == staging ]]; then
	build_image=true
else
	echo "Missing image receipt for staging run $run" >&2
	exit 1
fi

# The CLI prints connection credentials to stderr. Consume its local URL
# privately and keep both the CLI and its ssh child in a dedicated group.
if ! command -v psql >/dev/null; then
	echo "Cannot check tunnel to $db: psql unavailable; tunnel not started, probe not attempted" >&2
	exit 1
fi
eval "$(ssh-agent -s)" >/dev/null
printf '%s\n' "$TUNNEL_SSH_KEY" | ssh-add - >/dev/null 2>&1
unset TUNNEL_SSH_KEY
env -u RAILWAY_TOKEN RAILWAY_API_TOKEN="$TUNNEL_API_TOKEN" setsid railway connect "$db" \
	--project "$RAILWAY_PROJECT_ID" --environment "$RAILWAY_ENVIRONMENT" --tunnel-only \
	>"$work/tunnel" 2>&1 </dev/null &
tunnel=$!
unset TUNNEL_API_TOKEN
ready=false
url_seen=no
probe_attempted=no
for ((i = 0; i < 60; i++)); do
	kill -0 "$tunnel" 2>/dev/null || break
	DATABASE_URL=$(awk '$1=="URL:" && $2 ~ /^postgres(ql)?:\/\// {print $2; exit}' "$work/tunnel")
	if [[ -n $DATABASE_URL ]]; then
		url_seen=yes
		probe_attempted=yes
		if PGCONNECT_TIMEOUT=2 PGDATABASE="$DATABASE_URL" psql -XAtqc 'SELECT 1' >/dev/null 2>&1; then
			ready=true
			break
		fi
	fi
	sleep 1
done
if [[ $ready != true ]]; then
	cli_state="still running at timeout"
	if ! kill -0 "$tunnel" 2>/dev/null; then
		status=0
		wait "$tunnel" || status=$?
		cli_state="exited with status $status"
	fi
	echo "Tunnel to $db not ready: Railway CLI $cli_state; URL recognized=$url_seen; psql available=yes; probe attempted=$probe_attempted" >&2
	exit 1
fi
export DATABASE_URL
cargo sqlx migrate run
if [[ $build_image == true ]]; then
	cargo build --release --locked
else
	cargo check --release --locked
fi
unset DATABASE_URL
kill -- "-$tunnel"
wait "$tunnel" || true
tunnel=

if [[ $build_image == true ]]; then
	tag="$repo:staging-$sha-$RELEASE_RUN_ID"
	bash .forgejo/scripts/package-image.sh "$CARGO_TARGET_DIR/release/ixirsii-tech" "$tag"
	digest=$(crane digest "$tag")
	[[ $digest =~ ^sha256:[0-9a-f]{64}$ ]]
fi
RELEASE_IMAGE="$repo@$digest" node .forgejo/scripts/deploy.mjs deploy
if [[ $build_image == true ]]; then
	node .forgejo/scripts/receipt.mjs publish "$run" "$sha" "$digest"
fi
