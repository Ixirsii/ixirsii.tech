import assert from "node:assert/strict";
import { test } from "node:test";
import { deployImage, verifyStaging, waitForDeployment } from "./deploy.mjs";
import { publishReceipt, readReceipt, receiptUrl } from "./receipt.mjs";

const sha = "a".repeat(40);
const successful = {
  id: 123,
  workflow_id: "staging.yml",
  commit_sha: sha,
  event: "push",
  prettyref: "main",
  status: "success",
};
const options = {
  api: "https://forgejo.example/api/v1",
  repository: "owner/repo",
  token: "token",
  sha,
};
const response = (runs) => ({
  ok: true,
  json: async () => ({ workflow_runs: runs }),
});

test("promotion requires the exact commit in a successful main staging run", async () => {
  for (const replacement of [
    { commit_sha: "b".repeat(40) },
    { status: "running" },
    { status: "failure" },
    { workflow_id: "checks.yml" },
    { event: "pull_request" },
    { prettyref: "feature" },
  ]) {
    let page = 0;
    await assert.rejects(
      verifyStaging({
        ...options,
        request: async () =>
          response(page++ ? [] : [{ ...successful, ...replacement }]),
      }),
      /No successful staging deployment/,
    );
  }
});

test("promotion follows pagination and accepts historical staging success", async () => {
  const pages = [];
  const id = await verifyStaging({
    ...options,
    request: async (url) => {
      pages.push(url.searchParams.get("page"));
      return response(
        pages.length === 1
          ? [{ ...successful, status: "failure" }]
          : [successful],
      );
    },
  });
  assert.deepEqual(pages, ["1", "2"]);
  assert.equal(id, successful.id);
});

test("deployment requires a pinned image and successful environment-specific update", async () => {
  const ids = { environmentId: "staging", serviceId: "web" };
  const calls = [];
  const mutate = async (_, variables) => {
    calls.push(variables);
    return { serviceInstanceUpdate: false };
  };
  await assert.rejects(deployImage("repo:latest", ids, mutate), /digest/);
  assert.equal(calls.length, 0);
  const image = `repo@sha256:${"b".repeat(64)}`;
  await assert.rejects(deployImage(image, ids, mutate), /update failed/);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].environmentId, "staging");
  assert.equal(calls[0].input.source.image, image);
});

test("deployment updates the image before deploying the same target", async () => {
  const ids = { environmentId: "staging", serviceId: "web" };
  const image = `repo@sha256:${"b".repeat(64)}`;
  const calls = [];
  const id = await deployImage(image, ids, async (query, variables) => {
    calls.push({ query, variables });
    return calls.length === 1
      ? { serviceInstanceUpdate: true }
      : { serviceInstanceDeployV2: "deployment-123" };
  });
  assert.equal(id, "deployment-123");
  assert.equal(calls.length, 2);
  assert.match(calls[0].query, /serviceInstanceUpdate\(/);
  const { input, ...target } = calls[0].variables;
  assert.deepEqual(target, ids);
  assert.deepEqual(input.source, { image });
  assert.match(calls[1].query, /serviceInstanceDeployV2\(/);
  assert.deepEqual(calls[1].variables, ids);

  for (const result of [{ serviceInstanceDeployV2: "" }, {}]) {
    const replies = [{ serviceInstanceUpdate: true }, result];
    await assert.rejects(
      deployImage(image, ids, () => replies.shift()),
      /Railway did not return a deployment ID/,
    );
  }
});

test("a staging retry recovers the digest after a lost publication response", async () => {
  const url = receiptUrl(
    "https://forgejo.example/git/api/v1/",
    "Owner/Repo",
    123,
  );
  assert.equal(
    url,
    "https://forgejo.example/git/api/packages/Owner/generic/repo-staging/123/image",
  );
  const digest = `sha256:${"b".repeat(64)}`;
  const files = new Map();
  const request = async (path, opts) => {
    if (opts?.method === "PUT") {
      files.set(path, opts.body);
      throw new Error("connection lost");
    }
    return files.has(path)
      ? new Response(files.get(path))
      : new Response(null, { status: 404 });
  };
  assert.equal(await readReceipt(url, sha, request), null);
  await assert.rejects(
    publishReceipt(url, sha, digest, request),
    /connection lost/,
  );
  assert.equal(await readReceipt(url, sha, request), digest);
  await assert.rejects(
    readReceipt(url, "wrong-commit", request),
    /Invalid staging receipt/,
  );
});

test("receipt lookup rejects failed requests and malformed receipts", async () => {
  await assert.rejects(
    readReceipt("url", sha, async () => new Response(null, { status: 403 })),
    /403/,
  );
  await assert.rejects(
    readReceipt("url", sha, async () => new Response(`${sha} latest\n`)),
    /Invalid staging receipt/,
  );
});

test("history errors fail closed", async () => {
  await assert.rejects(
    verifyStaging({
      ...options,
      request: async () => ({ ok: false, status: 403 }),
    }),
    /403/,
  );
  await assert.rejects(
    verifyStaging({
      ...options,
      request: async () => ({ ok: true, json: async () => ({}) }),
    }),
    /Invalid staging history/,
  );
});

test("deployment waits for its own health result, not another successful deployment", async () => {
  const states = ["BUILDING", "DEPLOYING", "SUCCESS"];
  await waitForDeployment(
    "new",
    async () => [
      { id: "old", status: "SUCCESS" },
      { id: "new", status: states.shift() },
    ],
    async () => {},
    3,
  );
  assert.equal(states.length, 0);
});

test("failed, removed, and stuck deployments cannot succeed", async () => {
  for (const status of ["FAILED", "CRASHED", "REMOVED", "CANCELED"]) {
    await assert.rejects(
      waitForDeployment(
        "new",
        async () => [{ id: "new", status }],
        async () => {},
      ),
      new RegExp(status),
    );
  }
  await assert.rejects(
    waitForDeployment(
      "new",
      async () => [],
      async () => {},
      2,
    ),
    /Timed out/,
  );
});
