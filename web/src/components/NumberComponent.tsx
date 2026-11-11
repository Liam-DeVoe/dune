// GenericComponents/NumberComponent.razor: a number drawn over a symbol (a faction sigil, a card
// back), centred or in the bottom right corner on a small disc. Offsets are computed in single
// precision where C# does (0.4f * Size), so the CSS lengths come out as in C#.
import type { CSSProperties } from "react";
import { supportRound } from "../text";

export type NumberAlignment = "Center" | "BottomRight";

// C# double.ToString() of a float widened to double, or of a double.
const px = (x: number) => `${x}px`;
const f32 = (k: number, size: number) => Math.fround(Math.fround(k) * size);

// Callers pass plain "white" and "black"; those become the parchment and ink tokens.
function tok(color: string | null | undefined): string {
  if (color == null) return "none";
  switch (color.toLowerCase()) {
    case "white": return "var(--parchment)";
    case "black": return "var(--ink-0)";
    default: return color;
  }
}

export function NumberComponent({ value, symbolSrc, symbolBackgroundFillColor, symbolBackgroundBorderColor, numberColor = "black",
  numberBorderColor = "white", numberBorderWidth = 0, numberAlignment = "Center", numberBackgroundFillColor,
  numberBackgroundBorderColor, behindNumberSrc, size = 36, title = "", onClick }: {
  value: number;
  symbolSrc?: string;
  symbolBackgroundFillColor?: string;
  symbolBackgroundBorderColor?: string;
  numberColor?: string;
  numberBorderColor?: string;
  numberBorderWidth?: number;
  numberAlignment?: NumberAlignment;
  numberBackgroundFillColor?: string;
  numberBackgroundBorderColor?: string;
  behindNumberSrc?: string;
  size?: number;
  title?: string;
  onClick?: () => void;
}) {
  const symbolSize = `${size}px`;
  const center = numberAlignment === "Center";
  const numberBoxSize = center ? `${size}px` : `${Math.trunc(f32(0.6, size))}px`;
  const numberBoxOffsetX = center ? "0" : px(f32(0.4, size));
  const numberBoxOffsetY = center ? px(f32(0.05, size)) : px(f32(0.45, size));
  const behindSize = center ? px(1.2 * size) : px(0.7 * size);
  const behindOffsetX = center ? px(-0.1 * size) : px(f32(0.35, size));
  const behindOffsetY = center ? px(-0.1 * size) : px(f32(0.35, size));
  const fontSize = px(f32(0.7, size));
  const bgRadius = center ? "7.5" : "4.75";
  const bgOffset = center ? 8 : 11;
  const border: CSSProperties = numberBorderWidth > 0
    ? { WebkitTextStroke: `${supportRound(0.3 * numberBorderWidth)}px ${numberBorderColor}`, textShadow: `0 0 ${2 * numberBorderWidth}px ${numberBorderColor}` }
    : {};
  const abs = { position: "absolute", left: 0, top: 0, width: symbolSize } as const;
  return (
    <div style={{ display: "inline-block", position: "relative", height: symbolSize, width: symbolSize, ...(onClick ? { cursor: "pointer" } : {}) }}
         title={title} onClick={onClick}>
      {(symbolBackgroundFillColor != null || symbolBackgroundBorderColor != null) && (
        <svg style={{ ...abs, height: symbolSize }} width="32" height="32" viewBox="0 0 32 32">
          <circle cx="16" cy="16" r="15.5" strokeWidth="2%" style={{ stroke: tok(symbolBackgroundBorderColor), fill: tok(symbolBackgroundFillColor) }} />
        </svg>
      )}
      {symbolSrc != null && <img src={symbolSrc} style={abs} alt={String(value)} />}
      {(numberBackgroundFillColor != null || numberBackgroundBorderColor != null) && (
        <svg style={{ ...abs, height: symbolSize }} width="16" height="16" viewBox="0 0 16 16">
          <circle cx={bgOffset} cy={bgOffset} r={bgRadius} strokeWidth="2%" style={{ stroke: tok(numberBackgroundBorderColor), fill: tok(numberBackgroundFillColor) }} />
        </svg>
      )}
      {behindNumberSrc != null && (
        <img src={behindNumberSrc} style={{ position: "absolute", left: behindOffsetX, top: behindOffsetY, width: behindSize }} alt={String(value)} />
      )}
      <span style={{ position: "absolute", left: numberBoxOffsetX, top: numberBoxOffsetY, width: numberBoxSize, fontSize, fontFamily: "var(--font-display)",
        fontWeight: 400, color: tok(numberColor), lineHeight: 1.0, textAlign: "center", whiteSpace: "nowrap", ...border }}>{value}</span>
    </div>
  );
}
