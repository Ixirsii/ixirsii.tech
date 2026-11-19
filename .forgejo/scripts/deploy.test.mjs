import assert from "node:assert/strict";
import { test } from "node:test";
import { deployImage, verifyStaging, waitForDeployment } from "./deploy.mjs";

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
