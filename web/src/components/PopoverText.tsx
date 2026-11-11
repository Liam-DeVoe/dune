// GenericComponents/RichText.razor: a name (or other content) in a span whose hover shows a
// popover: a card, a leader, a token. `popover` is the popover's HTML (popups.ts).
import type { CSSProperties, ReactNode } from "react";
import { popoverAttributes, usePopover } from "../popover";

export function PopoverText({ popover, className = "fw-bold", style, children }: {
  popover: string;
  className?: string;
  style?: CSSProperties;
  children: ReactNode;
}) {
  const ref = usePopover<HTMLSpanElement>(popover);
  return (
    <span ref={ref} {...popoverAttributes(popover, "text")} className={className} style={{ cursor: "pointer", ...style }}>
      {children}
    </span>
  );
}
