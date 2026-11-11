// GenericComponents/SimpleNumberComponent.razor: a number on a small disc.
import type { ReactNode } from "react";
import { supportRound } from "../text";
import "./SimpleNumber.css";

export function SimpleNumber({ size = 20, children }: { size?: number; children: ReactNode }) {
  // Size is a float parameter in C#; the radius is 0.5f * Size.
  const s = supportRound(Math.fround(size));
  const radius = supportRound(Math.fround(0.5 * Math.fround(size)));
  return (
    <svg data-simple-number="" width={s} height={s}>
      <circle cx={radius} cy={radius} r={radius} fill="white" />
      <text x={radius} y={radius} textAnchor="middle" dominantBaseline="middle" stroke="black">{children}</text>
    </svg>
  );
}
