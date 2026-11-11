/* Offline design prototype. All content is fictional and bounded locally. */
const projects = [
  "Silkboard",
  "Bombyx",
  "Hoard",
  "MuTrack",
  "Phren",
  "Rustbase",
  "Atlas",
  "Juniper",
  "Lantern",
  "Meadow",
];
const tasks = [
  "Refine controls at scale",
  "Check phone filter layouts",
  "Prepare VM health checks",
  "Verify provisioning recovery",
  "Review release admission",
  "Document restore procedure",
  "Simplify activity summaries",
  "Check tagged upgrade",
  "Improve search results",
  "Verify signature refusal",
  "Extract shared updater policy",
  "Review build cache behavior",
  "Tidy project settings",
  "Verify access boundaries",
  "Polish empty states",
  "Review keyboard navigation",
  "Improve workflow details",
  "Check received history",
  "Clarify agent identity",
  "Verify dark theme contrast",
];
const reports = [
  "Comparing compact layouts with twenty agents.",
  "Phone filters fit without horizontal scrolling.",
  "Waiting for the operator's target selection.",
  "Restart fixtures preserve the last healthy release.",
  "Exact revision checks pass in local fixtures.",
  "Recovery notes are ready for review.",
  "Activity rows now show concise summaries.",
  "Preparing an isolated upgrade fixture.",
  "Search results retain stable project identity.",
  "Untrusted signature fixtures are refused.",
];
const states = ["active", "completed", "blocked", "active", "completed"];
const agents = tasks.map((title, i) => ({
  id: `agent-${i + 1}`,
  project: projects[Math.floor(i / 2)],
  title,
  tool: i % 3 === 0 ? "Claude Code" : "Codex",
  host: i % 2 ? "lumi" : "frosti",
  state: states[i % states.length],
  report: reports[i % reports.length],
  received: new Date(Date.UTC(2026, 10, 3, 12, 0) - i * 27 * 60_000),
  session: `00000000-0000-4000-8000-${String(i + 1).padStart(12, "0")}`,
}));
const sampleNow = Date.UTC(2026, 10, 3, 12, 0);
const $ = (selector) => document.querySelector(selector);
let project = "",
  agent = "",
  range = null,
  pickerKind = "project";
let view = location.hash === "#events" ? "events" : "agents";
let theme =
  new URLSearchParams(location.search).get("theme") === "dark"
    ? "dark"
    : "light";
const picker = $("#picker"),
  rangeDialog = $("#range");
for (const dialog of [picker, rangeDialog]) {
  dialog.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      event.preventDefault();
      dialog.close();
    }
    if (event.key !== "Tab") return;
    const targets = [...dialog.querySelectorAll("button, input")].filter(
      (element) => !element.disabled && element.getClientRects().length,
    );
    const first = targets[0],
      last = targets.at(-1);
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  });
}
function node(tag, className, text) {
  const result = document.createElement(tag);
  if (className) result.className = className;
  if (text !== undefined) result.textContent = text;
  return result;
}
function timeText(date) {
  return date.toISOString().slice(11, 16);
}
function selectionButton(label, selected, onClick, className) {
  const button = node("button", className, label);
  button.type = "button";
  button.setAttribute("aria-pressed", String(selected));
  button.onclick = onClick;
  return button;
}
function updateTheme() {
  document.documentElement.dataset.theme = theme;
  $("#theme").setAttribute(
    "aria-label",
    `Switch to ${theme === "light" ? "dark" : "light"} theme`,
  );
}
function renderProjects() {
  const root = $("#projects");
  if (root.childElementCount) {
    for (const button of root.children) {
      button.setAttribute(
        "aria-pressed",
        String(button.dataset.project === project),
      );
    }
    return;
  }
  for (const name of ["", ...projects]) {
    const button = selectionButton(
      "",
      project === name,
      () => {
        project = name;
        agent = "";
        render();
      },
      "project-choice",
    );
    button.dataset.project = name;
    button.append(
      node(
        "span",
        "project-initial",
        name ? name.slice(0, 2).toUpperCase() : "▦",
      ),
      node("span", "", name || "All projects"),
      node("span", "count", name ? "2" : "20"),
    );
    root.append(button);
  }
}
function render() {
  const isEvents = view === "events";
  renderProjects();
  $("#page-title").textContent = isEvents ? "Events" : "Agents";
  $("#intro").textContent = isEvents
    ? "Received activity across your projects. Narrow the view without losing context."
    : "20 main agents across 10 projects. Expand Report & identity for session details.";
  for (const link of document.querySelectorAll("[data-view]")) {
    if (link.dataset.view === view) link.setAttribute("aria-current", "page");
    else link.removeAttribute("aria-current");
  }
  $("#project-trigger strong").textContent = project || "All projects";
  $("#agent-trigger strong").textContent = agent
    ? agents.find((a) => a.id === agent).title
    : "All agents";
  $("#range-trigger").hidden = !isEvents;
  $("#range-trigger strong").textContent = range?.label || "All time";
  $("#toolbar-note").textContent = isEvents
    ? "Received time · UTC"
    : "Main sessions only";
  $("#clear").hidden = !project && !agent && !(isEvents && range);
  const visible = agents.filter(
    (a) =>
      (!project || a.project === project) &&
      (!agent || a.id === agent) &&
      (!isEvents ||
        !range ||
        (a.received.getTime() >= range.from &&
          a.received.getTime() <= range.to)),
  );
  $("#result-count").textContent =
    `${visible.length} ${isEvents ? "events" : "agents"} · ${project || "All 10 projects"}`;
  const root = $("#results");
  root.replaceChildren();
  root.className = isEvents ? "event-list" : "agent-grid";
  root.setAttribute(
    "aria-label",
    isEvents ? "Received events" : "Main agent reports",
  );
  if (!visible.length) {
    root.append(
      node(
        "p",
        "empty",
        "No reports match these filters. Clear filters to see all reports.",
      ),
    );
    return;
  }
  for (const a of visible) {
    if (isEvents) {
      const row = node("article", "event-row");
      const time = node("time", "", `${timeText(a.received)}\n11-03`);
      time.dateTime = a.received.toISOString();
      row.append(
        time,
        node("p", "event-project", a.project),
        node("p", "event-agent", `${a.tool} · ${a.host}`),
        node("h2", "", a.report),
      );
      root.append(row);
    } else {
      const card = node("article", "agent-card");
      card.dataset.agent = a.id;
      const top = node("div", "card-top");
      top.append(
        node("span", "project-tag", a.project),
        node(
          "span",
          `state ${a.state}`,
          `● ${a.state === "completed" ? "Completed" : a.state === "blocked" ? "Blocked" : "Active"}`,
        ),
      );
      const footer = node("div", "card-footer");
      const time = node("time", "", `Received ${timeText(a.received)} UTC`);
      time.dateTime = a.received.toISOString();
      const details = node("details");
      details.append(node("summary", "", "Report & identity"));
      const identity = node("div", "identity");
      identity.append(
        node("p", "", `Workflow: ${a.title} · Reported ${a.state}`),
        node("p", "", `Project: ${a.project} · ${a.tool} · ${a.host}`),
        node("code", "", `Native session: ${a.session}`),
      );
      details.append(identity);
      footer.append(time);
      card.append(
        top,
        node("h2", "", a.title),
        node("p", "meta", `${a.tool} / ${a.host}`),
        node("p", "activity", a.report),
        footer,
        details,
      );
      root.append(card);
    }
  }
}
function renderOptions() {
  const query = $("#picker-search").value.trim().toLowerCase();
  const list = $("#picker-options");
  list.replaceChildren();
  const isProject = pickerKind === "project";
  const options = isProject
    ? projects.map((p) => ({ id: p, label: p, sub: "2 main agents" }))
    : agents
        .filter((a) => !project || a.project === project)
        .map((a) => ({
          id: a.id,
          label: a.title,
          sub: `${a.project} · ${a.tool} / ${a.host}`,
        }));
  const all = [
    {
      id: "",
      label: isProject ? "All projects" : "All agents",
      sub: isProject
        ? "10 projects · 20 agents"
        : "All sessions in selected project scope",
    },
    ...options,
  ];
  for (const item of all.filter((o) =>
    `${o.label} ${o.sub}`.toLowerCase().includes(query),
  )) {
    const selected = (isProject ? project : agent) === item.id;
    const button = selectionButton(
      "",
      selected,
      () => {
        if (isProject) {
          project = item.id;
          agent = "";
        } else agent = item.id;
        picker.close();
        render();
      },
      "picker-option",
    );
    const label = node("span", "", item.label);
    label.append(node("small", "", item.sub));
    button.append(label, node("span", "check", selected ? "✓" : ""));
    list.append(button);
  }
  if (!list.children.length)
    list.append(
      node(
        "p",
        "empty",
        isProject
          ? "No matching projects. Try a project name."
          : "No matching choices. Try a project, tool, host or task name.",
      ),
    );
}
function openPicker(kind) {
  pickerKind = kind;
  $("#picker-title").textContent =
    kind === "project" ? "Choose project" : "Choose agent";
  $("#search-kind").textContent = kind === "project" ? "projects" : "agents";
  $("#picker-search").value = "";
  renderOptions();
  picker.showModal();
  $("#picker-search").focus();
}
function openRange() {
  $("#range-error").hidden = true;
  const from = new Date(range?.from ?? Date.UTC(2026, 10, 3, 9));
  const to = new Date(range?.to ?? sampleNow);
  for (const [side, date] of [
    ["from", from],
    ["to", to],
  ]) {
    $(`#${side}-date`).value = date.toISOString().slice(0, 10);
    $(`#${side}-time`).value = timeText(date);
  }
  for (const button of document.querySelectorAll("[data-preset]"))
    button.setAttribute(
      "aria-pressed",
      String(
        button.dataset.preset === (range?.preset || (range ? "custom" : "all")),
      ),
    );
  rangeDialog.showModal();
}
function parseEndpoint(side) {
  const date = $(`#${side}-date`).value.trim(),
    time = $(`#${side}-time`).value.trim();
  if (
    !/^\d{4}-\d{2}-\d{2}$/.test(date) ||
    !/^([01]\d|2[0-3]):[0-5]\d$/.test(time)
  )
    return NaN;
  const result = Date.parse(`${date}T${time}:00Z`);
  if (
    !Number.isFinite(result) ||
    result < 0 ||
    new Date(result).toISOString().slice(0, 16) !== `${date}T${time}`
  )
    return NaN;
  return result;
}
$("#range-form").onsubmit = (event) => {
  event.preventDefault();
  const from = parseEndpoint("from"),
    to = parseEndpoint("to");
  if (!Number.isFinite(from) || !Number.isFinite(to) || from > to) {
    $("#range-error").textContent =
      "Enter valid dates and 24-hour times, with From before To. Your applied range has not changed.";
    $("#range-error").hidden = false;
    return;
  }
  range = {
    from,
    to,
    label: `${$("#from-date").value} ${$("#from-time").value} → ${$("#to-date").value} ${$("#to-time").value}`,
  };
  rangeDialog.close();
  render();
};
for (const button of document.querySelectorAll("[data-preset]"))
  button.onclick = () => {
    const preset = button.dataset.preset;
    range =
      preset === "all"
        ? null
        : {
            preset,
            label: button.textContent,
            from:
              sampleNow -
              { hour: 3_600_000, day: 86_400_000, week: 604_800_000 }[preset],
            to: sampleNow,
          };
    rangeDialog.close();
    render();
  };
$("#cancel-range").onclick = () => rangeDialog.close();
for (const button of document.querySelectorAll(".close"))
  button.onclick = () => button.closest("dialog").close();
$("#project-trigger").onclick = () => openPicker("project");
$("#agent-trigger").onclick = () => openPicker("agent");
$("#picker-search").oninput = renderOptions;
$("#range-trigger").onclick = openRange;
$("#clear").onclick = () => {
  project = agent = "";
  range = null;
  render();
  $("#agent-trigger").focus();
};
$("#theme").onclick = () => {
  theme = theme === "light" ? "dark" : "light";
  updateTheme();
};
window.onhashchange = () => {
  view = location.hash === "#events" ? "events" : "agents";
  render();
};
updateTheme();
render();
