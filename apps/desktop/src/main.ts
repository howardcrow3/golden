import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import { inTauri } from "./lib/api";
import { installGlobalHandlers, shellReporter } from "./lib/clientLog";

if (inTauri()) {
  import("@tauri-apps/api/core").then((mod) => {
    installGlobalHandlers(window, shellReporter(mod.invoke as never));
  });
}

mount(App, { target: document.getElementById("app")! });
