import { execFileSync } from "node:child_process";
import { setTimeout } from "node:timers/promises";
import { pathToFileURL } from "node:url";

// Query history so a newer staging deployment does not invalidate an older,
// successfully tested release candidate.
export async function verifyStaging({
  api,
  repository,
  token,
  sha,
  request = fetch,
}) {
  for (let page = 1; ; page++) {
    const url = new URL(`${api}/repos/${repository}/actions/runs`);
    url.search = new URLSearchParams({
      workflow_id: "staging.yml",
      head_sha: sha,
      event: "push",
      ref: "refs/heads/main",
      status: "success",
      limit: "50",
      page: String(page),
    });
    const response = await request(url, {
      headers: { Authorization: `Bearer ${token}` },
      signal: AbortSignal.timeout(30_000),
    });
    if (!response.ok)
      throw new Error(`Staging history request failed: ${response.status}`);
    const { workflow_runs: runs } = await response.json();
    if (!Array.isArray(runs))
      throw new Error("Invalid staging history response");
    const staged = runs.find(
      (run) =>
        run.workflow_id === "staging.yml" &&
        run.commit_sha === sha &&
        run.event === "push" &&
        run.prettyref === "main" &&
        run.status === "success",
    );
    if (staged) {
      if (!Number.isSafeInteger(staged.id) || staged.id < 1)
        throw new Error("Invalid staging run ID");
      return staged.id;
    }
    if (runs.length === 0)
      throw new Error(`No successful staging deployment for ${sha}`);
  }
}

export async function waitForDeployment(
  id,
  load,
  sleep = setTimeout,
  attempts = 120,
) {
  for (let attempt = 0; attempt < attempts; attempt++) {
    const deployment = (await load()).find((item) => item.id === id);
    if (deployment?.status === "SUCCESS") return;
    if (
      deployment &&
      !["INITIALIZING", "QUEUED", "WAITING", "BUILDING", "DEPLOYING"].includes(
        deployment.status,
      )
    ) {
      throw new Error(`Deployment ${id}: ${deployment.status}`);
    }
    await sleep(10_000);
  }
  throw new Error(`Timed out waiting for deployment ${id}`);
}

export async function deployImage(image, ids, mutate) {
  if (!/^[^@\s]+@sha256:[0-9a-f]{64}$/.test(image))
    throw new Error("Expected image digest");
  if (!ids.serviceId || !ids.environmentId)
    throw new Error("Missing Railway target IDs");
  const updated = await mutate(
    `mutation($serviceId:String!,$environmentId:String!,$input:ServiceInstanceUpdateInput!) {
    serviceInstanceUpdate(serviceId:$serviceId, environmentId:$environmentId, input:$input)
  }`,
    {
      ...ids,
      input: {
        source: { image },
        startCommand: "/app/ixirsii-tech",
        healthcheckPath: "/health",
        healthcheckTimeout: 300,
        restartPolicyType: "ON_FAILURE",
        restartPolicyMaxRetries: 10,
      },
    },
  );
  if (updated.serviceInstanceUpdate !== true)
    throw new Error("Railway image source update failed");
  const result = await mutate(
    `mutation($serviceId:String!,$environmentId:String!) {
    serviceInstanceDeployV2(serviceId:$serviceId, environmentId:$environmentId)
  }`,
    ids,
  );
  if (
    typeof result.serviceInstanceDeployV2 !== "string" ||
    !result.serviceInstanceDeployV2
  )
    throw new Error("Railway did not return a deployment ID");
  return result.serviceInstanceDeployV2;
}

async function main() {
  const sha = execFileSync("git", ["rev-parse", "HEAD"], {
    encoding: "utf8",
  }).trim();
  if (process.argv[2] === "verify-staging") {
    const run = await verifyStaging({
      api: process.env.FORGEJO_API_URL,
      repository: process.env.FORGEJO_REPOSITORY,
      token: process.env.FORGEJO_TOKEN,
      sha,
    });
    console.log(run);
    return;
  }
  if (process.argv[2] !== "deploy")
    throw new Error("Expected deploy or verify-staging");
  const environment = process.env.RAILWAY_ENVIRONMENT;
  if (!["staging", "production"].includes(environment))
    throw new Error("Invalid environment");
  const railway = (args) =>
    execFileSync("railway", args, {
      encoding: "utf8",
      timeout: 300_000,
      stdio: ["ignore", "pipe", "inherit"],
    });
  const target = [
    "--environment",
    environment,
    "--service",
    process.env.RAILWAY_SERVICE,
  ];
  const vars = JSON.parse(railway(["variable", "list", "--json", ...target]));
  if (
    vars.RAILWAY_PROJECT_ID !== process.env.RAILWAY_PROJECT_ID ||
    vars.RAILWAY_ENVIRONMENT_NAME !== environment
  )
    throw new Error("Railway target mismatch");
  const id = await deployImage(
    process.env.RELEASE_IMAGE,
    {
      environmentId: vars.RAILWAY_ENVIRONMENT_ID,
      serviceId: vars.RAILWAY_SERVICE_ID,
    },
    async (query, variables) => {
      const response = await fetch("https://backboard.railway.com/graphql/v2", {
        method: "POST",
        signal: AbortSignal.timeout(30_000),
        headers: {
          "Content-Type": "application/json",
          "Project-Access-Token": process.env.RAILWAY_TOKEN,
        },
        body: JSON.stringify({ query, variables }),
      });
      if (!response.ok)
        throw new Error(`Railway API request failed: ${response.status}`);
      const { data, errors } = await response.json();
      if (errors || !data)
        throw new Error(
          `Railway API: ${errors?.[0]?.message ?? "empty response"}`,
        );
      return data;
    },
  );
  console.log(`Waiting for ${environment} deployment ${id}`);
  await waitForDeployment(id, () =>
    JSON.parse(
      railway(["deployment", "list", "--json", "--limit", "100", ...target]),
    ),
  );
  console.log(
    `Healthy ${environment} deployment of ${sha}: ${id} (${process.env.RELEASE_IMAGE})`,
  );
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
