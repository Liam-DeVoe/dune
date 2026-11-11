// .NET string helpers the C# lobby forms validate with, so the web forms accept and refuse the
// same input.

// Char.IsWhiteSpace: Unicode Zs, Zl, Zp, U+0009..U+000D, U+0085. JavaScript's trim() differs on
// U+0085 (not trimmed) and U+FEFF (trimmed).
const WHITE_CODES = new Set([0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x20, 0x85, 0xa0, 0x1680, 0x2000, 0x2001, 0x2002, 0x2003, 0x2004, 0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200a, 0x2028, 0x2029, 0x202f, 0x205f, 0x3000]);
const WHITE = { test: (c: string) => WHITE_CODES.has(c.charCodeAt(0)) };

/** `String.Trim()`. */
export function netTrim(s: string): string {
  let start = 0;
  let end = s.length;
  while (start < end && WHITE.test(s[start])) start++;
  while (end > start && WHITE.test(s[end - 1])) end--;
  return s.slice(start, end);
}

/** `s.Trim().Length`, in UTF-16 units like .NET. */
export function netTrimLength(s: string | null | undefined): number {
  return s == null ? 0 : netTrim(s).length;
}

export function isNullOrEmpty(s: string | null | undefined): boolean {
  return s == null || s.length === 0;
}

/** `String.IsNullOrWhiteSpace`. */
export function isNullOrWhiteSpace(s: string | null | undefined): boolean {
  return s == null || netTrim(s).length === 0;
}

/** `PlayerJoined.ValidName`. */
export function validName(name: string | null | undefined): string {
  if (isNullOrEmpty(name)) return "Please enter a name.";
  if (/[&#"<>]/.test(name!)) return "Name cannot contain &, #, \", < or >";
  return "";
}
