// A div whose hover shows a popover (HTML, popups.ts).
import type { CSSProperties, ReactNode } from "react";
import { IMAGE_POPOVER, usePopover } from "../popover";

export function Hoverable({ className, style, popover, children }: {
  className?: string;
  style?: CSSProperties;
  popover?: string | null;
  children?: ReactNode;
}) {
  const content = popover ? popover : null;
  const ref = usePopover<HTMLDivElement>(content, IMAGE_POPOVER);
  return content
    ? <div ref={ref} className={className} style={style}>{children}</div>
    : <div className={className} style={style}>{children}</div>;
}
