import { spawnSync } from "node:child_process";

// PGDATABASE is a database name, not a URI. Keep credentials out of argv.
const url = new URL(process.env.DATABASE_URL);
const env = {
  ...process.env,
  PGHOST: url.hostname,
  PGPORT: url.port || "5432",
  PGUSER: decodeURIComponent(url.username),
  PGPASSWORD: decodeURIComponent(url.password),
  PGDATABASE: decodeURIComponent(url.pathname.slice(1)),
  PGCONNECT_TIMEOUT: "2",
};
delete env.DATABASE_URL;

// Pass URI options through libpq's service file so psql interprets them
// itself, including options with no corresponding environment variable.
const options = Array.from(url.searchParams);
if (options.length) {
  const { writeFileSync } = await import("node:fs");
  for (const [key, value] of options) {
    if (!/^[a-z_][a-z_0-9]*$/.test(key) || /[\r\n\0]/.test(value)) {
      process.exit(1);
    }
  }
  env.PGSERVICE = "probe";
  env.PGSERVICEFILE = process.env.PROBE_SERVICE_FILE;
  writeFileSync(env.PGSERVICEFILE, "[probe]\n" +
    options.map(([key, value]) => `${key}=${value}\n`).join(""), { mode: 0o600 });
}

const result = spawnSync("psql", ["-XwAtqc", "SELECT 1"], { env, stdio: "ignore" });
process.exitCode = result.status ?? 1;
