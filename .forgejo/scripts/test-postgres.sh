#!/bin/sh
# The ci-rust FreeBSD runner installs packages as root. Run PostgreSQL as its
# package-provided user, in a disposable cluster with no TCP listener.
set -eu

ci_database_dir=$(mktemp -d /tmp/ixirsii-ci.XXXXXX)
chown postgres "$ci_database_dir"

cleanup() {
    su -m postgres -c "pg_ctl -D '$ci_database_dir/data' -m immediate -w stop" || true
    rm -rf "$ci_database_dir"
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM

su -m postgres -c "initdb -D '$ci_database_dir/data' --username=postgres --auth-local=trust --auth-host=reject --encoding=UTF8 --no-locale"
su -m postgres -c "pg_ctl -D '$ci_database_dir/data' -l '$ci_database_dir/server.log' -o \"-h '' -k '$ci_database_dir' -c timezone=UTC\" -w start"

# SQLx creates and migrates an isolated database for each #[sqlx::test].
export DATABASE_URL="postgresql://postgres@localhost/postgres?host=$ci_database_dir"
export SQLX_OFFLINE=true
cargo test --locked --all-targets
