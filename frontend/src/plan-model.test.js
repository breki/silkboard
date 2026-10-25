import { expect, test } from "vitest";
import { graphLayout, routeFor, parsePlanRoute, reportAge } from "./plan-model";

test("stable prerequisite ordering and geometry ignore state and handle branching", () => {
  const steps = [
    { id: "test", depends_on: ["code", "docs"], state: "pending" },
    { id: "docs", depends_on: ["plan"], state: "active" },
    { id: "plan", depends_on: [], state: "done" },
    { id: "code", depends_on: ["plan"], state: "active" },
  ];
  const layout = graphLayout(steps);
  expect(layout.nodes.map((n) => n.id)).toEqual([
    "plan",
    "docs",
    "code",
    "test",
  ]);
  expect(layout.edges.map((e) => [e.from, e.to])).toContainEqual([
    "plan",
    "code",
  ]);
  expect(graphLayout(steps.map((s) => ({ ...s, state: "done" })))).toEqual(
    layout,
  );
  expect(layout.nodes.find((n) => n.id === "test").y).toBeGreaterThan(
    layout.nodes.find((n) => n.id === "code").y,
  );
});
test("bounded layout rejects invalid graph identity and cycles", () => {
  for (const steps of [
    [],
    [{ id: "x", depends_on: ["x"] }],
    [{ id: "x", depends_on: ["missing"] }],
    [
      { id: "x", depends_on: [] },
      { id: "x", depends_on: [] },
    ],
    [
      { id: "a", depends_on: ["b"] },
      { id: "b", depends_on: ["a"] },
    ],
  ]) {
    expect(() => graphLayout(steps)).toThrow();
  }
  const maximum = Array.from({ length: 128 }, (_, i) => ({
    id: `s${i}`,
    depends_on: i ? [`s${i - 1}`] : [],
  }));
  expect(graphLayout(maximum).nodes).toHaveLength(128);
  expect(graphLayout(maximum).height).toBeLessThan(20000);
});
test("routes contain only validated identity and exact counters, never return URLs", () => {
  const task = "11111111-1111-4111-8111-111111111111";
  expect(parsePlanRoute("/plans")).toEqual({ kind: "list" });
  expect(parsePlanRoute(routeFor("silkboard", task, "9"))).toEqual({
    kind: "task",
    project: "silkboard",
    task,
    sequence: "9",
  });
  for (const path of [
    "/plans/../x",
    "/plans/silkboard/bad",
    "/plans?return=https://evil.invalid",
    `/plans/silkboard/${task}?sequence=01`,
    `/plans/silkboard/${task}?sequence=1&sequence=2`,
  ])
    expect(parsePlanRoute(path).kind).toBe("invalid");
  expect(reportAge(1000, 61000)).toBe("1 minute ago");
  expect(reportAge(1000, 999)).toBe("just received");
});

test("layout bounds dense DAGs and phone ordering without dropping edges", () => {
  const nodes = Array.from({ length: 128 }, (_, i) => ({
    id: `s${i}`,
    depends_on:
      i < 16
        ? []
        : Array.from({ length: Math.min(i - 15, 32) }, (_, j) => `s${j}`),
  }));
  const dense = nodes.map((n) => ({ ...n, depends_on: [] }));
  let edges = 512;
  for (let i = 16; i < 128 && edges; i++) {
    dense[i].depends_on = Array.from(
      { length: Math.min(16, edges) },
      (_, j) => `s${j}`,
    );
    edges -= dense[i].depends_on.length;
  }
  expect(graphLayout(dense, 1).edges).toHaveLength(512);
  expect(graphLayout(dense, 1).nodes).toHaveLength(128);
  dense[127].depends_on = ["s0"];
  expect(() => graphLayout(dense)).toThrow();
  for (const input of [
    [{ id: "bad ID", depends_on: [] }],
    [
      { id: "a", depends_on: [] },
      { id: "b", depends_on: ["a", "a"] },
    ],
  ])
    expect(() => graphLayout(input)).toThrow();
  expect(() => graphLayout(nodes, 2)).toThrow();
  expect(() =>
    graphLayout(
      Array.from({ length: 129 }, (_, i) => ({ id: `s${i}`, depends_on: [] })),
    ),
  ).toThrow();
});
test("route creation and parsing reject foreign and malformed navigation", () => {
  const task = "11111111-1111-4111-8111-111111111111";
  for (const args of [
    ["../bad", task],
    ["ok", "bad"],
    ["ok", task, "0"],
  ])
    expect(() => routeFor(...args)).toThrow();
  expect(parsePlanRoute(routeFor("ok", task))).toMatchObject({
    sequence: null,
  });
  for (const path of [
    "https://evil.invalid/plans",
    "//evil.invalid/plans",
    "x",
    "/" + "x".repeat(2048),
    `/plans/ok/${task}?bad=1`,
  ])
    expect(parsePlanRoute(path).kind).toBe("invalid");
  expect(reportAge(0, 120000)).toBe("2 minutes ago");
  expect(reportAge(0, 3600000)).toBe("1 hour ago");
  expect(reportAge(0, 7200000)).toBe("2 hours ago");
  expect(reportAge(0, 86400000)).toBe("1 day ago");
  expect(reportAge(0, 172800000)).toBe("2 days ago");
});
