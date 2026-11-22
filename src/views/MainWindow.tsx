import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, PROTECTED_DATABASES, CollectionInfo, DatabaseInfo, errorText, OpenConnection, ProfileView, relativeTime, TAG_LABEL } from "../api";
import { CollectionView } from "../components/CollectionView";
import { Chevron, Close, Collection, Database, Link, Plus, Refresh, Search, Server, Trash } from "../components/Icons";

interface Tab {
  key: string;
  connId: string;
  db: string;
  collection: string;
}

type Loadable<T> = T | "loading" | { error: string };
const isList = <T,>(v: Loadable<T[]> | undefined): v is T[] => Array.isArray(v);
const isError = (v: unknown): v is { error: string } => !!v && typeof v === "object" && "error" in (v as object);

function formatBytes(n: number): string {
  if (!n) return "";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v < 10 && i > 0 ? 1 : 0)} ${units[i]}`;
}

function hostOf(uri: string): string {
  const rest = uri.replace(/^mongodb(\+srv)?:\/\//, "");
  return rest.split(/[/?]/)[0] || uri;
}

export function MainWindow() {
  const [conns, setConns] = useState<OpenConnection[]>([]);
  const [dbs, setDbs] = useState<Record<string, Loadable<DatabaseInfo[]>>>({});
  const [cols, setCols] = useState<Record<string, Loadable<CollectionInfo[]>>>({});
  const [open, setOpen] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState("");
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activeTab, setActiveTab] = useState<string>("quickstart");
  const [error, setError] = useState<string | null>(null);
  const [dropTarget, setDropTarget] = useState<{ conn: OpenConnection; db: string } | null>(null);
  const run = (p: Promise<unknown>) => p.catch((e) => setError(errorText(e)));

  // ---------------------------------------------------------------- loading

  const loadDbs = useCallback(async (connId: string) => {
    setDbs((d) => ({ ...d, [connId]: "loading" }));
    try {
      const list = await api.listDatabases(connId);
      setDbs((d) => ({ ...d, [connId]: list }));
      return list;
    } catch (e) {
      setDbs((d) => ({ ...d, [connId]: { error: errorText(e) } }));
      return [];
    }
  }, []);

  const loadCols = useCallback(async (connId: string, db: string) => {
    const key = `${connId}/${db}`;
    setCols((c) => ({ ...c, [key]: "loading" }));
    try {
      const list = await api.listCollections(connId, db);
      setCols((c) => ({ ...c, [key]: list }));
    } catch (e) {
      setCols((c) => ({ ...c, [key]: { error: errorText(e) } }));
    }
  }, []);

  const refreshConnections = useCallback(async () => {
    const list = await api.listOpenConnections();
    setConns((prev) => {
      // Newly opened connections: expand them and their default database.
      for (const c of list) {
        if (!prev.some((p) => p.profile.id === c.profile.id)) {
          const id = c.profile.id;
          loadDbs(id).then((found) => {
            const def = c.profile.defaultDatabase;
            if (def && found.some((d) => d.name === def)) {
              setOpen((o) => new Set(o).add(`d:${id}/${def}`));
              loadCols(id, def);
            }
          });
          setOpen((o) => new Set(o).add(`c:${id}`));
        }
      }
      return list;
    });
    // Close tabs of connections that went away.
    setTabs((t) => t.filter((tab) => list.some((c) => c.profile.id === tab.connId)));
  }, [loadDbs, loadCols]);

  useEffect(() => {
    refreshConnections();
    const un1 = listen("connections-changed", refreshConnections);
    const un2 = listen("profiles-changed", refreshConnections);
    return () => {
      un1.then((f) => f());
      un2.then((f) => f());
    };
  }, [refreshConnections]);

  useEffect(() => {
    if (activeTab !== "quickstart" && !tabs.some((t) => t.key === activeTab)) {
      setActiveTab(tabs.length ? tabs[tabs.length - 1].key : "quickstart");
    }
  }, [tabs, activeTab]);

  // ---------------------------------------------------------------- actions

  function toggle(key: string, onOpen?: () => void) {
    setOpen((o) => {
      const next = new Set(o);
      if (next.has(key)) next.delete(key);
      else {
        next.add(key);
        onOpen?.();
      }
      return next;
    });
  }

  function openTab(connId: string, db: string, collection: string) {
    const key = `${connId}/${db}/${collection}`;
    setTabs((t) => (t.some((x) => x.key === key) ? t : [...t, { key, connId, db, collection }]));
    setActiveTab(key);
  }

  function closeTab(key: string) {
    setTabs((t) => t.filter((x) => x.key !== key));
  }

  async function disconnect(connId: string) {
    await run(api.closeConnection(connId));
    setTabs((t) => t.filter((x) => x.connId !== connId));
  }

  // ---------------------------------------------------------------- render

  const q = search.trim().toLowerCase();
  const connById = useMemo(() => Object.fromEntries(conns.map((c) => [c.profile.id, c])), [conns]);

  return (
    <div className="col" style={{ height: "100%" }}>
      <div className="toolbar" role="toolbar" aria-label="Main" style={{ background: "var(--panel-2)", padding: "8px 12px" }}>
        <button className="btn" onClick={() => run(api.openConnectionManager())}>
          <Link /> Connect…
        </button>
        <button className="btn" onClick={() => run(api.openConnectionEditor(null))}>
          <Plus /> New connection
        </button>
        {error && (
          <span className="row banner error" style={{ padding: "4px 10px" }}>
            {error}
            <button className="btn ghost icon-btn" style={{ height: 20, width: 20 }} aria-label="Dismiss" onClick={() => setError(null)}>
              <Close size={11} />
            </button>
          </span>
        )}
      </div>

      <div className="row" style={{ flex: 1, minHeight: 0, alignItems: "stretch", gap: 0 }}>
        {/* Sidebar: open connections */}
        <nav aria-label="Open connections" className="col sidebar">
          <label className="row input" style={{ height: 30, margin: "10px 10px 6px", gap: 8, color: "var(--muted)" }}>
            <Search />
            <input
              type="search"
              placeholder="Search open connections"
              aria-label="Search open connections"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              style={{ flex: 1, minWidth: 0, border: 0, background: "transparent", outline: "none", fontSize: 12 }}
            />
          </label>
          <div style={{ flex: 1, overflow: "auto", padding: "0 6px 10px" }} role="tree">
            {conns.length === 0 && (
              <div className="muted" style={{ padding: "18px 10px", fontSize: 12, lineHeight: 1.5 }}>
                No open connections.
                <br />
                <button className="btn" style={{ marginTop: 10 }} onClick={() => run(api.openConnectionManager())}>
                  <Link /> Connect…
                </button>
              </div>
            )}
            {conns.map(({ profile: p, server }) => {
              const cKey = `c:${p.id}`;
              const cOpen = open.has(cKey);
              const dbList = dbs[p.id];
              return (
                <div key={p.id} role="treeitem" aria-expanded={cOpen}>
                  <div className="row tree-item conn" style={{ paddingLeft: 4 }}>
                    <button className="row tree-btn" onClick={() => toggle(cKey)} title={`${p.name} · ${server.topology} · MongoDB ${server.version}`}>
                      <span className={`chev ${cOpen ? "open" : ""}`} aria-hidden="true"><Chevron /></span>
                      <Server />
                      <span className={`dot ${p.tag}`} />
                      <span className="ellipsis" style={{ fontWeight: 600, flexShrink: 0, maxWidth: "65%" }}>{p.name}</span>
                      <span className="ellipsis muted" style={{ fontSize: 11, flex: 1 }}>{server.topology}</span>
                    </button>
                    <span className="tree-actions">
                      <button className="btn ghost icon-btn" style={{ height: 22, width: 22 }} aria-label={`Refresh ${p.name}`} onClick={() => loadDbs(p.id)}>
                        <Refresh size={12} />
                      </button>
                      <button className="btn ghost icon-btn" style={{ height: 22, width: 22 }} aria-label={`Disconnect ${p.name}`} title="Disconnect" onClick={() => disconnect(p.id)}>
                        <Close size={12} />
                      </button>
                    </span>
                  </div>
                  {cOpen && dbList === "loading" && <div className="tree-note" style={{ paddingLeft: 48 }}>Loading…</div>}
                  {cOpen && isError(dbList) && (
                    <div className="banner error" style={{ margin: "4px 6px 4px 28px" }}>
                      {dbList.error}
                      <div style={{ marginTop: 6 }}>
                        <button className="btn" onClick={async () => { await api.reconnect(p.id).catch(() => null); loadDbs(p.id); }}>Reconnect</button>
                      </div>
                    </div>
                  )}
                  {cOpen && isList(dbList) &&
                    dbList.map((d) => {
                      const dKey = `d:${p.id}/${d.name}`;
                      const dOpen = open.has(dKey);
                      const colList = cols[`${p.id}/${d.name}`];
                      const matchingCols = isList(colList) ? colList.filter((c) => !q || c.name.toLowerCase().includes(q)) : [];
                      if (q && !d.name.toLowerCase().includes(q) && matchingCols.length === 0) return null;
                      const showOpen = dOpen || (!!q && matchingCols.length > 0);
                      return (
                        <div key={d.name} role="treeitem" aria-expanded={showOpen}>
                          <div className="row tree-item has-actions">
                            <button
                              className="row tree-btn"
                              style={{ paddingLeft: 22 }}
                              onClick={() => toggle(dKey, () => !isList(colList) && loadCols(p.id, d.name))}
                            >
                              <span className={`chev ${showOpen ? "open" : ""}`} aria-hidden="true"><Chevron /></span>
                              <Database />
                              <span className="ellipsis" style={{ flex: 1 }}>{d.name}</span>
                              <span className="muted" style={{ fontSize: 10.5 }}>{formatBytes(d.sizeOnDisk)}</span>
                            </button>
                            <span className="tree-actions" style={{ width: 26, justifyContent: "flex-end" }}>
                              {!p.readOnly && !PROTECTED_DATABASES.includes(d.name) && (
                                <button
                                  className="btn ghost icon-btn danger"
                                  style={{ height: 22, width: 22 }}
                                  aria-label={`Drop database ${d.name}`}
                                  title="Drop database…"
                                  onClick={() => setDropTarget({ conn: connById[p.id], db: d.name })}
                                >
                                  <Trash size={12} />
                                </button>
                              )}
                            </span>
                          </div>
                          {showOpen && colList === "loading" && <div className="tree-note" style={{ paddingLeft: 66 }}>Loading…</div>}
                          {showOpen && isError(colList) && <div className="banner error" style={{ margin: "4px 6px 4px 46px" }}>{colList.error}</div>}
                          {showOpen && isList(colList) && colList.length === 0 && <div className="tree-note" style={{ paddingLeft: 66 }}>No collections</div>}
                          {showOpen &&
                            matchingCols.map((c) => {
                              const key = `${p.id}/${d.name}/${c.name}`;
                              return (
                                <button
                                  key={c.name}
                                  role="treeitem"
                                  className={`row tree-btn tree-item ${activeTab === key ? "active" : ""}`}
                                  style={{ paddingLeft: 60 }}
                                  onClick={() => openTab(p.id, d.name, c.name)}
                                >
                                  <Collection size={13} />
                                  <span className="ellipsis" style={{ flex: 1 }}>{c.name}</span>
                                  {c.kind !== "collection" && <span className="muted" style={{ fontSize: 10.5 }}>{c.kind}</span>}
                                </button>
                              );
                            })}
                        </div>
                      );
                    })}
                </div>
              );
            })}
          </div>
        </nav>

        {/* Tabs */}
        <main className="col" style={{ flex: 1, minWidth: 0 }}>
          <div role="tablist" className="row tabbar">
            <button role="tab" aria-selected={activeTab === "quickstart"} className={`tab ${activeTab === "quickstart" ? "on" : ""}`} onClick={() => setActiveTab("quickstart")}>
              Quickstart
            </button>
            {tabs.map((t) => {
              const c = connById[t.connId];
              return (
                <div key={t.key} className={`tab ${activeTab === t.key ? "on" : ""}`} title={`${c?.profile.name} › ${t.db}.${t.collection}`}>
                  <button role="tab" aria-selected={activeTab === t.key} className="row tab-label" onClick={() => setActiveTab(t.key)}>
                    {c && <span className={`dot ${c.profile.tag}`} />}
                    <span className="ellipsis">{t.collection}</span>
                    <span className="muted ellipsis" style={{ fontSize: 11 }}>{c?.profile.name}</span>
                  </button>
                  <button className="tab-close" aria-label={`Close ${t.collection}`} onClick={() => closeTab(t.key)}>
                    <Close size={11} />
                  </button>
                </div>
              );
            })}
          </div>

          {activeTab === "quickstart" && <Quickstart />}
          {tabs.map((t) => {
            const c = connById[t.connId];
            if (!c) return null;
            const on = activeTab === t.key;
            return (
              <div key={t.key} className="col" style={{ flex: 1, minHeight: 0, display: on ? "flex" : "none" }}>
                <div className="row crumb">
                  <span className={`dot ${c.profile.tag}`} />
                  <strong>{c.profile.name}</strong>
                  <span className="muted mono" style={{ fontSize: 12 }}>› {t.db} › {t.collection}</span>
                  <span className="spacer" />
                  <span className="muted" style={{ fontSize: 11 }}>MongoDB {c.server.version}</span>
                  {c.profile.tag !== "none" && (
                    <span className={`tag ${c.profile.tag}`}>
                      {TAG_LABEL[c.profile.tag]}
                      {c.profile.readOnly ? " · READ-ONLY" : ""}
                    </span>
                  )}
                  {c.profile.tag === "none" && c.profile.readOnly && <span className="tag">READ-ONLY</span>}
                </div>
                <CollectionView profile={c.profile} db={t.db} collection={t.collection} active={on} />
              </div>
            );
          })}
        </main>
      </div>
      {dropTarget && (
        <DropDatabaseDialog
          conn={dropTarget.conn}
          db={dropTarget.db}
          onCancel={() => setDropTarget(null)}
          onDropped={() => {
            const { conn, db } = dropTarget;
            const id = conn.profile.id;
            setDropTarget(null);
            setTabs((t) => t.filter((x) => !(x.connId === id && x.db === db)));
            setCols((c) => {
              const next = { ...c };
              delete next[`${id}/${db}`];
              return next;
            });
            loadDbs(id);
          }}
        />
      )}
    </div>
  );
}

function DropDatabaseDialog({ conn, db, onCancel, onDropped }: { conn: OpenConnection; db: string; onCancel: () => void; onDropped: () => void }) {
  const [typed, setTyped] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const prod = conn.profile.tag === "production";
  const matches = typed === db;

  async function drop() {
    if (!matches) return;
    setBusy(true);
    setError(null);
    try {
      await api.dropDatabase(conn.profile.id, db);
      onDropped();
    } catch (e) {
      setError(errorText(e));
      setBusy(false);
    }
  }

  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && !busy && onCancel()}>
      <form
        className="modal"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="drop-title"
        onSubmit={(e) => {
          e.preventDefault();
          drop();
        }}
        onKeyDown={(e) => e.key === "Escape" && !busy && onCancel()}
      >
        <h2 id="drop-title">
          Drop database <span className="mono">{db}</span>?
        </h2>
        <p>
          This permanently deletes the database <strong>{db}</strong> on <strong>{conn.profile.name}</strong>, with all of its collections, documents and indexes. It can't be undone.
        </p>
        {prod && <div className="banner error">⚠️ This is a <strong>Production</strong> connection.</div>}
        <label className="field">
          Type the database name to confirm
          <input className="input mono" value={typed} onChange={(e) => setTyped(e.target.value)} placeholder={db} autoFocus spellCheck={false} autoComplete="off" />
        </label>
        {error && <div className="banner error selectable">{error}</div>}
        <div className="row" style={{ justifyContent: "flex-end" }}>
          <button type="button" className="btn lg" onClick={onCancel} disabled={busy}>
            Cancel
          </button>
          <button type="submit" className="btn lg danger-solid" disabled={!matches || busy}>
            {busy ? "Dropping…" : "Drop database"}
          </button>
        </div>
      </form>
    </div>
  );
}

function Quickstart() {
  const [profiles, setProfiles] = useState<ProfileView[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    api.listProfiles().then(setProfiles, (e) => setError(errorText(e)));
  }, []);
  useEffect(() => {
    load();
    const un = listen("profiles-changed", load);
    return () => {
      un.then((f) => f());
    };
  }, [load]);

  const recent = [...profiles].sort((a, b) => (b.lastUsed ?? 0) - (a.lastUsed ?? 0)).slice(0, 5);

  async function connect(id: string) {
    setBusy(id);
    setError(null);
    try {
      await api.openConnection(id);
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(null);
    }
  }

  return (
    <div style={{ flex: 1, overflow: "auto" }}>
      <div className="col quickstart">
        <h1>
          Welcome to
          <br />
          MongoBoard
        </h1>
        <h2>Recent connections</h2>
        <p className="muted">Pick up where you left off. You can keep several connections open at the same time.</p>
        {error && <div className="banner error selectable">{error}</div>}
        {recent.length === 0 && <p className="muted">No saved connections yet.</p>}
        {recent.map((p) => (
          <button key={p.id} className="qs-card" onClick={() => connect(p.id)} disabled={busy !== null}>
            <Server size={18} />
            <span className="col" style={{ gap: 2, flex: 1, minWidth: 0, alignItems: "flex-start" }}>
              <span className="row" style={{ gap: 8 }}>
                <strong>{p.name}</strong>
                {p.tag !== "none" && <span className={`tag ${p.tag}`}>{TAG_LABEL[p.tag]}</span>}
              </span>
              <span className="muted mono ellipsis" style={{ fontSize: 11.5, maxWidth: "100%" }}>{hostOf(p.uri)}</span>
              <span className="muted" style={{ fontSize: 11.5 }}>
                {busy === p.id ? "Connecting…" : `Last used ${relativeTime(p.lastUsed).toLowerCase()}`}
              </span>
            </span>
          </button>
        ))}
        <div className="qs-sep" />
        <button className="qs-card qs-action" onClick={() => api.openConnectionManager()}>
          <Link size={16} /> Open Connection Manager
        </button>
        <button className="qs-card qs-action" onClick={() => api.openConnectionEditor(null)}>
          <Plus size={14} /> Create a new connection
        </button>
      </div>
    </div>
  );
}
