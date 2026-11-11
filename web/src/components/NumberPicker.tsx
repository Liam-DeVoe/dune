// GenericComponents/NumberPickerComponent.razor, with its two spice wrappers
// SelectResourcesComponent and SelectResourcesFromFactionComponent. A value outside [min, max]
// is moved into it whenever the range changes (OnParametersSetAsync), so a form's draft follows
// its options.
import { useEffect, useState } from "react";
import type { Faction } from "../generated/enums";
import { useGame } from "../state/context";

function clampInto(value: number, min: number, max: number): number {
  // Min wins when max < min, as the C# checks Min first.
  return value < min ? min : value > max ? max : value;
}

export function NumberPicker({ value, onChange, onStep, min, max, symbolSrc, symbolStarred = false, symbolFill, spice = false, disabled = false, title = "" }: {
  value: number;
  onChange: (v: number) => void;
  // OnUp/OnDown without a bound Value, as ForceSelectionComponent uses the picker: the buttons
  // and arrow keys step the owner's value, and a typed number only changes what the picker
  // shows (its own Value) until the owner's value changes.
  onStep?: (delta: number) => void;
  min: number;
  max: number;
  symbolSrc?: string;
  // SymbolStarred: a starred special force, drawn with the star badge.
  symbolStarred?: boolean;
  // SymbolBackgroundFillColor: a force symbol's faction colour (IconStyle "--fc").
  symbolFill?: string;
  // SymbolBackgroundBorderColor "#FF5400": the spice style of the symbol.
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

  // The picker's own Value: the owner's, or with onStep the last number typed.
  const [own, setOwn] = useState(value);
  useEffect(() => setOwn(clampInto(value, min, max)), [value, min, max]);
  const set = (v: number) => {
    if (onStep) {
      if (v !== own) onStep(v > own ? 1 : -1);
      setOwn(v);
    } else if (v !== value) onChange(v);
    setText(String(v));
  };
  const commit = (typed: string) => {
    const n = Number.parseInt(typed.trim(), 10);
    if (Number.isNaN(n) || !/^[+-]?\d+$/.test(typed.trim())) setText(String(onStep ? own : value));
    else if (onStep) {
      const v = clampInto(n, min, max);
      setOwn(v);
      setText(String(v));
    } else set(clampInto(n, min, max));
  };
  const current = onStep ? own : value;
  const mayIncrease = !disabled && current < max;
  const mayDecrease = !disabled && current > min;
  return (
    <div className={`num ${disabled ? "off" : ""}`} title={title}>
      {symbolSrc !== undefined && (
        <span className={`ic ${spice ? "spice" : "force"}`}
              style={!spice && symbolFill && symbolFill.toLowerCase() !== "white" ? { ["--fc" as string]: symbolFill } : undefined}>
          {symbolStarred
            ? <span className="force-token starred" style={{ ["--d" as string]: "18px" }}><img src={symbolSrc} alt="" /></span>
            : <img src={symbolSrc} alt="" />}
        </span>
      )}
      <button type="button" tabIndex={-1} aria-label="Decrease" disabled={!mayDecrease} onClick={() => mayDecrease && set(current - 1)}>−</button>
      <input type="text" inputMode="numeric" value={text} disabled={disabled} aria-label={title}
             onChange={(e) => setText(e.target.value)}
             onBlur={(e) => commit(e.target.value)}
             onKeyDown={(e) => {
               if (e.key === "Enter") commit((e.target as HTMLInputElement).value);
               else if (e.key === "ArrowUp" && mayIncrease) set(current + 1);
               else if (e.key === "ArrowDown" && mayDecrease) set(current - 1);
             }} />
      <button type="button" tabIndex={-1} aria-label="Increase" disabled={!mayIncrease} onClick={() => mayIncrease && set(current + 1)}>+</button>
    </div>
  );
}

// GenericComponents/SelectResourcesComponent.razor: a spice amount.
export function SelectResources({ value, onChange, min, max, label = "" }: {
  value: number; onChange: (v: number) => void; min: number; max: number; label?: string;
}) {
  const { skin } = useGame();
  return (
    <div className="d-inline-flex align-items-center">
      {label !== "" && <label className="me-1">{label}</label>}
      <NumberPicker value={value} onChange={onChange} min={min} max={max} symbolSrc={skin.url(skin.str("Harvester_URL"))} spice />
    </div>
  );
}

// GenericComponents/SelectResourcesFromFactionComponent.razor: spice paid by a faction, with its
// sigil as the symbol.
export function SelectResourcesFromFaction({ value, onChange, min, max, faction }: {
  value: number; onChange: (v: number) => void; min: number; max: number; faction: Faction;
}) {
  const { skin } = useGame();
  return (
    <div className="d-inline-flex align-items-center">
      <NumberPicker value={value} onChange={onChange} min={min} max={max} symbolSrc={skin.factionImage(faction)} spice />
    </div>
  );
}
