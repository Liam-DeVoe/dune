// String helpers the lobby forms validate with. Whitespace is Unicode's White_Space, as in Rust's
// str::trim, which the server trims names with.

// White space: Unicode Zs, Zl, Zp, U+0009..U+000D, U+0085. JavaScript's trim() differs on
// U+0085 (not trimmed) and U+FEFF (trimmed).
const WHITE_CODES = new Set([0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x20, 0x85, 0xa0, 0x1680, 0x2000, 0x2001, 0x2002, 0x2003, 0x2004, 0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200a, 0x2028, 0x2029, 0x202f, 0x205f, 0x3000]);
const WHITE = { test: (c: string) => WHITE_CODES.has(c.charCodeAt(0)) };

/** Trims white space from both ends. */
export function trimWhitespace(s: string): string {
  let start = 0;
  let end = s.length;
  while (start < end && WHITE.test(s[start])) start++;
  while (end > start && WHITE.test(s[end - 1])) end--;
  return s.slice(start, end);
}

/** The trimmed length in UTF-16 units; 0 for null. */
export function trimmedLength(s: string | null | undefined): number {
  return s == null ? 0 : trimWhitespace(s).length;
}

export function isNullOrEmpty(s: string | null | undefined): boolean {
  return s == null || s.length === 0;
}

/** Null, empty, or only white space. */
export function isNullOrWhiteSpace(s: string | null | undefined): boolean {
  return s == null || trimWhitespace(s).length === 0;
}

/** The error text for a name, or "" when it is valid. */
export function validName(name: string | null | undefined): string {
  if (isNullOrEmpty(name)) return "Please enter a name.";
  if (/[&#"<>]/.test(name!)) return "Name cannot contain &, #, \", < or >";
  return "";
}
