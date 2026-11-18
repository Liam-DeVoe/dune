// The images rich text draws: a leader disc, a force token,
// an ambassador. A rich popover (`popover`) or a tooltip (`tooltip`). The state classes
// (highlighted, darkened, blurred, greyed) are the faction panel's; their styles are Image.css.
import type { CSSProperties } from "react";
import { IMAGE_POPOVER, usePopover } from "../popover";
import { supportRound } from "../text";
import "./Image.css";

export function Image({ src, alt, width, className = "", noShadow = false, popover = null, tooltip = null, starred = false,
  highlighted = false, darkened = false, blurred = false, greyedOut = false, pointer = false, highlightOnHover = false, style, onClick }: {
  src: string;
  alt: string;
  width: number;
  className?: string;
  noShadow?: boolean;
  popover?: string | null;
  tooltip?: string | null;
  starred?: boolean;
  highlighted?: boolean;
  darkened?: boolean;
  blurred?: boolean;
  greyedOut?: boolean;
  // Pointer: the pointer cursor without a popover. HighlightOnHover: image--hover on "image".
  pointer?: boolean;
  highlightOnHover?: boolean;
  // Inline style on the img. A filter here replaces the state classes' filters.
  style?: CSSProperties;
  onClick?: () => void;
}) {
  const ref = usePopover<HTMLImageElement>(popover, IMAGE_POPOVER);
  const w = width > 0 ? supportRound(width) : "";
  // JointClass: image, Class, shadowy, pointer, highlighted, darkened, blurred, greyed, as image--* modifiers.
  const classes = [highlightOnHover ? "image image--hover" : "image", className, noShadow ? "" : "image--shadowy", popover !== null || tooltip !== null || pointer ? "image--pointer" : "",
    highlighted ? "image--highlighted" : "", darkened ? "image--darkened" : "", blurred ? "image--blurred" : "", greyedOut ? "image--greyed" : "", starred && width > 0 ? "force-token__img" : ""]
    .filter((c) => c).join(" ");
  const img = popover !== null
    ? <img ref={ref} alt={alt} className={classes} width={w} src={src} style={style} onClick={onClick} />
    : <img alt={alt} className={classes} width={w} src={src} style={style} onClick={onClick} title={tooltip ?? undefined} />;
  return starred && width > 0 ? <span className="force-token force-token--starred" style={{ ["--d" as string]: `${w}px` }}>{img}</span> : img;
}
