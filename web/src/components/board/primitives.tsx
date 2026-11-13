// The map's drawing primitives: MapComponents/MapImage, MapText, Circle, Rectangle,
// MapPopoverArea, MapForce and MapLeader. Each keeps the element and inline style of its razor
// file, since the look comes from the C# stylesheet.
import type { CSSProperties, MouseEvent as ReactMouseEvent, ReactNode } from "react";
import type { Faction } from "../../generated/enums";
import { f, px, useBoard } from "./geometry";
import { html, type Html, useMapPopover } from "./popover";

export interface Box {
  x: number;
  y: number;
  diameter?: number;
  width?: number;
  height?: number;
}

function size(b: Box): [number, number] {
  return b.diameter !== undefined ? [b.diameter, b.diameter] : [b.width ?? 0, b.height ?? 0];
}

// Circle.razor.
export function Circle({ x, y, radius = 1, fillColor, fillOpacity = 1, strokeColor = "transparent", strokeWidth = 0 }: {
  x: number; y: number; radius?: number; fillColor?: string; fillOpacity?: number; strokeColor?: string; strokeWidth?: number;
}) {
  return (
    <svg width={2 * radius} height={2 * radius} style={{ position: "absolute", left: px(x - radius), top: px(y - radius) }} pointerEvents="none">
      <circle cx={radius} cy={radius} r={radius - strokeWidth} fill={fillColor} fillOpacity={fillOpacity} stroke={strokeColor} strokeWidth={px(strokeWidth)} />
    </svg>
  );
}

// Rectangle.razor.
export function Rectangle({ x, y, width, height, fillColor, fillOpacity = 1, strokeColor = "transparent", strokeWidth = 0 }: {
  x: number; y: number; width: number; height: number; fillColor?: string; fillOpacity?: number; strokeColor?: string; strokeWidth?: number;
}) {
  return (
    <svg width={width} height={height} style={{ position: "absolute", left: px(x), top: px(y) }} pointerEvents="none">
      <rect width={width} height={height} fill={fillColor} fillOpacity={fillOpacity} stroke={strokeColor} strokeWidth={px(strokeWidth)} />
    </svg>
  );
}

export interface MapImageProps extends Box {
  src: string;
  opacity?: number;
  shadowX?: number;
  shadowY?: number;
  shadowBlur?: number;
  shadowColor?: string;
  filter?: string;
  animation?: string;
  backgroundCircleColor?: string;
  backgroundCircleDiameter?: number;
  backgroundRectangleColor?: string;
  onClick?: (e: ReactMouseEvent) => void;
  // HTML for a hover popover, or with popoverAsTooltip the text of a title.
  popover?: Html;
  popoverAsTooltip?: boolean;
  popoverStyle?: string;
}

// MapImage.razor.
export function MapImage(p: MapImageProps) {
  const [w, h] = size(p);
  const { x, y } = p;
  const shadowX = p.shadowX ?? 0;
  const shadowY = p.shadowY ?? 0;
  const shadowFilter = shadowX > 0 || shadowY > 0 ? `drop-shadow(${px(shadowX)} ${px(shadowY)} ${px(p.shadowBlur ?? 0)} ${p.shadowColor ?? "black"})` : "";
  const filter = p.filter ?? "";
  const interactive = p.onClick !== undefined || html(p.popover) != null;
  const style: CSSProperties = {
    position: "absolute", left: px(x), top: px(y), width: px(w), height: px(h), opacity: p.opacity ?? 1,
    ...(interactive ? { cursor: "pointer", pointerEvents: "all" } : { pointerEvents: "none" }),
    ...(shadowFilter.length > 0 || filter.length > 0 ? { filter: `${shadowFilter} ${filter.replace(/;$/, "")}` } : {}),
    ...(p.animation ? { animation: p.animation } : {}),
  };
  const content = html(p.popover);
  const tooltip = content != null && p.popoverAsTooltip ? content : undefined;
  const { ref, attributes } = useMapPopover<HTMLImageElement>(p.popoverAsTooltip ? null : content, { style: p.popoverStyle });
  return (
    <>
      {p.backgroundCircleColor != null && (
        <Circle x={x + f(0.5 * w)} y={y + f(0.5 * h)} radius={(p.backgroundCircleDiameter ?? 0) > 0 ? f(0.5 * p.backgroundCircleDiameter!) : f(0.5 * w)} fillColor={p.backgroundCircleColor} />
      )}
      {p.backgroundRectangleColor != null && <Rectangle x={x} y={y} width={w} height={h} fillColor={p.backgroundRectangleColor} />}
      <img ref={ref} src={p.src} style={style} title={tooltip} alt={tooltip ?? ""} onClick={p.onClick} {...attributes} />
    </>
  );
}

export type Align = "left" | "center" | "right";

// Support.TextBorder.
export function textBorder(width: number, color: string): CSSProperties {
  return { WebkitTextStroke: `${Number((0.3 * width).toFixed(3))}px ${color}`, textShadow: `0 0 ${2 * width}px ${color}` };
}

export interface MapTextProps {
  x: number;
  y: number;
  toShow?: string;
  children?: ReactNode;
  font?: string;
  fontColor?: string;
  borderWidth?: number;
  borderColor?: string;
  align?: Align;
  maxWidth?: number;
  popover?: Html;
  popoverAsTooltip?: boolean;
  popoverStyle?: string;
}

// MapText.razor.
export function MapText(p: MapTextProps) {
  const align = p.align ?? "left";
  const transformX = align === "center" ? "translateX(-50%)" : align === "right" ? "translateX(-100%)" : "";
  const maxWidth = p.maxWidth ?? 0;
  const content = html(p.popover);
  const hasPopover = content != null && !p.popoverAsTooltip;
  const { ref, attributes } = useMapPopover<HTMLSpanElement>(hasPopover ? content : null, { style: p.popoverStyle, delay: 50 });
  const style: CSSProperties = {
    position: "absolute", textAlign: align, transform: `${transformX} translateY(-40%)`, left: px(p.x), top: px(p.y),
    userSelect: "none", font: p.font, color: p.fontColor ?? "white",
    ...(maxWidth > 0 ? { maxWidth: px(maxWidth) } : { whiteSpace: "nowrap" }),
    ...textBorder(p.borderWidth ?? 0, p.borderColor ?? "black"),
    ...(content != null ? { cursor: "pointer", pointerEvents: "all" } : { pointerEvents: "none" }),
  };
  if (hasPopover) {
    return <span ref={ref} style={style} {...attributes}>{p.toShow}{p.children}</span>;
  }
  return <span title={content ?? undefined} style={style}>{p.toShow}{p.children}</span>;
}

// MapPopoverArea.razor: an invisible rectangle with one popover for everything under it.
export function MapPopoverArea({ x, y, width, height, popover, placement = "right" }: {
  x: number; y: number; width: number; height: number; popover: Html; placement?: "right" | "left" | "top" | "bottom";
}) {
  const { ref, attributes } = useMapPopover<HTMLDivElement>(html(popover), { placement });
  return <div ref={ref} {...attributes} style={{ position: "absolute", left: px(x), top: px(y), width: px(width), height: px(height), pointerEvents: "all" }} />;
}

// Support.HasStarredSpecialForce.
export function hasStarredSpecialForce(faction: Faction): boolean {
  return faction === "Fremen" || faction === "Emperor" || faction === "Ixian";
}

// Support.ForceTokenHtml.
export function forceTokenHtml(src: string, faction: Faction, special: boolean, sizePx: number, title = ""): string {
  const starred = special && hasStarredSpecialForce(faction) ? " starred" : "";
  return `<span class='force-token${starred}' style='--d:${sizePx}px' title='${title}'><img src='${src}' width='${sizePx}' height='${sizePx}' alt='${title}'/></span>`;
}

// MapForce.razor: a stack of force tokens with its count.
export function MapForce({ x, y, special, amount, faction, threatened = false, showPopover = true, font = "", diameter = 0 }: {
  x: number; y: number; special: boolean; amount: number; faction: Faction; threatened?: boolean; showPopover?: boolean; font?: string; diameter?: number;
}) {
  const b = useBoard();
  const d = diameter === 0 ? 2 * b.forceRadius : diameter;
  const fnt = font === "" ? b.s("FORCETOKEN_FONT") : font;
  const src = b.forceImage(faction, special);
  const starred = special && hasStarredSpecialForce(faction);
  const shadowColor = threatened ? "red" : b.shadow;
  const shadowSize = threatened ? 1 : 0.5;
  const shadowBlur = threatened ? 1 : 0.5;
  const borderColor = threatened ? "red" : b.s("FORCETOKEN_FONT_BORDERCOLOR");
  const borderWidth = threatened ? 2 * b.n("FORCETOKEN_FONT_BORDERWIDTH") : b.n("FORCETOKEN_FONT_BORDERWIDTH");
  const { ref, attributes } = useMapPopover<HTMLDivElement>(showPopover
    ? `<div style='position:relative'>${forceTokenHtml(src, faction, special, 80)}<span style='color:white;font-size:32px;font-weight:900;position:absolute;left:55px;top:-10px;-webkit-text-stroke:1px black'>${amount}</span></div>`
    : null);
  const tokens = [];
  for (let i = 0; i < amount; i++) {
    const position: CSSProperties = {
      ...(showPopover ? {} : { pointerEvents: "none" }),
      position: "absolute", left: px(-0.07 * i * d), top: px(-0.08 * i * d), opacity: 1,
    };
    const look: CSSProperties = { width: px(d), filter: `drop-shadow(${px(shadowSize)} ${px(shadowSize)} ${px(shadowBlur)} ${shadowColor})` };
    if (starred && i === amount - 1) {
      tokens.push(
        <span key={i} className="force-token starred" style={{ ...position, ["--d" as string]: px(d) }}>
          <img src={src} style={look} alt="Token" />
        </span>,
      );
    } else {
      tokens.push(<img key={i} src={src} style={{ ...position, ...look }} alt="Token" />);
    }
  }
  return (
    <>
      <div ref={ref} {...attributes} style={{ position: "absolute", left: px(x), top: px(y) }}>{tokens}</div>
      {!((faction === "Richese" && special) || amount === 0) && (
        <MapText toShow={String(amount)} x={x + f(0.5 * d)} y={y + f(0.5 * d)} font={fnt} borderWidth={borderWidth} borderColor={borderColor} />
      )}
    </>
  );
}

// MapLeader.razor.
export function MapLeader({ src, x, y, diameter, popover }: { src: string; x: number; y: number; diameter: number; popover?: Html }) {
  const b = useBoard();
  return <MapImage x={x} y={y} diameter={diameter} src={src} shadowColor={b.shadow} shadowX={2} shadowY={2} shadowBlur={1} popover={popover} popoverStyle="" />;
}
