// A number picker, with its two spice wrappers SelectResources and SelectResourcesFromFaction. A
// value outside [min, max] is moved into it whenever the range changes, so a form's draft follows
// its options.
import { useEffect, useState } from "react";
import type { Faction } from "../generated/enums";
import { useGame } from "../state/context";

function clampInto(value: number, min: number, max: number): number {
  // Min wins when max < min.
  return value < min ? min : value > max ? max : value;
}

export function NumberPicker({ value, onChange, min, max, symbolSrc, symbolStarred = false, symbolFill, spice = false, disabled = false, title = "" }: {
  value: number;
  onChange: (v: number) => void;
  min: number;
  max: number;
  symbolSrc?: string;
  // A starred special force, drawn with the star badge.
  symbolStarred?: boolean;
  // A force symbol's faction colour (IconStyle "--fc").
  symbolFill?: string;
  // Border colour "#FF5400": the spice style of the symbol.
  spice?: boolean;
  disabled?: boolean;
  title?: string;
}) {
  const [text, setText] = useState(String(value));
  useEffect(() => {
    const v = clampInto(value, min, max);
    if (v !== value) onChange(v);
    setText(String(v));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [value, min, max]);

  const set = (v: number) => {
    if (v !== value) onChange(v);
    setText(String(v));
  };
  // A typed number goes to the owner like a step, clamped into [min, max].
  const commit = (typed: string) => {
    const n = Number.parseInt(typed.trim(), 10);
    if (Number.isNaN(n) || !/^[+-]?\d+$/.test(typed.trim())) setText(String(value));
    else set(clampInto(n, min, max));
  };
  const mayIncrease = !disabled && value < max;
  const mayDecrease = !disabled && value > min;
  return (
    <div className={`num ${disabled ? "num--off" : ""}`} title={title}>
      {symbolSrc !== undefined && (
        <span className={`num__ic ${spice ? "" : "num__ic--force"}`}
              style={!spice && symbolFill && symbolFill.toLowerCase() !== "white" ? { ["--fc" as string]: symbolFill } : undefined}>
          {symbolStarred
            ? <span className="force-token force-token--starred" style={{ ["--d" as string]: "18px" }}><img className="force-token__img num__ic-img" src={symbolSrc} alt="" /></span>
            : <img className="num__ic-img" src={symbolSrc} alt="" />}
        </span>
      )}
      <button type="button" className="num__button" tabIndex={-1} aria-label="Decrease" disabled={!mayDecrease} onClick={() => mayDecrease && set(value - 1)}>−</button>
      <input type="text" className="num__input" inputMode="numeric" value={text} disabled={disabled} aria-label={title}
             onChange={(e) => setText(e.target.value)}
             onBlur={(e) => commit(e.target.value)}
             onKeyDown={(e) => {
               if (e.key === "Enter") commit((e.target as HTMLInputElement).value);
               else if (e.key === "ArrowUp" && mayIncrease) set(value + 1);
               else if (e.key === "ArrowDown" && mayDecrease) set(value - 1);
             }} />
      <button type="button" className="num__button" tabIndex={-1} aria-label="Increase" disabled={!mayIncrease} onClick={() => mayIncrease && set(value + 1)}>+</button>
    </div>
  );
}

// A spice amount.
export function SelectResources({ value, onChange, min, max, label = "" }: {
  value: number; onChange: (v: number) => void; min: number; max: number; label?: string;
}) {
  const { skin } = useGame();
  return (
    <div className="select-resources">
      {label !== "" && <label className="select-resources__label">{label}</label>}
      <NumberPicker value={value} onChange={onChange} min={min} max={max} symbolSrc={skin.url(skin.str("Harvester_URL"))} spice />
    </div>
  );
}

// Spice paid by a faction, with its
// sigil as the symbol.
export function SelectResourcesFromFaction({ value, onChange, min, max, faction }: {
  value: number; onChange: (v: number) => void; min: number; max: number; faction: Faction;
}) {
  const { skin } = useGame();
  return (
    <div className="select-resources">
      <NumberPicker value={value} onChange={onChange} min={min} max={max} symbolSrc={skin.factionImage(faction)} spice />
    </div>
  );
}
