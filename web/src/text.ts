// Text helpers for markup built as strings: HTML encoding, rounding numbers for display, and the
// regular expression that finds spice amounts. They follow rust/skin's text.rs, so the popovers
// built here read the same as the server's.

// < > & " ' as entities, and U+00A0..U+00FF and characters
// outside the BMP as &#N;.
export function htmlEncode(text: string): string {
  let out = "";
  for (const ch of text) {
    const c = ch.codePointAt(0)!;
    switch (ch) {
      case "<": out += "&lt;"; break;
      case ">": out += "&gt;"; break;
      case '"': out += "&quot;"; break;
      case "'": out += "&#39;"; break;
      case "&": out += "&amp;"; break;
      default: out += (c >= 0xa0 && c <= 0xff) || c >= 0x10000 ? `&#${c};` : ch;
    }
  }
  return out;
}

// Rounds to the nearest whole number, halves to even.
function roundHalfEven(x: number): number {
  const r = Math.round(x);
  return Math.abs(x % 1) === 0.5 && r % 2 !== 0 ? r - 1 : r;
}

// Rounds to 3 decimals (halves to even) and prints the shortest form. Negative zero prints as
// "-0".
export function supportRound(x: number): string {
  const v = Math.abs(x) < 1e16 ? roundHalfEven(x * 1000) / 1000 : x;
  return Object.is(v, -0) ? "-0" : String(v);
}

// Prints a number as it is, without rounding. Negative zero prints as "-0".
export function utilitiesRound(x: number): string {
  return Object.is(x, -0) ? "-0" : String(x);
}

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

// The regular expression that finds spice amounts in a text:
//   \b(\d+) (?:spice|{name})\b(?! (?:cards?|blows?|decks?)\b), case-insensitive,
// where name is Describe(Concept.Resource). JavaScript's \b and \d are ASCII-only, rust/skin's are
// Unicode, which only matters outside the ASCII skin texts.
export function replaceSpiceAmounts(text: string, name: string, replace: (digits: string) => string): string {
  const re = new RegExp(`\\b(\\d+) (?:spice|${escapeRegExp(name)})\\b(?! (?:cards?|blows?|decks?)\\b)`, "gi");
  return text.replace(re, (_m, digits: string) => replace(digits));
}
