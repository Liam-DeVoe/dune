// The hooks the comparison harness (scripts/ui/compare.py) drives the /scene route with. They are
// the same as the C# client's wwwroot/js/scene.js, so one harness drives both clients:
//
//   <html data-scene="loading|ready|error">   ready once the scene has rendered
//   <html class="scene-still">                 transitions and animations off (the C# _scene.scss rule)
//   window.treacheryScene.error                the error text when data-scene is "error"
//   window.treacheryScene.locations            { "Carthag": {id, x, y}, ... } in map coordinates; empty until the board exists
//   window.treacheryScene.click(name, opts)    click a location; rejects until the board exists
//   window.treacheryScene.button(label)        click the first visible enabled button with that text
//   window.treacheryScene.settled(quietMs)     resolves once #app has had no DOM change for quietMs and no animation runs
//
// Only active on /scene paths; elsewhere this module does nothing.

import { withBase } from "./base";

interface SceneLocation { id: number; x: number; y: number }

interface SceneHooks {
  error: string | null;
  locations: Record<string, SceneLocation>;
  click(name: string, options?: { shift?: boolean; ctrl?: boolean; right?: boolean }): Promise<void>;
  button(label: string): Promise<void>;
  settled(quietMs?: number): Promise<void>;
}

declare global {
  interface Window { treacheryScene?: SceneHooks }
}

export const onScene = window.location.pathname.startsWith(withBase("scene/"));

const still = `html.scene-still *, html.scene-still *::before, html.scene-still *::after {
  transition: none !important; animation: none !important; }`;

function settled(quietMs = 100): Promise<void> {
  return new Promise((resolve) => {
    const app = document.getElementById("app")!;
    let timer: number | undefined;
    const observer = new MutationObserver(schedule);
    function schedule() {
      window.clearTimeout(timer);
      timer = window.setTimeout(check, quietMs);
    }
    function check() {
      if (document.getAnimations().some((a) => a.playState === "running")) { schedule(); return; }
      observer.disconnect();
      resolve();
    }
    observer.observe(app, { subtree: true, childList: true, attributes: true, characterData: true });
    schedule();
  });
}

function pressButton(label: string): boolean {
  const wanted = label.trim();
  const buttons = Array.from(document.querySelectorAll("button")).filter((b) => b.offsetParent !== null && !b.disabled);
  const button = buttons.find((b) => b.textContent?.trim() === wanted)
    ?? buttons.find((b) => b.textContent?.trim().toLowerCase() === wanted.toLowerCase());
  button?.click();
  return button != null;
}

if (onScene) {
  const html = document.documentElement;
  html.dataset.scene = "loading";
  html.classList.add("scene-still");
  const style = document.createElement("style");
  style.textContent = still;
  document.head.appendChild(style);
  window.treacheryScene = {
    error: null,
    locations: {},
    click: () => Promise.reject(new Error("the web client has no board yet (client-plan.md package 4)")),
    async button(label) {
      if (!pressButton(label)) throw new Error(`No visible button labelled '${label}'.`);
      await settled();
    },
    settled,
  };
}

/** The scene has rendered: the harness may start. */
export function sceneReady(locations: Record<string, SceneLocation> = {}): void {
  if (!onScene || !window.treacheryScene) return;
  // The board registers its locations itself (registerSceneBoard).
  if (Object.keys(locations).length > 0) window.treacheryScene.locations = locations;
  document.documentElement.dataset.scene = "ready";
}

/** The scene could not be shown. */
export function sceneFailed(error: string): void {
  if (!onScene || !window.treacheryScene) return;
  window.treacheryScene.error = error;
  document.documentElement.dataset.scene = "error";
}

/** The board is on screen: its locations by skin name, and how to click one (package 4). */
export function registerSceneBoard(
  locations: Record<string, SceneLocation>,
  click: (id: number, options: { shift?: boolean; ctrl?: boolean; right?: boolean }) => void,
): void {
  if (!onScene || !window.treacheryScene) return;
  window.treacheryScene.locations = locations;
  window.treacheryScene.click = async (name, options = {}) => {
    const wanted = name.trim();
    const byId = /^\d+$/.test(wanted) ? Object.values(locations).find((l) => l.id === Number(wanted)) : undefined;
    const key = Object.keys(locations).find((k) => k.toLowerCase() === wanted.toLowerCase());
    const l = byId ?? (key !== undefined ? locations[key] : undefined);
    if (!l) throw new Error(`No location named '${name}'.`);
    click(l.id, options);
    await settled();
  };
}
