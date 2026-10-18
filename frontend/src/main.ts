import "./app.css";
import { mount } from "svelte";
import App from "./App.svelte";
import { takeSetupLink } from "./setup-link";

let setupCredential = takeSetupLink(window.location, window.history);

const target = document.getElementById("app");
if (!target) {
  throw new Error("#app element not found");
}

const app = mount(App, {
  target,
  props: {
    setupPage: window.location.pathname === "/setup",
    takeSetupCredential: () => {
      const value = setupCredential;
      setupCredential = "";
      return value;
    },
  },
});

export default app;
