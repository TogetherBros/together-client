import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import CharacterOverlay from "./components/CharacterOverlay";
import "./App.css";

const label = getCurrentWindow().label;

if (label.startsWith('char_')) {
  document.documentElement.classList.add('char-mode');
}


ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {label.startsWith("char_") ? <CharacterOverlay /> : <App />}
  </React.StrictMode>
);
