import { execFileSync } from "node:child_process";
import { setTimeout } from "node:timers/promises";
import { pathToFileURL } from "node:url";

// Query history so a newer staging deployment does not invalidate an older,
// successfully tested release candidate.
export async function verifyStaging({ api, repository, token, sha, request = fetch }) {
  for (let page = 1; ; page++) {
    const url = new URL(`${api}/repos/${repository}/actions/runs`);
    url.search = new URLSearchParams({
      workflow_id: "deploy.yml", head_sha: sha, event: "push",
      ref: "refs/heads/main", status: "success", limit: "50", page: String(page),
    });
    const response = await request(url, {
      headers: { Authorization: `Bearer ${token}` },
      signal: AbortSignal.timeout(30_000),
    });
    if (!response.ok) throw new Error(`Staging history request failed: ${response.status}`);
    const { workflow_runs: runs } = await response.json();
    if (!Array.isArray(runs)) throw new Error("Invalid staging history response");
    if (runs.some(run => run.workflow_id === "deploy.yml" &&
        run.commit_sha === sha && run.event === "push" &&
        run.prettyref === "main" && run.status === "success")) return;
    if (runs.length === 0) throw new Error(`No successful staging deployment for ${sha}`);
  }
}

export async function waitForDeployment(id, load, sleep = setTimeout, attempts = 120) {
  for (let attempt = 0; attempt < attempts; attempt++) {
    const deployment = (await load()).find(item => item.id === id);
    if (deployment?.status === "SUCCESS") return;
    if (deployment && !["INITIALIZING", "QUEUED", "WAITING", "BUILDING", "DEPLOYING"].includes(deployment.status)) {
      throw new Error(`Deployment ${id}: ${deployment.status}`);
    }
    await sleep(10_000);
  }
  throw new Error(`Timed out waiting for deployment ${id}`);
}

async function main() {
  const sha = execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim();
  if (process.argv[2] === "verify-staging") {
    await verifyStaging({
      api: process.env.FORGEJO_API_URL, repository: process.env.FORGEJO_REPOSITORY,
      token: process.env.FORGEJO_TOKEN, sha,
    });
    console.log(`Verified staging deployment of ${sha}`);
    return;
  }
  if (process.argv[2] !== "deploy") throw new Error("Expected deploy or verify-staging");
  const environment = process.env.RAILWAY_ENVIRONMENT;
  if (!["staging", "production"].includes(environment)) throw new Error("Invalid environment");
  const railway = args => execFileSync(process.env.RAILWAY_BIN, args, {
    encoding: "utf8", timeout: 300_000, stdio: ["ignore", "pipe", "inherit"],
  });
  const target = ["--environment", environment, "--service", process.env.RAILWAY_SERVICE];
  const output = railway(["up", "--detach", "--json", ...target, "--message", `${environment} ${sha}`]);
  // The CLI can emit progress lines before its final JSON result.
  const result = output.split("\n").flatMap(line => {
    try { return [JSON.parse(line)]; } catch { return []; }
  }).find(value => value && typeof value.deploymentId === "string");
  if (!result) throw new Error("Railway did not return a deployment ID");
  console.log(`Waiting for ${environment} deployment ${result.deploymentId}`);
  await waitForDeployment(result.deploymentId, () =>
    JSON.parse(railway(["deployment", "list", "--json", "--limit", "100", ...target])));
  console.log(`Healthy ${environment} deployment of ${sha}: ${result.deploymentId}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch(error => { console.error(error.message); process.exitCode = 1; });
}
