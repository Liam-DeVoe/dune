// GenericComponents/SelectComponent.razor with wwwroot/js/listbox.js: a button that looks like
// Bootstrap's .form-select and opens a list the app draws, in the top layer (a popover) under the
// trigger, or above it when there is more room there. An outside click, a scroll or a resize
// closes it. Keyboard as in C#: arrows, Page Up/Down, Home and End move the highlight (and open
// a closed list), Enter, Space or Tab pick the highlighted option, Escape closes, and typing
// jumps to the first label starting with the typed text (repeating one letter cycles through the
// labels starting with it). Closed, Enter and Space click the button.
//
// The action forms of packages 7b and 7e share this one. Their server normalizes the draft, so a
// value missing from the list is left alone unless `required` asks for C#'s fallback to the first.
import { useEffect, useId, useLayoutEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";

export interface SelectOption<T> {
  value: T;
  label: string;
  // A heading is drawn where the group changes (GroupLabels).
  group?: string | null;
}

const GAP = 4; // px between trigger and list
const MARGIN = 8; // px kept clear of the viewport edge
const TYPE_AHEAD_RESET_MS = 600;

export function Select<T>({ options, value, onChange, label, className = "mt-1", small = false, disabled = false, required = false }: {
  options: SelectOption<T>[];
  value: T | null;
  onChange: (v: T) => void;
  label?: ReactNode;
  className?: string;
  small?: boolean;
  disabled?: boolean;
  required?: boolean;
}) {
  const id = useId();
  const trigger = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLUListElement>(null);
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(-1);
  const typed = useRef({ text: "", at: 0 });

  useEffect(() => {
    if (required && options.length > 0 && !options.some((o) => o.value === value)) onChange(options[0].value);
  });

  // OnParametersSetAsync: a list that becomes empty or disabled closes; the highlight stays in range.
  useEffect(() => {
    if (open && (disabled || options.length === 0)) setOpen(false);
    else if (open && active > options.length - 1) setActive(options.length - 1);
  }, [open, disabled, options.length, active]);

  const selectedIndex = options.findIndex((o) => o.value === value);
  const optionId = (i: number) => `${id}-option-${i}`;

  useLayoutEffect(() => {
    const l = list.current;
    const t = trigger.current;
    if (!l || !t) return;
    if (!open) {
      if (typeof l.hidePopover === "function" && l.matches(":popover-open")) l.hidePopover();
      return;
    }
    if (typeof l.showPopover === "function" && !l.matches(":popover-open")) l.showPopover();
    // listbox.js place().
    const r = t.getBoundingClientRect();
    const maxFromCss = parseFloat(getComputedStyle(l).getPropertyValue("--listbox-max-height")) || 320;
    const below = window.innerHeight - r.bottom - GAP - MARGIN;
    const above = r.top - GAP - MARGIN;
    const wanted = Math.min(l.scrollHeight, maxFromCss);
    const openAbove = below < wanted && above > below;
    const height = Math.min(wanted, openAbove ? above : below);
    l.style.minWidth = `${r.width}px`;
    l.style.maxHeight = `${Math.max(height, 40)}px`;
    // Set the width first so the horizontal clamp sees the real size.
    const left = Math.min(r.left, window.innerWidth - MARGIN - l.offsetWidth);
    l.style.left = `${Math.max(MARGIN, left)}px`;
    if (openAbove) {
      l.style.top = "auto";
      l.style.bottom = `${window.innerHeight - r.top + GAP}px`;
    } else {
      l.style.bottom = "auto";
      l.style.top = `${r.bottom + GAP}px`;
    }
    const close = () => setOpen(false);
    const onPointerDown = (e: PointerEvent) => {
      if (t.contains(e.target as Node) || l.contains(e.target as Node)) return;
      close();
    };
    const onScroll = (e: Event) => { if (!l.contains(e.target as Node)) close(); };
    document.addEventListener("pointerdown", onPointerDown, true);
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", close);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", close);
    };
  }, [open]);

  // listbox.js reveal(): keep the highlighted option in view.
  useEffect(() => {
    if (open && active >= 0) document.getElementById(optionId(active))?.scrollIntoView({ block: "nearest" });
  });

  const openList = (): number => {
    if (disabled || options.length === 0) return -1;
    const start = Math.max(selectedIndex, 0);
    setOpen(true);
    setActive(start);
    typed.current.text = "";
    return start;
  };
  const choose = (i: number) => {
    if (options[i] && options[i].value !== value) onChange(options[i].value);
    setOpen(false);
  };
  const typeAhead = (key: string, from: number): number => {
    const now = Date.now();
    if (now - typed.current.at > TYPE_AHEAD_RESET_MS) typed.current.text = "";
    typed.current.at = now;
    typed.current.text += key;
    const t = typed.current.text;
    const cycling = t.length > 1 && [...t].every((c) => c === t[0]);
    const prefix = (cycling ? t[0] : t).toLowerCase();
    const start = cycling ? from + 1 : 0;
    for (let step = 0; step < options.length; step++) {
      const i = (start + step) % options.length;
      if (options[i].label.toLowerCase().startsWith(prefix)) return i;
    }
    return from;
  };
  const onKeyDown = (e: KeyboardEvent) => {
    const navigation = ["ArrowDown", "ArrowUp", "Home", "End", "PageDown", "PageUp"];
    const printable = e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey;
    const handled = navigation.includes(e.key) || printable || (open && ["Enter", " ", "Escape", "Tab"].includes(e.key));
    // Closed, Enter and Space fall through to the button's click.
    if (!handled) return;
    // Tab keeps its default so focus still moves on.
    if (e.key !== "Tab") e.preventDefault();
    if (!open) {
      const start = openList();
      if (start >= 0 && e.key.length === 1) setActive(typeAhead(e.key, start));
      return;
    }
    const last = options.length - 1;
    const move = (d: number) => setActive((a) => Math.min(Math.max(a + d, 0), last));
    switch (e.key) {
      case "ArrowDown": move(1); break;
      case "ArrowUp": move(-1); break;
      case "PageDown": move(10); break;
      case "PageUp": move(-10); break;
      case "Home": setActive(0); break;
      case "End": setActive(last); break;
      case "Enter": case " ": case "Tab": choose(active); break;
      case "Escape": setOpen(false); break;
      default: if (e.key.length === 1) setActive(typeAhead(e.key, active));
    }
  };

  return (
    <div className={`select ${className}`}>
      {label != null && <label className="mb-0" id={`${id}-label`} htmlFor={id}>{label}</label>}
      <button type="button" id={id} ref={trigger} className={`select-trigger form-select ${small ? "form-select-sm" : ""}`}
              role="combobox" aria-haspopup="listbox" aria-expanded={open ? "true" : "false"} aria-controls={`${id}-list`}
              aria-activedescendant={open && active >= 0 ? optionId(active) : undefined}
              disabled={disabled} onClick={() => (open ? setOpen(false) : openList())} onKeyDown={onKeyDown}>
        {options[selectedIndex]?.label ?? ""}
      </button>
      <ul className="select-list" id={`${id}-list`} role="listbox" ref={list} {...{ popover: "manual" }}>
        {options.map((o, i) => (
          <SelectRow key={i} option={o} heading={o.group != null && (i === 0 || options[i - 1].group !== o.group)}
                     selected={i === selectedIndex} active={open && i === active} id={optionId(i)}
                     onPointerMove={() => setActive(i)} onClick={() => choose(i)} />
        ))}
      </ul>
    </div>
  );
}

function SelectRow<T>({ option, heading, selected, active, id, onPointerMove, onClick }: {
  option: SelectOption<T>; heading: boolean; selected: boolean; active: boolean; id: string;
  onPointerMove: () => void; onClick: () => void;
}) {
  return (
    <>
      {heading && <li className="select-group" role="presentation">{option.group}</li>}
      <li id={id} role="option" aria-selected={selected ? "true" : "false"}
          className={`select-option ${selected ? "is-selected" : ""} ${active ? "is-active" : ""}`}
          onPointerMove={onPointerMove} onClick={onClick}>
        {option.label}
      </li>
    </>
  );
}
