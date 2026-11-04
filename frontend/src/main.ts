import "./theme.css";
import "./app.css";
import { mount } from "svelte";
import App from "./App.svelte";
import { takeSetupLink } from "./setup-link";
import { initializeTheme } from "./theme";

initializeTheme();

let setupCredential = takeSetupLink(window.location, window.history);

const target = document.getElementById("app");
if (!target) {
  throw new Error("#app element not found");
}

const app = mount(App, {
  target,
  props: {
    setupPage: window.location.pathname === "/setup",
    unknownPage:
      !["/", "/setup", "/status", "/todos", "/events", "/plans"].includes(
        window.location.pathname,
      ) && !window.location.pathname.startsWith("/plans/"),
    statusPage: window.location.pathname === "/status",
    todosPage: window.location.pathname === "/todos",
    eventsPage: window.location.pathname === "/events",
    plansPage:
      window.location.pathname === "/plans" ||
      window.location.pathname.startsWith("/plans/"),
    takeSetupCredential: () => {
      const value = setupCredential;
      setupCredential = "";
      return value;
    },
  },
});

export default app;
