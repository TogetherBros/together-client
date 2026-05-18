import React from "react";
import ReactDOM from "react-dom/client";
import CharacterOverlay from "./components/CharacterOverlay";
import "./App.css";

document.documentElement.classList.add('char-mode');

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <CharacterOverlay />
  </React.StrictMode>
);
