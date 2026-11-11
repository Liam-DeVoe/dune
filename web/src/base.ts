// The path the app is served under: "/" by default, "/beta/" when built with TREACHERY_BASE=/beta/
// (vite.config.ts). Every absolute URL the app makes goes through withBase. The server behind the
// prefix needs no setting: the reverse proxy strips it (deploy/Caddyfile).
export const BASE: string = import.meta.env.BASE_URL;

/** `path` ("art/x.png" or "/art/x.png") under the base path. */
export function withBase(path: string): string {
  return BASE + path.replace(/^\//, "");
}
