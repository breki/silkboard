import { expect, test } from "@playwright/test";
import { password } from "./fixture.mjs";
import { readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import { gunzipSync } from "node:zlib";
import { fixturePort } from "./ports.mjs";
import { startLinksServer } from "./links-server.mjs";

const origin = `https://127.0.0.1:${fixturePort(38130)}`;
test.use({ baseURL: origin });
let stopServer;
test.beforeAll(async () => {
  stopServer = await startLinksServer();
});
test.afterAll(async () => {
  await stopServer?.();
});

test("compressed assets contain the final bundle including lazy import maps", () => {
  const directory = resolve(import.meta.dirname, "../dist/assets");
  const files = readdirSync(directory).filter((name) =>
    /\.(js|css)$/.test(name),
  );
  expect(files.length).toBeGreaterThan(0);
  for (const file of files) {
    const original = readFileSync(resolve(directory, file));
    const decoded = gunzipSync(readFileSync(resolve(directory, `${file}.gz`)));
    expect(decoded.equals(original), file).toBe(true);
  }
});

test("built assets stay cached across owner page navigation", async ({
  page,
  context,
}, info) => {
  test.setTimeout(60000);
  const signedIn = await context.request.post("/api/auth/login", {
    headers: { Origin: origin },
    data: { password },
  });
  expect(signedIn.status()).toBe(204);
  const network = await context.newCDPSession(page);
  const assetResponses = [];
  page.on("response", (response) => {
    if (new URL(response.url()).pathname.startsWith("/assets/"))
      assetResponses.push(response);
  });
  await network.send("Network.enable");
  // Controlled diagnostic conditions, not a claim about the owner's phone.
  await network.send("Network.emulateNetworkConditions", {
    offline: false,
    latency: 75,
    downloadThroughput: 125000,
    uploadThroughput: 1250000,
  });
  const samples = [];
  const sample = async (view, open, visible) => {
    const began = performance.now();
    await open();
    await expect(visible).toBeVisible();
    const assets = await page.evaluate(() =>
      performance
        .getEntriesByType("resource")
        .filter((entry) => new URL(entry.name).pathname.startsWith("/assets/"))
        .map((entry) => ({
          transferred: entry.transferSize,
          encoded: entry.encodedBodySize,
        })),
    );
    samples.push({
      view,
      elapsed_ms: Math.round(performance.now() - began),
      asset_transfer_bytes: assets.reduce(
        (sum, entry) => sum + entry.transferred,
        0,
      ),
      asset_encoded_bytes: assets.reduce(
        (sum, entry) => sum + entry.encoded,
        0,
      ),
    });
  };
  await sample(
    "cold-agents",
    () => page.goto("/"),
    page.getByRole("list", { name: "Main agents" }),
  );
  const primary = page.getByRole("navigation", { name: "Primary navigation" });
  await sample(
    "events",
    () => primary.getByRole("link", { name: "Events", exact: true }).click(),
    page.getByRole("table", { name: "Events from all projects" }),
  );
  await sample(
    "workflows",
    () => primary.getByRole("link", { name: "Workflows", exact: true }).click(),
    page.getByRole("table", { name: "Workflows from all projects" }),
  );
  console.log("Page loading measurements:", JSON.stringify(samples));
  await info.attach("page-loading.json", {
    body: JSON.stringify(samples, null, 2),
    contentType: "application/json",
  });
  expect(samples[0].asset_transfer_bytes).toBeGreaterThan(0);
  for (const sample of samples.slice(1))
    expect(sample.asset_transfer_bytes).toBe(0);
  const coldAsset = assetResponses.find((response) =>
    response.url().endsWith(".js"),
  );
  expect(coldAsset.headers()["content-encoding"]).toBe("gzip");
  expect(coldAsset.headers()["cache-control"]).toBe(
    "public, max-age=31536000, immutable",
  );
  const privateResponse = await context.request.get("/api/owner/plans");
  expect(privateResponse.status()).toBe(200);
  expect(privateResponse.headers()["cache-control"]).toBe("no-store");
  const refreshed = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === "/api/owner/plans" &&
      response.request().resourceType() === "fetch",
  );
  const refresh = await refreshed;
  expect(refresh.status()).toBe(200);
  expect(refresh.headers()["cache-control"]).toBe("no-store");
  expect(
    await page.evaluate(() =>
      performance
        .getEntriesByType("resource")
        .filter((entry) => new URL(entry.name).pathname.startsWith("/assets/"))
        .reduce((sum, entry) => sum + entry.transferSize, 0),
    ),
  ).toBe(0);
});
