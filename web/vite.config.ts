// Vite config for the React client. See rust/docs/client-plan.md.
//
// The look must match the C# client, so the app uses the C# client's own compiled CSS and art
// from treachery-online/Client/wwwroot, unchanged. They are served from there, not copied: in dev
// and preview by the middleware below, in production by the Rust server, which serves the same
// directories next to dist/.
import { createReadStream, existsSync, statSync } from "node:fs";
import { extname, join, normalize, resolve } from "node:path";
import type { Connect, Plugin } from "vite";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const wwwroot = resolve(__dirname, "../treachery-online/Client/wwwroot");
// The path the app is served under, "/" by default. deploy/deploy-beta.sh builds with
// TREACHERY_BASE=/beta/; the app's URLs follow it through import.meta.env.BASE_URL (src/base.ts).
const base = (process.env.TREACHERY_BASE ?? "/").replace(/\/*$/, "/");
// The wwwroot paths the C# index.html and skin use.
const shared = /^\/(art|css|fonts|js|docs)\/|^\/[^/]+\.ttf$|^\/favicon\.ico$/;
const types: Record<string, string> = {
  ".css": "text/css", ".js": "text/javascript", ".svg": "image/svg+xml", ".png": "image/png", ".jpg": "image/jpeg",
  ".gif": "image/gif", ".webp": "image/webp", ".ttf": "font/ttf", ".woff": "font/woff", ".woff2": "font/woff2",
  ".mp3": "audio/mpeg", ".ico": "image/x-icon", ".pdf": "application/pdf",
};

const unbased = (path: string) => path.slice(base.length - 1);

function serveWwwroot(): Connect.NextHandleFunction {
  return (req, res, next) => {
    const url = decodeURIComponent((req.url ?? "").split("?")[0]);
    if (!url.startsWith(base)) return next();
    const path = url.slice(base.length - 1);
    if (!shared.test(path)) return next();
    const file = normalize(join(wwwroot, path));
    if (!file.startsWith(wwwroot) || !existsSync(file) || !statSync(file).isFile()) return next();
    res.setHeader("Content-Type", types[extname(file).toLowerCase()] ?? "application/octet-stream");
    createReadStream(file).pipe(res);
  };
}

const wwwrootAssets: Plugin = {
  name: "treachery-wwwroot",
  configureServer: (server) => { server.middlewares.use(serveWwwroot()); },
  configurePreviewServer: (server) => { server.middlewares.use(serveWwwroot()); },
};

export default defineConfig({
  base,
  plugins: [react(), wwwrootAssets],
  // In dev, the Rust server (default http://localhost:5000, override with TREACHERY_SERVER) answers
  // the WebSocket and the dev scene endpoint.
  server: {
    port: 5173,
    strictPort: true,
    // The server is mounted at "/", as behind the deploy's reverse proxy, so the base is stripped.
    proxy: {
      [`${base}ws`]: { target: process.env.TREACHERY_SERVER ?? "http://localhost:5000", ws: true, rewrite: unbased },
      [`${base}dev`]: { target: process.env.TREACHERY_SERVER ?? "http://localhost:5000", rewrite: unbased },
    },
  },
});
