// Checks client helpers that need no browser: the lobby's participation order, name lengths, and
// the turn popover's times and markup.
//
//   npm run check:client
//
// Exits 1 on any failure.
import { resolve } from "node:path";
import { createServer } from "vite";

// The turn popover prints local times, so the check fixes the time zone (UTC+5:30, no daylight
// saving) to tell them from UTC.
process.env.TZ = "Asia/Kolkata";

const web = resolve(import.meta.dirname, "..");
const failures: string[] = [];
function expect(what: string, actual: unknown, expected: unknown) {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) failures.push(`${what}\n  expected ${JSON.stringify(expected)}\n  actual   ${JSON.stringify(actual)}`);
}

const server = await createServer({
  root: web,
  configFile: resolve(web, "vite.config.ts"),
  server: { middlewareMode: true },
  appType: "custom",
  logLevel: "error",
  // No dependency scan of the whole app, which this doesn't load.
  optimizeDeps: { noDiscovery: true, include: [] },
});
try {
  const { Participation } = await server.ssrLoadModule("/src/lobby/participation.ts");
  const { trimmedLength } = await server.ssrLoadModule("/src/lobby/text.ts");
  const popups = await server.ssrLoadModule("/src/components/board/popups.ts");

  // Participation: a user who joins after another left is listed last, as the server lists them
  // (IndexMap with shift_remove), so a reload shows the same order.
  {
    const p = new Participation({ playerNames: [[1, "Alice"], [2, "Bob"], [3, "Carol"]], seatedPlayers: [[1, 0], [2, 1], [3, 2]],
      observers: [], hosts: [1], availableSeats: [], kicked: [], botsSpeed: 1 });
    p.removeUser(2, false);
    p.addPlayer(4, "Dave", 1);
    expect("seated players after Bob leaves and Dave joins", p.seatedPlayers.keys(), [1, 3, 4]);
    expect("player names after Bob leaves and Dave joins", p.playerNames.values(), ["Alice", "Carol", "Dave"]);
    // A user who is there keeps their place when set again.
    p.addPlayer(1, "Alice", 5);
    expect("a seated user who takes another seat keeps their place", p.seatedPlayers.items(), [[1, 5], [3, 2], [4, 1]]);
    const c = p.clone();
    c.removeUser(3, false);
    expect("a clone is separate", [p.seatedPlayers.keys(), c.seatedPlayers.keys()], [[1, 3, 4], [1, 4]]);
    expect("toJson keeps the order", c.seatedPlayers.toJson(), [[1, 5], [4, 1]]);
  }

  // Name lengths count code points, as the server does (chars().count()), and ignore white space
  // at the ends.
  expect("trimmed length of ' abcd '", trimmedLength(" abcd "), 4);
  expect("trimmed length of four emoji", trimmedLength("😀😀😀😀"), 4);
  expect("trimmed length of null", trimmedLength(null), 0);

  // Durations: hours don't wrap at a day.
  expect("hms of 25 hours", popups.hms(25 * 3600 + 61), "25:01:01");
  expect("hms of 59 seconds", popups.hms(59.9), "0:00:59");
  // Times are in the browser's time zone: 2026-11-02T03:35:00Z is 9:05 in Kolkata.
  expect("longDate in the local time zone", popups.longDate("2026-11-02T03:35:00Z"), "Monday, 02 November 2026 9:05:00");
  expect("longDate of a day that differs from UTC's", popups.longDate("2026-11-01T20:00:00Z"), "Monday, 02 November 2026 1:30:00");

  // The turn popover's timer labels are bold.
  {
    const view = {
      version: 191,
      ruleLabels: { ruleset: "Basic game", groups: [] },
      timing: { started: "2026-11-02T03:35:00Z", lastActivity: "2026-11-03T03:35:00Z", duration: 86400, timeSpent: [{ faction: "Atreides", bidding: 90000, shipmentAndMove: 0, battle: 0 }], timer: -1 },
      players: [{ faction: "Atreides", name: "Alice" }],
    };
    const html: string = popups.turn({ view });
    expect("no <string> tags in the turn popover", html.includes("<string>"), false);
    for (const label of ["Game started:", "Last activity:", "Game duration:"]) {
      expect(`"${label}" is bold`, html.includes(`<strong>${label}</strong>`), true);
    }
    expect("a player's 25 hours of bidding", html.includes("<td>25:00:00</td>"), true);
  }

  if (failures.length > 0) {
    console.log(failures.join("\n"));
    process.exitCode = 1;
  } else console.log("check-client: all cases pass");
} finally {
  await server.close();
}
