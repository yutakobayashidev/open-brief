import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { FixtureDesktopPort, TauriDesktopPort } from "./desktop-port";
import "./styles.css";

const port =
  "__TAURI_INTERNALS__" in window
    ? new TauriDesktopPort()
    : new FixtureDesktopPort();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App port={port} />
  </StrictMode>,
);
