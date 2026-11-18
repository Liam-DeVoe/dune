// The generic C# components the lobby and setup screens use, for pages without a game view (the
// shared ones in components/ read the skin from the game context). Each names its razor file.
import { useEffect, useId, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { flushSync } from "react-dom";
import type { Faction } from "../../generated/enums";
import { useLobbyClient } from "../../lobby/context";
import "../Image.css";
import "./lobby.css";

/** GenericComponents/CommandButton.razor: disabled while its click runs. */
export function CommandButton({ onClick, children, className = "button--primary", small = false, enabled = true, marginClass = "command-button", paddingClass = "", title = "" }: {
  onClick: () => unknown;
  children?: ReactNode;
  className?: string;
  small?: boolean;
  enabled?: boolean;
  marginClass?: string;
  paddingClass?: string;
  title?: string;
}) {
  const [busy, setBusy] = useState(false);
  const mounted = useRef(true);
  useEffect(() => () => { mounted.current = false; }, []);
  const click = async () => {
    setBusy(true);
    try {
      await onClick();
    } finally {
      if (mounted.current) setBusy(false);
    }
  };
  const classes = ["button", className, small ? "button--small" : "", marginClass, paddingClass].join(" ");
  return <button className={classes} onClick={click} disabled={!enabled || busy} title={title}>{children}</button>;
}

/** GenericComponents/FactionComponent.razor, with the lobby's skin. */
export function FactionImage({ faction, title = "", diameter = 30, hasShadow = true, marginClass = "", onClick }: {
  faction: Faction;
  title?: string;
  diameter?: number;
  hasShadow?: boolean;
  marginClass?: string;
  onClick?: () => void;
}) {
  const { skin } = useLobbyClient();
  const t = title || skin.faction(faction);
  const style = { ...(hasShadow ? { filter: "drop-shadow(1px 1px 1px black)" } : {}), ...(onClick ? { cursor: "pointer" } : {}) };
  return <img className={`faction-image ${marginClass}`} src={skin.factionImage(faction)} width={diameter} height={diameter} alt={t} title={t} style={style} onClick={onClick} />;
}

/**
 * GenericComponents/Image.razor with `Shown` a faction: the sigil, its name as the tooltip (a
 * faction has no rich popover), and the image classes.
 */
export function FactionShown({ faction, width, noShadow = false, darkened = false, pointer = false, highlightOnHover = false, onClick }: {
  faction: Faction;
  width: number;
  noShadow?: boolean;
  darkened?: boolean;
  pointer?: boolean;
  highlightOnHover?: boolean;
  onClick?: (e: React.MouseEvent) => void;
}) {
  const { skin } = useLobbyClient();
  const name = skin.faction(faction);
  // JointClass: image, Class, shadowy, pointer (a tooltip counts), highlighted, darkened, ...
  const classes = [highlightOnHover ? "image image--hover" : "image", "", noShadow ? "" : "image--shadowy", "image--pointer", "", darkened ? "image--darkened" : "", "", ""].join(" ");
  void pointer;
  return <img alt={name} className={classes} width={String(width)} src={skin.factionImage(faction)} onClick={onClick} title={name} />;
}

/** GenericComponents/SelectFromImageComponent.razor with `Selected`: a multiple choice of factions. */
export function SelectFactions({ values, selected, onChange, imageWidth = 60, label }: {
  values: Faction[];
  selected: Faction[];
  onChange: (selected: Faction[]) => void;
  imageWidth?: number;
  label?: ReactNode;
}) {
  const click = (item: Faction, e: React.MouseEvent) => {
    // In a multi-select row, shift-click picks only the clicked value.
    if (e.shiftKey) onChange([item]);
    else if (selected.includes(item)) onChange(selected.filter((f) => f !== item));
    else onChange([...selected, item]);
  };
  return (
    <>
      {label != null && <label className="select-factions__label">{label}</label>}
      <div className="img-choices">
        {values.map((f) => (
          <FactionShown key={f} faction={f} width={imageWidth} darkened={!selected.includes(f)} pointer highlightOnHover onClick={(e) => click(f, e)} />
        ))}
      </div>
    </>
  );
}

/** GenericComponents/SelectFromImageComponent.razor with `Value`: a single faction, or none when not required. */
export function SelectFaction({ values, value, onChange, imageWidth = 60, required = true }: {
  values: Faction[];
  value: Faction | null;
  onChange: (v: Faction | null) => void;
  imageWidth?: number;
  required?: boolean;
}) {
  useEffect(() => {
    if (required && (value == null || !values.includes(value))) onChange(values[0] ?? null);
  });
  const click = (item: Faction) => {
    if (item === value) {
      if (!required) onChange(null);
    } else onChange(item);
  };
  return (
    <div className="img-choices">
      {values.map((f, i) => (
        <FactionShown key={`${f}-${i}`} faction={f} width={imageWidth} darkened={f !== value} pointer highlightOnHover onClick={() => click(f)} />
      ))}
    </div>
  );
}

function clampInto(value: number, min: number, max: number): number {
  return value < min ? min : value > max ? max : value;
}

/**
 * GenericComponents/SelectNumberComponent.razor around NumberPickerComponent: a label and a
 * typeable number with arrows. A value outside [min, max] is moved into it.
 */
export function SelectNumber({ value, onChange, min, max, stepUp = 1, stepDown = 1, label = "", disabled = false, size = 36 }: {
  value: number;
  onChange: (v: number) => void;
  min: number;
  max: number;
  stepUp?: number;
  stepDown?: number;
  label?: string;
  disabled?: boolean;
  size?: number;
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
  // NumberPickerComponent.Normalize.
  const normalize = (v: number) => {
    v = clampInto(v, min, max);
    if (stepUp > 1) {
      const steps = (v - min) / stepUp;
      v = clampInto(min + Math.sign(steps) * Math.round(Math.abs(steps)) * stepUp, min, max);
    }
    return v;
  };
  const commit = (typed: string) => {
    const t = typed.trim();
    if (!/^[+-]?\d+$/.test(t)) setText(String(value));
    else set(normalize(Number.parseInt(t, 10)));
  };
  const mayIncrease = !disabled && value < max;
  const mayDecrease = !disabled && value > min;
  return (
    <div className="select-number">
      {label !== "" && <label className="select-number__label">{label}</label>}
      <div className={`num ${size <= 30 ? "num--sm" : ""} ${disabled ? "num--off" : ""}`} title="">
        <button type="button" className="num__button" tabIndex={-1} aria-label="Decrease" disabled={!mayDecrease} onClick={() => mayDecrease && set(value - stepDown)}>−</button>
        <input type="text" className="num__input" inputMode="numeric" value={text} disabled={disabled} aria-label=""
               onChange={(e) => setText(e.target.value)}
               onBlur={(e) => commit(e.target.value)}
               onKeyDown={(e) => {
                 if (e.key === "Enter") commit((e.target as HTMLInputElement).value);
                 else if (e.key === "ArrowUp" && mayIncrease) set(value + stepUp);
                 else if (e.key === "ArrowDown" && mayDecrease) set(value - stepDown);
               }} />
        <button type="button" className="num__button" tabIndex={-1} aria-label="Increase" disabled={!mayIncrease} onClick={() => mayIncrease && set(value + stepUp)}>+</button>
      </div>
    </div>
  );
}

/** GenericComponents/ChecklistItemComponent.razor: a checkbox labelled with the value's description. */
export function ChecklistItem({ label, checked, onChange, disabled = false }: { label: string; checked: boolean; onChange: (v: boolean) => void; disabled?: boolean }) {
  const id = useId();
  return (
    <div className="check">
      <input className="check__input" style={{ cursor: "pointer" }} id={id} type="checkbox" disabled={disabled} checked={checked} onChange={(e) => onChange(e.target.checked)} />
      <label className="check__label" style={{ cursor: "pointer" }} htmlFor={id}>{label} </label>
    </div>
  );
}

/**
 * A dropdown button, as SubscribeComponent and UserStatusComponent draw them: the toggle opens a
 * menu of items below it. A click outside, Escape, or tabbing away closes it; the arrow keys move
 * through the items. Styles in components/_dropdown.scss.
 */
export function DropdownButton({ buttonClass, label, items, disabled = false, className = "" }: {
  buttonClass: string;
  label: string;
  items: { label: string; onClick: () => void }[];
  disabled?: boolean;
  className?: string;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLUListElement>(null);
  useEffect(() => {
    if (!open) return;
    const outside = (e: Event) => { if (!ref.current?.contains(e.target as Node)) setOpen(false); };
    const tabbedAway = (e: KeyboardEvent) => { if (e.key === "Tab") outside(e); };
    document.addEventListener("click", outside, true);
    document.addEventListener("keyup", tabbedAway);
    return () => {
      document.removeEventListener("click", outside, true);
      document.removeEventListener("keyup", tabbedAway);
    };
  }, [open]);
  // Below the button with a 2 px gap, at its start; at its end when the menu would leave the
  // nearest clipping ancestor (or the window).
  useLayoutEffect(() => {
    const m = menu.current;
    const b = button.current;
    if (!open || !m || !b) return;
    let right = document.documentElement.clientWidth;
    for (let e = b.parentElement; e; e = e.parentElement) {
      if (getComputedStyle(e).overflowX !== "visible") { right = e.getBoundingClientRect().right; break; }
    }
    const br = b.getBoundingClientRect();
    const mw = m.getBoundingClientRect().width;
    const x = br.left + mw > right ? br.width - mw : 0;
    m.style.transform = `translate(${Math.round(x)}px, ${Math.round(br.height + 2)}px)`;
  }, [open]);
  // Escape closes the menu and returns to the toggle. The arrow keys open it and move through its
  // items: from the toggle, down to the first and up to the last; within it, without wrapping.
  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      if (!open) return;
      e.preventDefault();
      e.stopPropagation();
      setOpen(false);
      button.current?.focus();
      return;
    }
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    e.preventDefault();
    e.stopPropagation();
    if (!open) flushSync(() => setOpen(true));
    const list = [...(menu.current?.querySelectorAll<HTMLElement>(".dropdown__item") ?? [])];
    if (list.length === 0) return;
    const down = e.key === "ArrowDown";
    const i = list.indexOf(e.target as HTMLElement);
    const next = i < 0 ? (down ? 0 : list.length - 1) : Math.max(0, Math.min(i + (down ? 1 : -1), list.length - 1));
    list[next].focus();
  };
  return (
    <div className={className ? `dropdown ${className}` : "dropdown"} ref={ref} onKeyDown={onKeyDown}>
      <button ref={button} type="button" className={`button ${buttonClass} button--small dropdown__toggle`} aria-expanded={open ? "true" : "false"}
              disabled={disabled} onClick={() => setOpen(!open)}>
        {label}
      </button>
      <ul ref={menu} className={open ? "dropdown__menu dropdown__menu--open" : "dropdown__menu"}>
        {items.map((it, i) => (
          <li key={i}><a className="dropdown__item" href="#" onClick={(e) => { e.preventDefault(); setOpen(false); it.onClick(); }}>{it.label}</a></li>
        ))}
      </ul>
    </div>
  );
}

/** GenericComponents/DateTimePicker.razor: a date and a time in the browser's time zone. */
export function DateTimePicker({ value, onChange, label = "Select Date", timeLabel = "Select Time" }: {
  value: Date;
  onChange: (d: Date) => void;
  label?: string;
  timeLabel?: string;
}) {
  const pad = (n: number) => String(n).padStart(2, "0");
  const date = `${value.getFullYear()}-${pad(value.getMonth() + 1)}-${pad(value.getDate())}`;
  const time = `${pad(value.getHours())}:${pad(value.getMinutes())}`;
  // ConstructDateTimeOffset: the date (today if cleared) at the time (midnight if cleared).
  const build = (d: string, t: string) => {
    const [y, m, day] = d ? d.split("-").map(Number) : [new Date().getFullYear(), new Date().getMonth() + 1, new Date().getDate()];
    const [h, min] = t ? t.split(":").map(Number) : [0, 0];
    return new Date(y, m - 1, day, h, min, 0, 0);
  };
  return (
    <div className="date-time-picker">
      <div className="date-time-picker__col">
        <label htmlFor="datePicker">{label}</label>
        <input type="date" id="datePicker" className="input" value={date} onChange={(e) => onChange(build(e.target.value, time))} />
      </div>
      <div className="date-time-picker__col">
        <label htmlFor="timePicker">{timeLabel}</label>
        <input type="time" id="timePicker" className="input" value={time} onChange={(e) => onChange(build(date, e.target.value))} />
      </div>
    </div>
  );
}

/** A `DateTimeOffset` as .NET writes it in JSON: local time with the offset, `2026-09-18T00:14:35.6224050-04:00`. */
export function toDateTimeOffset(d: Date): string {
  const pad = (n: number, w = 2) => String(Math.abs(n)).padStart(w, "0");
  const off = -d.getTimezoneOffset();
  const sign = off >= 0 ? "+" : "-";
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}.${pad(d.getMilliseconds(), 3)}0000${sign}${pad(Math.trunc(off / 60))}:${pad(off % 60)}`;
}

/** `.ToLocalTime().ToString("dd-MM HH:mm")`. */
export function shortDate(s: string): string {
  const d = new Date(s);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getDate())}-${pad(d.getMonth() + 1)} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/**
 * `DateTime.ToString()` (and `DateTimeOffset.ToString()` with `withOffset`) in en-US, the culture
 * the C# client runs in for an English browser: `11/10/2026 7:51:03 PM -08:00`.
 */
export function longDate(s: string | Date, withOffset = false): string {
  const d = typeof s === "string" ? new Date(s) : s;
  const pad = (n: number) => String(n).padStart(2, "0");
  const h = d.getHours() % 12 === 0 ? 12 : d.getHours() % 12;
  const base = `${d.getMonth() + 1}/${d.getDate()}/${d.getFullYear()} ${h}:${pad(d.getMinutes())}:${pad(d.getSeconds())} ${d.getHours() < 12 ? "AM" : "PM"}`;
  if (!withOffset) return base;
  const off = -d.getTimezoneOffset();
  return `${base} ${off >= 0 ? "+" : "-"}${pad(Math.trunc(Math.abs(off) / 60))}:${pad(Math.abs(off) % 60)}`;
}
