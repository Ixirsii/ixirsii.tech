import assert from "node:assert/strict";
import { test } from "node:test";
import { verifyStaging, waitForDeployment } from "./deploy.mjs";

const sha = "a".repeat(40);
const successful = {
  workflow_id: "deploy.yml", commit_sha: sha, event: "push",
  prettyref: "main", status: "success",
};
const options = { api: "https://forgejo.example/api/v1", repository: "owner/repo", token: "token", sha };
const response = runs => ({ ok: true, json: async () => ({ workflow_runs: runs }) });

test("promotion requires the exact commit in a successful main staging run", async () => {
  for (const replacement of [
    { commit_sha: "b".repeat(40) }, { status: "running" }, { status: "failure" },
    { workflow_id: "build.yml" }, { event: "pull_request" }, { prettyref: "feature" },
  ]) {
    let page = 0;
    await assert.rejects(verifyStaging({
      ...options, request: async () => response(page++ ? [] : [{ ...successful, ...replacement }]),
    }), /No successful staging deployment/);
  }
});

test("promotion follows pagination and accepts historical staging success", async () => {
  const pages = [];
  await verifyStaging({ ...options, request: async url => {
    pages.push(url.searchParams.get("page"));
    return response(pages.length === 1 ? [{ ...successful, status: "failure" }] : [successful]);
  } });
  assert.deepEqual(pages, ["1", "2"]);
});

test("history errors fail closed", async () => {
  await assert.rejects(verifyStaging({
    ...options, request: async () => ({ ok: false, status: 403 }),
  }), /403/);
  await assert.rejects(verifyStaging({
    ...options, request: async () => ({ ok: true, json: async () => ({}) }),
  }), /Invalid staging history/);
});

test("deployment waits for its own health result, not another successful deployment", async () => {
  const states = ["BUILDING", "DEPLOYING", "SUCCESS"];
  await waitForDeployment("new", async () => [
    { id: "old", status: "SUCCESS" }, { id: "new", status: states.shift() },
  ], async () => {}, 3);
  assert.equal(states.length, 0);
});

test("failed, removed, and stuck deployments cannot succeed", async () => {
  for (const status of ["FAILED", "CRASHED", "REMOVED", "CANCELED"]) {
    await assert.rejects(waitForDeployment("new", async () => [{ id: "new", status }], async () => {}), new RegExp(status));
  }
  await assert.rejects(waitForDeployment("new", async () => [], async () => {}, 2), /Timed out/);
});
