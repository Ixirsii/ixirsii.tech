import { pathToFileURL } from "node:url";

export function receiptUrl(api, repository, run) {
  const [owner, name] = repository.split("/");
  return `${api.replace(/\/v1\/?$/, "")}/packages/${owner}/generic/${name.toLowerCase()}-staging/${run}/image`;
}

export async function readReceipt(url, sha, request) {
  const response = await request(url);
  if (response.status === 404) return null;
  if (response.status !== 200)
    throw new Error(`Receipt lookup failed: ${response.status}`);
  const match = /^([0-9a-f]+) (sha256:[0-9a-f]{64})\n?$/.exec(
    await response.text(),
  );
  if (!match || match[1] !== sha) throw new Error("Invalid staging receipt");
  return match[2];
}

export async function publishReceipt(url, sha, digest, request) {
  const response = await request(url, {
    method: "PUT",
    body: `${sha} ${digest}\n`,
  });
  if (!response.ok)
    throw new Error(`Receipt publication failed: ${response.status}`);
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  const [command, run, sha, digest] = process.argv.slice(2);
  const env = process.env;
  const url = receiptUrl(env.FORGEJO_API_URL, env.FORGEJO_REPOSITORY, run);
  const credentials = Buffer.from(
    `${env.REGISTRY_USER}:${env.REGISTRY_TOKEN}`,
  ).toString("base64");
  const request = (url, options) =>
    fetch(url, {
      ...options,
      headers: { Authorization: `Basic ${credentials}` },
      signal: AbortSignal.timeout(30_000),
      redirect: "error",
    });
  if (command === "read") {
    const recorded = await readReceipt(url, sha, request);
    if (recorded) console.log(recorded);
  } else if (command === "publish") {
    await publishReceipt(url, sha, digest, request);
  } else throw new Error("Expected read or publish");
}
