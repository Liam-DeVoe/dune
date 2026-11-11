// MapComponents/RecentMoves.razor and Arrow.razor: during ship and move, an arc of small arrows
// for each recent move, along the sphere of the planet. The view lists the segments
// (board.recentMoves, already split at the Polar Sink as RecentMoves.razor does).
import { useId } from "react";
import type { Faction } from "../../generated/enums";
import { f, num, useBoard, type Point } from "./geometry";

export function RecentMoves() {
  const b = useBoard();
  const v = b.view;
  if (!b.d.ShowArrowsForRecentMoves || v.mainPhase !== "ShipmentAndMove") return null;
  const size = b.planetRadius * 2;
  return (
    <svg version="1.1" xmlns="http://www.w3.org/2000/svg" xmlnsXlink="http://www.w3.org/1999/xlink" width={size} height={size}
      style={{ position: "absolute", left: "0px", top: "0px", height: "100%", width: "100%" }} pointerEvents="none">
      {v.board.recentMoves.map((m, i) => (
        <Arrow key={i} faction={m.faction} from={b.getCenter(m.from)} to={b.getCenter(m.to)} radius={f(f(1.2) * b.planetRadius)} />
      ))}
    </svg>
  );
}

const cross = (a: number[], c: number[]) => [a[1] * c[2] - a[2] * c[1], a[2] * c[0] - a[0] * c[2], a[0] * c[1] - a[1] * c[0]];
const inner = (a: number[], c: number[]) => a.reduce((s, x, i) => s + x * c[i], 0);
const length = (a: number[]) => Math.sqrt(a.reduce((s, x) => s + x * x, 0));

// Arrow.razor, with MarginX = MarginY = 0, ArrowScale 0.05, shadow distance 4 and blur 1.
function Arrow({ faction, from, to, radius }: { faction: Faction; from: Point; to: Point; radius: number }) {
  const b = useBoard();
  const id = useId().replace(/:/g, "");
  const fillColor = b.factionColor(faction);
  const strokeColor = b.factionColor(faction) + "bb";
  // (int) of a double saturates in .NET 9 and later: NaN becomes 0.
  const toInt = (x: number) => (Number.isNaN(x) ? 0 : Math.trunc(x));
  const fromA = [from.X - radius, from.Y - radius, 0];
  fromA[2] = toInt(Math.pow(radius * radius - fromA[0] * fromA[0] - fromA[1] * fromA[1], 0.5));
  const toA = [to.X - radius, to.Y - radius, 0];
  toA[2] = toInt(Math.pow(radius * radius - toA[0] * toA[0] - toA[1] * toA[1], 0.5));
  const u = fromA.map((a) => a / radius);
  const w = cross(fromA, toA);
  const v = cross(u, w).map((a) => a / length(w));
  const tMax = Math.acos(inner(fromA, toA) / length(fromA) / length(toA));
  // Math.Round rounds half to even.
  const raw = 50 * tMax / Math.PI;
  let n = Math.round(raw);
  if (Math.abs(raw % 1) === 0.5 && n % 2 !== 0) n -= 1;
  const arrows = [];
  for (let i = 1; i <= n; i++) {
    const t = -tMax * (i / n);
    const x = radius * (u[0] * Math.cos(t) + v[0] * Math.sin(t));
    const y = radius * (u[1] * Math.cos(t) + v[1] * Math.sin(t));
    const t2 = -tMax * ((i + 1) / n);
    const x2 = radius * (u[0] * Math.cos(t2) + v[0] * Math.sin(t2));
    const y2 = radius * (u[1] * Math.cos(t2) + v[1] * Math.sin(t2));
    const alpha = -Math.atan2(x2 - x, y2 - y) / Math.PI * 180 + 90;
    arrows.push({ tx: num(x + radius), ty: num(y + radius), rotate: num(alpha) });
  }
  return (
    <>
      <defs>
        <linearGradient id={`arrowgradient${id}`} x1="60%" y1="60%" x2="80%" y2="80%">
          <stop offset="0%" stopColor={fillColor} />
          <stop offset="100%" stopColor="black" />
        </linearGradient>
        <filter id={`arrowshadow${id}`} height="200%" width="200%">
          <feGaussianBlur in="SourceAlpha" stdDeviation={1} />
          <feOffset dx={4} dy={4} result="offsetblur" />
          <feComponentTransfer>
            <feFuncA type="linear" slope="0.5" />
          </feComponentTransfer>
          <feMerge>
            <feMergeNode />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
        <path id={`arrowtemplate${id}`} d="M-50,-50L0,0L-50,50L0,100L100,0L0,-100L-50,-50Z" stroke={strokeColor} strokeWidth={0} strokeLinejoin="round" transform={`scale(${num(f(0.05))})`} />
      </defs>
      {arrows.map((a, i) => (
        <g key={i} transform={`translate(${a.tx},${a.ty}) rotate(${a.rotate})`} fill={`url(#arrowgradient${id})`} filter={`url(#arrowshadow${id})`}>
          <use href={`#arrowtemplate${id}`} />
        </g>
      ))}
    </>
  );
}
