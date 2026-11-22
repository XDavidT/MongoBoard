import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { errorText } from "../api";
import { Close, Refresh } from "./Icons";

type State =
  | { kind: "idle" }
  | { kind: "checking"; manual: boolean }
  | { kind: "downloading"; version: string; percent: number | null }
  | { kind: "ready"; version: string }
  | { kind: "upToDate" }
  | { kind: "error"; message: string };

const CHECK_EVERY_MS = 6 * 60 * 60 * 1000;
const DEMO = import.meta.env.VITE_DEMO === "1";

/**
 * Self-update from GitHub Releases. Checks shortly after launch and every 6 hours,
 * downloads + installs new versions in the background, then offers a restart.
 * "MongoBoard → Check for Updates…" runs a manual check.
 */
export function Updater() {
  const [state, setState] = useState<State>({ kind: "idle" });
  const busy = useRef(false);

  const run = useCallback(async (manual: boolean) => {
    if (busy.current || DEMO) return;
    busy.current = true;
    setState({ kind: "checking", manual });
    try {
      const update: Update | null = await check();
      if (!update) {
        setState(manual ? { kind: "upToDate" } : { kind: "idle" });
        return;
      }
      let total = 0;
      let done = 0;
      setState({ kind: "downloading", version: update.version, percent: null });
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        if (event.event === "Progress") {
          done += event.data.chunkLength;
          setState({ kind: "downloading", version: update.version, percent: total ? Math.round((done / total) * 100) : null });
        }
      });
      setState({ kind: "ready", version: update.version });
    } catch (e) {
      // Background checks fail quietly (offline, no release yet); manual ones explain.
      const message = errorText(e);
      setState(
        !manual
          ? { kind: "idle" }
          : /valid release JSON|404/i.test(message)
            ? { kind: "upToDate" } // nothing published for the updater yet
            : { kind: "error", message },
      );
    } finally {
      busy.current = false;
    }
  }, []);

  useEffect(() => {
    const first = window.setTimeout(() => run(false), 4000);
    const timer = window.setInterval(() => run(false), CHECK_EVERY_MS);
    const un = listen<string>("menu", (e) => {
      if (e.payload === "check-updates") run(true);
    });
    return () => {
      window.clearTimeout(first);
      window.clearInterval(timer);
      un.then((f) => f());
    };
  }, [run]);

  const dismiss = (
    <button className="btn ghost icon-btn" style={{ height: 20, width: 20 }} aria-label="Dismiss" onClick={() => setState({ kind: "idle" })}>
      <Close size={11} />
    </button>
  );

  switch (state.kind) {
    case "checking":
      return state.manual ? <span className="muted update-pill">Checking for updates…</span> : null;
    case "downloading":
      return (
        <span className="update-pill" role="status">
          <Refresh size={12} /> Downloading MongoBoard {state.version}
          {state.percent !== null ? ` · ${state.percent}%` : "…"}
        </span>
      );
    case "ready":
      return (
        <span className="update-pill ready" role="status">
          MongoBoard {state.version} is installed
          <button className="btn primary" style={{ height: 24 }} onClick={() => relaunch()}>
            Restart now
          </button>
          {dismiss}
        </span>
      );
    case "upToDate":
      return (
        <span className="update-pill" role="status">
          You're on the latest version {dismiss}
        </span>
      );
    case "error":
      return (
        <span className="update-pill error selectable" role="status" title={state.message}>
          Update check failed: {state.message.slice(0, 80)} {dismiss}
        </span>
      );
    default:
      return null;
  }
}
