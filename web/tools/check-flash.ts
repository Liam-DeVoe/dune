// Checks when ImageFlash starts a flash (src/flashTrigger.ts): on the first view when it has one,
// after each new event that brings one, and never for a view that is not a new event.
//
//   npm run check:flash
//
// Exits 1 on any failure.
import { initialFlash, nextFlash, type FlashState } from "../src/flashTrigger.ts";

const failures: string[] = [];
function expect(what: string, ok: boolean) {
  if (!ok) failures.push(what);
}

type S = FlashState<string>;
// The states after the first view and after each later view [eventNumber, flash].
function run(first: [number, string[]], views: [number, string[]][]): { states: S[] } {
  let s: S = initialFlash(first[0], first[1]);
  const states = [s];
  for (const [n, f] of views) {
    s = nextFlash(s, n, f);
    states.push(s);
  }
  return { states };
}

// Opening the page with a current flash shows it, for 5000 + 1000 per item.
{
  const s = initialFlash(10, ["a", "b"]);
  expect("opened with a flash: shown", s.key === 1 && s.onOpen && s.time === 7000);
  expect("opened without a flash: nothing", initialFlash(10, []).time === 0);
}

// A new event with a flash starts one; a later one replaces it.
{
  const { states } = run([10, []], [[11, ["a"]], [12, ["b", "c", "d"]]]);
  expect("event 11 flashes", states[1].key === 1 && !states[1].onOpen && states[1].time === 6000 && states[1].items[0] === "a");
  expect("event 12 replaces it", states[2].key === 2 && states[2].time === 8000 && states[2].items.length === 3);
}

// The same event again (draft reply, chat, reconnect re-send) changes nothing, even with a flash.
{
  const { states } = run([10, []], [[11, ["a"]], [11, ["a"]], [11, ["a"]]]);
  expect("re-sent view: same state", states[2] === states[1] && states[3] === states[1]);
  const opened = run([11, ["a"]], [[11, ["a"]]]).states;
  expect("re-sent view after opening: same state", opened[1] === opened[0]);
}

// An event without a flash leaves the shown one alone (it hides by its timer or a click).
{
  const { states } = run([10, []], [[11, ["a"]], [12, []]]);
  expect("event without a flash: no new flash", states[2].key === 1 && states[2].items[0] === "a" && states[2].lastEvent === 12);
}

// An undo goes back to an earlier event: no flash, though that event had one. The next new event
// after it flashes again.
{
  const { states } = run([10, []], [[11, ["a"]], [12, ["b"]], [11, ["a"]], [12, ["c"]]]);
  expect("undo: no flash", states[3].key === 2 && states[3].lastEvent === 11);
  expect("new event after undo flashes", states[4].key === 3 && states[4].items[0] === "c");
}

if (failures.length > 0) {
  for (const f of failures) console.error(`FAIL ${f}`);
  process.exit(1);
}
console.log("check-flash: all cases pass");
