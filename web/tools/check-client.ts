// Checks client helpers that need no browser: the lobby's participation order and name lengths.
//
//   npm run check:client
//
// Exits 1 on any failure.
import { resolve } from "node:path";
import { createServer } from "vite";

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

  if (failures.length > 0) {
    console.log(failures.join("\n"));
    process.exitCode = 1;
  } else console.log("check-client: all cases pass");
} finally {
  await server.close();
}
