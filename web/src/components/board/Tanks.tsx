// MapComponents/Tanks.razor: the forces and leaders in the tanks, top left, with one popover
// over all of them.
import type React from "react";
import { PhaseValue } from "../../generated/enums";
import { f, useBoard } from "./geometry";
import { MapForce, MapImage, MapLeader, MapPopoverArea } from "./primitives";
import * as pop from "./popups";

const SPACING = f(0.85);

class Bounds {
  isSet = false;
  minX = Number.MAX_VALUE;
  minY = Number.MAX_VALUE;
  maxX = -Number.MAX_VALUE;
  maxY = -Number.MAX_VALUE;

  add(x: number, y: number, w: number, h: number): void {
    this.isSet = true;
    this.minX = Math.min(this.minX, x);
    this.minY = Math.min(this.minY, y);
    this.maxX = Math.max(this.maxX, x + w);
    this.maxY = Math.max(this.maxY, y + h);
  }

  // A force stack: MapForce draws each further token up and to the left of the first.
  addStack(x: number, y: number, d: number, amount: number): void {
    const n = Math.max(amount - 1, 0);
    this.add(x - 0.07 * n * d, y - 0.08 * n * d, d + 0.07 * n * d, d + 0.08 * n * d);
  }
}

export function Tanks() {
  const b = useBoard();
  const v = b.view;
  if (PhaseValue[v.phase] < PhaseValue.MetheorAndStormSpell) return null;
  const fr = b.forceRadius;
  let y = f(f(1.75) * fr);
  let x = f(f(1.5) * fr);
  const leaderDiameter = f(f(3.2) * fr);
  const leaderPctHeight = f(f(f(f(0.25) * leaderDiameter) * b.n("BattleWheelHeroHeight")) / b.n("BattleWheelHeroWidth"));
  const forceDiameter = 2 * fr;
  const bounds = new Bounds();
  const out: React.ReactNode[] = [];
  for (const t of v.tanks) {
    const k = t.faction;
    if (t.forcesKilled > 0 || t.specialForcesKilled > 0) {
      if (t.forcesKilled > 0) {
        out.push(<MapForce key={`${k}f`} x={x} y={y} special={false} amount={t.forcesKilled} faction={t.faction} showPopover={false} />);
        bounds.addStack(x, y, forceDiameter, t.forcesKilled);
      }
      if (t.specialForcesKilled > 0) {
        const dX = t.forcesKilled > 0 ? 2 * fr : 0;
        out.push(<MapForce key={`${k}s`} x={x + dX} y={y} special amount={t.specialForcesKilled} faction={t.faction} showPopover={false} />);
        bounds.addStack(x + dX, y, forceDiameter, t.specialForcesKilled);
      }
      y = f(y + f(f(SPACING * 2) * fr));
    }
    if (t.leaders.length > 0) {
      y = f(y + f(SPACING * f(leaderPctHeight * (t.leaders.length - 1))));
      let dy = 0;
      let displaceX = Math.trunc(f(f(0.2) * leaderDiameter));
      t.leaders.forEach((l, i) => {
        const lx = x;
        const ly = y - dy;
        if (l.faceDown && l.id != null) {
          // Greyed out, as in the tanks popover, so it still reads as a leader lying face down.
          out.push(<MapImage key={`${k}l${i}`} x={lx + displaceX} y={ly} diameter={leaderDiameter} src={b.personImage(l.id)} filter="grayscale(1) brightness(0.65)"
            shadowColor={b.shadow} shadowX={2} shadowY={2} shadowBlur={1} />);
        } else if (l.faceDown) {
          out.push(<MapImage key={`${k}l${i}`} x={lx + displaceX} y={ly} diameter={leaderDiameter} src={b.factionFaceDownImage(t.faction)}
            shadowColor={b.shadow} shadowX={2} shadowY={2} shadowBlur={1} />);
        } else {
          out.push(<MapLeader key={`${k}l${i}`} src={b.personImage(l.id!)} x={lx + displaceX} y={ly} diameter={leaderDiameter} />);
        }
        bounds.add(lx + displaceX, ly, leaderDiameter, leaderDiameter);
        displaceX *= -1;
        dy += Math.trunc(leaderPctHeight);
      });
      y = f(y + f(SPACING * leaderDiameter));
    }
    if (t.messiahDead) {
      const d = f(f(0.5) * leaderDiameter);
      out.push(<MapImage key={`${k}m`} x={x} y={y} diameter={d} src={b.plainUrl("Messiah_URL")} shadowColor={b.shadow} shadowX={2} shadowY={2} shadowBlur={1} />);
      bounds.add(x, y, d, d);
      y = f(y + f(SPACING * leaderDiameter));
    }
    // NextColumnIfNecessary.
    if (y > f(15 * fr) - f(f(0.8) * x)) {
      y = f(f(1.75) * fr);
      x = f(x + f(f(4.25) * fr));
    }
  }
  if (bounds.isSet) {
    out.push(<MapPopoverArea key="area" x={bounds.minX} y={bounds.minY} width={bounds.maxX - bounds.minX} height={bounds.maxY - bounds.minY} popover={() => pop.tanks(b)} />);
  }
  return <>{out}</>;
}
