// Checks that the React RichText shows the expected text for every log line of the scene
// fixtures and of the bot-game sample. It renders each line's parts (`text`, which
// rust/view/tests/log_lines.rs checks the view still produces) and compares the text content with
// the line's `dom`, the text the log must show.
//
//   npm run check:rich-text
//
// The snapshot files are rust/view/tests/golden/log_lines*.jsonl.gz (see log_lines.rs for how to
// rewrite them). Exits 1 on any difference.
import { readFileSync } from "node:fs";
import { gunzipSync } from "node:zlib";
import { resolve } from "node:path";
import { createServer } from "vite";

const web = resolve(import.meta.dirname, "..");
const golden = resolve(web, "../rust/view/tests/golden");

const lines: { where: string; text: unknown[]; dom: string }[] = [];
for (const file of ["log_lines.jsonl.gz", "log_lines_bots.jsonl.gz"]) {
  for (const l of gunzipSync(readFileSync(resolve(golden, file))).toString("utf8").split("\n")) {
    if (!l) continue;
    const o = JSON.parse(l);
    lines.push({ where: `${o.fixture ?? o.trace} report ${o.report} line ${o.line}`, text: o.text, dom: o.dom });
  }
}

const server = await createServer({
  root: web,
  configFile: resolve(web, "vite.config.ts"),
  server: { middlewareMode: true },
  appType: "custom",
  logLevel: "error",
});
try {
  const mod = await server.ssrLoadModule("/tools/rich-text-check.tsx");
  const skin = JSON.parse(readFileSync(resolve(web, "public/skin/default.json"), "utf8"));
  const statics = JSON.parse(readFileSync(resolve(web, "public/static/data.json"), "utf8"));
  const rendered: { text: string; html: string }[] = mod.render(lines.map((l) => l.text), skin, statics);
  const failures: string[] = [];
  lines.forEach((l, i) => {
    if (rendered[i].text !== l.dom) failures.push(`${l.where}\n  expected ${JSON.stringify(l.dom)}\n  react    ${JSON.stringify(rendered[i].text)}`);
  });
  console.log(`${lines.length - failures.length} of ${lines.length} log lines show the expected text.`);
  if (failures.length > 0) {
    console.log(failures.slice(0, 20).join("\n"));
    process.exitCode = 1;
  }
} finally {
  await server.close();
}
