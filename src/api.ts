// Typed wrappers around the Rust commands in src-tauri/src/commands.rs.
import { invoke as tauriInvoke } from "@tauri-apps/api/core";

// VITE_DEMO=1 swaps the Rust backend for sample data (README screenshots).
const invoke: typeof tauriInvoke =
  import.meta.env.VITE_DEMO === "1"
    ? (cmd, args) =>
        cmd.startsWith("open_connection_") || cmd === "parse_connection_string"
          ? tauriInvoke(cmd, args) // window management and pure parsing stay real in demo mode
          : import("./demo").then((m) => m.demoInvoke(cmd, args as Record<string, unknown>))
    : tauriInvoke;

export type EnvTag = "production" | "qa" | "dev" | "local" | "none";
export type AuthKind = "none" | "password" | "aws";

export interface Profile {
  id: string;
  name: string;
  uri: string;
  auth: AuthKind;
  username?: string | null;
  authSource?: string | null;
  authMechanism?: string | null;
  awsProfile?: string | null;
  tag: EnvTag;
  readOnly: boolean;
  confirmProdWrites: boolean;
  defaultDatabase?: string | null;
  lastUsed?: number | null;
}

export interface ProfileView extends Profile {
  hasPassword: boolean;
}

export interface ParsedUri {
  uri: string;
  name?: string | null;
  username?: string | null;
  password?: string | null;
  auth?: AuthKind | null;
  authSource?: string | null;
  authMechanism?: string | null;
  awsProfile?: string | null;
  defaultDatabase?: string | null;
  tag?: EnvTag | null;
  notes: string[];
}

export interface ServerInfo {
  version: string;
  topology: string;
  latencyMs: number;
}

export interface OpenConnection {
  profile: ProfileView;
  server: ServerInfo;
}

export interface DatabaseInfo {
  name: string;
  sizeOnDisk: number;
}

export interface CollectionInfo {
  name: string;
  kind: "collection" | "view" | "timeseries";
}

export interface TreeNode {
  key: string;
  typeName: string;
  value: string;
  children?: TreeNode[];
}

export interface ResultDoc {
  id: string | null;
  shell: string;
  shellWithoutId: string;
  tree: TreeNode;
}

export interface FindResult {
  docs: ResultDoc[];
  elapsedMs: number;
  skip: number;
  hasMore: boolean;
}

export interface FindRequest {
  db: string;
  collection: string;
  filter: string;
  projection: string;
  sort: string;
  skip: number;
  limit: number;
}

/** Errors from Rust arrive as plain strings. */
export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return String(e);
}

export const api = {
  listProfiles: () => invoke<ProfileView[]>("list_profiles"),
  getProfile: (id: string) => invoke<ProfileView>("get_profile", { id }),
  saveProfile: (profile: Profile, password: string | null) =>
    invoke<ProfileView>("save_profile", { input: { ...profile, password } }),
  duplicateProfile: (id: string) => invoke<ProfileView>("duplicate_profile", { id }),
  deleteProfile: (id: string) => invoke<void>("delete_profile", { id }),
  parseConnectionString: (uri: string) => invoke<ParsedUri>("parse_connection_string", { uri }),
  testConnection: (profile: Profile, password: string | null) =>
    invoke<ServerInfo>("test_connection", { input: { ...profile, password } }),

  openConnectionManager: () => invoke<void>("open_connection_manager"),
  openConnectionEditor: (id: string | null) => invoke<void>("open_connection_editor", { id }),
  openConnection: (id: string) => invoke<ServerInfo>("open_connection", { id }),
  listOpenConnections: () => invoke<OpenConnection[]>("list_open_connections"),
  closeConnection: (id: string) => invoke<void>("close_connection", { id }),
  reconnect: (id: string) => invoke<ServerInfo>("reconnect", { id }),
  listDatabases: (id: string) => invoke<DatabaseInfo[]>("list_databases", { id }),
  listCollections: (id: string, db: string) => invoke<CollectionInfo[]>("list_collections", { id, db }),
  find: (id: string, request: FindRequest) => invoke<FindResult>("find", { id, request }),
  countDocuments: (id: string, db: string, collection: string, filter: string) =>
    invoke<number>("count_documents", { id, db, collection, filter }),
  formatDocument: (text: string) => invoke<string>("format_document", { text }),

  insertDocument: (id: string, db: string, collection: string, text: string) =>
    invoke<string>("insert_document", { id, db, collection, text }),
  replaceDocument: (id: string, db: string, collection: string, docId: string, text: string) =>
    invoke<number>("replace_document", { id, db, collection, docId, text }),
  deleteDocument: (id: string, db: string, collection: string, docId: string) =>
    invoke<number>("delete_document", { id, db, collection, docId }),
};

export const TAG_LABEL: Record<EnvTag, string> = {
  production: "PROD",
  qa: "QA",
  dev: "DEV",
  local: "LOCAL",
  none: "—",
};

export function newProfile(id: string): Profile {
  return {
    id,
    name: "",
    uri: "mongodb://localhost:27017",
    auth: "none",
    username: "",
    authSource: "",
    authMechanism: "",
    awsProfile: "",
    tag: "none",
    readOnly: false,
    confirmProdWrites: true,
    defaultDatabase: "",
    lastUsed: null,
  };
}

export function relativeTime(ms?: number | null): string {
  if (!ms) return "Never";
  const diff = Date.now() - ms;
  const min = Math.round(diff / 60000);
  if (min < 1) return "Just now";
  if (min < 60) return `${min} min ago`;
  const h = Math.round(min / 60);
  if (h < 24) return `${h} h ago`;
  const d = Math.round(h / 24);
  if (d === 1) return "Yesterday";
  if (d < 30) return `${d} days ago`;
  return new Date(ms).toLocaleDateString();
}
