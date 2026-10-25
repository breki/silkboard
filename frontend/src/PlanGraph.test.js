import {
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import PlanGraph from "./PlanGraph.svelte";
import StepDetails from "./StepDetails.svelte";
import { detail, reporter } from "../test/plans";
let media, resize;
beforeEach(() => {
  media = {
    matches: false,
    addEventListener: vi.fn((_, fn) => (resize = fn)),
    removeEventListener: vi.fn(),
  };
  vi.stubGlobal("matchMedia", () => media);
  HTMLDialogElement.prototype.showModal = vi.fn(function () {
    this.open = true;
  });
  HTMLDialogElement.prototype.close = vi.fn(function () {
    this.open = false;
    this.dispatchEvent(new Event("close"));
  });
  Element.prototype.scrollIntoView = vi.fn();
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  delete Element.prototype.scrollIntoView;
  delete HTMLDialogElement.prototype.showModal;
  delete HTMLDialogElement.prototype.close;
});
const flush = () => new Promise((resolve) => setTimeout(resolve, 0));
const props = (value) => ({ detail: value, pinned: false, onopen: vi.fn() });
test("locates active steps and retains selection and geometry across state updates", async () => {
  let value = detail();
  const view = render(PlanGraph, props(value));
  const node = screen.getByRole("button", { name: "Build graph page, Active" });
  const position = node.style.cssText;
  await fireEvent.click(screen.getByText("Locate active step"));
  expect(document.activeElement).toBe(node);
  expect(node.getAttribute("aria-pressed")).toBe("true");
  value = structuredClone(value);
  value.snapshot.steps[1].state = "done";
  value.snapshot.steps[2].state = "active";
  value = detail(value.snapshot);
  await view.rerender(props(value));
  expect(node.style.cssText).toBe(position);
  expect(
    screen.getByRole("heading", { name: "Build graph page" }),
  ).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("button", { name: "Steps", exact: true }),
  );
  expect(
    screen.getByRole("list", { name: "Steps in prerequisite order" }),
  ).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("button", { name: "Agree contract, Done" }),
  );
  expect(screen.getByText("No prerequisites.")).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("button", { name: "Graph", exact: true }),
  );
  expect(screen.getByRole("region")).toBeTruthy();
});
test("several active steps require explicit choice and no active step remains honest", async () => {
  let value = detail();
  value.snapshot.steps[2].depends_on = ["contract"];
  value.snapshot.steps[2].state = "active";
  const view = render(PlanGraph, props(detail(value.snapshot)));
  const select = screen.getByRole("combobox", { name: "Choose active step" });
  await fireEvent.change(select, { target: { value: "verify" } });
  expect(
    screen.getByRole("heading", { name: "Verify browser behavior" }),
  ).toBeTruthy();
  await fireEvent.change(select, { target: { value: "" } });
  value = structuredClone(value);
  value.snapshot.steps.forEach((step) => (step.state = "pending"));
  await view.rerender(props(detail(value.snapshot)));
  expect(
    screen.getByText("No step is marked active in this snapshot."),
  ).toBeTruthy();
});
test("removed selected steps explain the change and restore a useful focus target", async () => {
  let value = detail();
  const view = render(PlanGraph, props(value));
  const selected = screen.getByRole("button", {
    name: "Verify browser behavior, Pending",
  });
  selected.focus();
  await fireEvent.click(selected);
  value = structuredClone(value);
  value.snapshot.steps.pop();
  await view.rerender(props(detail(value.snapshot)));
  await flush();
  expect(
    screen.getByText("The selected step is not in this snapshot."),
  ).toBeTruthy();
  expect(document.activeElement).toBe(
    screen.getByRole("heading", { name: "Steps and prerequisites" }),
  );
});
test("phone details open a native modal, close or Escape returns focus, and resizing remains usable", async () => {
  media.matches = true;
  const view = render(PlanGraph, props(detail()));
  await flush();
  const node = screen.getByRole("button", { name: "Build graph page, Active" });
  node.focus();
  await fireEvent.click(node);
  await flush();
  let dialog = screen.getByRole("dialog", { name: "Step details" });
  expect(dialog.open).toBe(true);
  await fireEvent.click(within(dialog).getByText("Close step details"));
  await flush();
  expect(document.activeElement).toBe(node);
  await fireEvent.click(screen.getByText("View selected step"));
  await flush();
  dialog = screen.getByRole("dialog");
  await fireEvent(dialog, new Event("cancel", { cancelable: true }));
  await flush();
  expect(dialog.open).toBe(false);
  expect(document.activeElement).toBe(node);
  await fireEvent.click(node);
  await flush();
  media.matches = false;
  resize();
  await flush();
  expect(
    screen.getByRole("complementary", { name: "Step details" }),
  ).toBeTruthy();
  const link = screen.getByRole("heading", { name: "Build graph page" });
  link.tabIndex = -1;
  link.focus();
  media.matches = true;
  resize();
  await flush();
  expect(screen.getByRole("dialog")).toBeTruthy();
  view.unmount();
  expect(media.removeEventListener).toHaveBeenCalled();
});
test("a removed selection in a modal closes safely", async () => {
  media.matches = true;
  let value = detail();
  const view = render(PlanGraph, props(value));
  await flush();
  await fireEvent.click(
    screen.getByRole("button", { name: "Verify browser behavior, Pending" }),
  );
  await flush();
  value = structuredClone(value);
  value.snapshot.steps.pop();
  await view.rerender(props(detail(value.snapshot)));
  await flush();
  expect(screen.queryByRole("dialog")).toBeNull();
});
test("safe Markdown, blocking reasons and real prior completion links retain identity", async () => {
  const value = detail({
    reporter: {
      ...reporter,
      subagent_id: "reviewer",
      parent_session_id: "parent",
    },
    steps: [
      {
        id: "retry",
        title: "Retry",
        description:
          "**Text** <script>bad()</script> ![x](https://evil.invalid/x)",
        depends_on: [],
        state: "blocked",
        report: "`report`",
        blocked_reason: "Waiting for **review**",
      },
    ],
  });
  value.prior_completions = [
    { step_id: "retry", sequence: "1", received_at: 1 },
  ];
  const onopen = vi.fn();
  render(StepDetails, {
    detail: value,
    step: value.snapshot.steps[0],
    pinned: true,
    onopen,
  });
  expect(screen.getByText("Blocking reason")).toBeTruthy();
  expect(screen.getByText("parent")).toBeTruthy();
  expect(screen.getByText("reviewer")).toBeTruthy();
  expect(document.querySelector("script,img")).toBeNull();
  await fireEvent.click(
    screen.getByRole("link", { name: "View that snapshot" }),
  );
  expect(onopen).toHaveBeenCalledOnce();
});

test("a reporter without a readable name still identifies the native session", () => {
  const value = detail({ reporter: { ...reporter, session_name: null } });
  render(StepDetails, {
    detail: value,
    step: value.snapshot.steps[0],
    pinned: false,
    onopen: vi.fn(),
  });
  expect(screen.getByText("native-1")).toBeTruthy();
  expect(screen.queryByText("Session name")).toBeNull();
});
