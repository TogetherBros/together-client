import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import OverlayApp from "./OverlayApp";
import "./App.css";

const label = getCurrentWindow().label;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {label === "overlay" ? <OverlayApp /> : <App />}
  </React.StrictMode>
);
