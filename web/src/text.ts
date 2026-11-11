// .NET text helpers the C# client's markup relies on: WebUtility.HtmlEncode, the client's
// Support.Round, and the regular expression of Skin.WithSpiceIcons. Ported from rust/skin
// (text.rs), which is checked byte for byte against C#.

// System.Net.WebUtility.HtmlEncode: < > & " ' as entities, and U+00A0..U+00FF and characters
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

// Math.Round(x) with MidpointRounding.ToEven.
function roundHalfEven(x: number): number {
  const r = Math.round(x);
  return Math.abs(x % 1) === 0.5 && r % 2 !== 0 ? r - 1 : r;
}

// Client/Support.Round: Math.Round(x, 3) (to even, .NET's power-of-ten algorithm), printed in
// the invariant culture. JavaScript's shortest round-trip form is .NET's for these magnitudes;
// .NET prints negative zero as "-0".
export function supportRound(x: number): string {
  const v = Math.abs(x) < 1e16 ? roundHalfEven(x * 1000) / 1000 : x;
  return Object.is(v, -0) ? "-0" : String(v);
}

// Shared Utilities.Round: the double's invariant ToString, no rounding.
export function utilitiesRound(x: number): string {
  return Object.is(x, -0) ? "-0" : String(x);
}

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

// The regular expression of Skin.WithSpiceIcons:
//   \b(\d+) (?:spice|{name})\b(?! (?:cards?|blows?|decks?)\b), case-insensitive,
// where name is Describe(Concept.Resource). JavaScript's \b and \d are ASCII-only; .NET's are
// Unicode, which only matters outside the ASCII skin texts.
export function replaceSpiceAmounts(text: string, name: string, replace: (digits: string) => string): string {
  const re = new RegExp(`\\b(\\d+) (?:spice|${escapeRegExp(name)})\\b(?! (?:cards?|blows?|decks?)\\b)`, "gi");
  return text.replace(re, (_m, digits: string) => replace(digits));
}
