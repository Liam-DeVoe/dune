// GenericComponents/Image.razor, for the images rich text draws: a leader disc, a force token,
// an ambassador. A rich popover (`popover`) or a tooltip (`tooltip`). The state classes
// (highlighted, darkened, blurred, greyed) are the faction panel's; their styles are Image.css.
import { popoverAttributes, usePopover } from "../popover";
import { supportRound } from "../text";
import "./Image.css";

export function Image({ src, alt, width, className = "", noShadow = false, popover = null, tooltip = null, starred = false,
  highlighted = false, darkened = false, blurred = false, greyedOut = false, pointer = false, highlightOnHover = false, onClick }: {
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
  // Pointer: the pointer cursor without a popover. HighlightOnHover: "image-hover" for "image".
  pointer?: boolean;
  highlightOnHover?: boolean;
  onClick?: () => void;
}) {
  const ref = usePopover<HTMLImageElement>(popover);
  const w = width > 0 ? supportRound(width) : "";
  // JointClass: image, Class, shadowy, pointer, highlighted, darkened, blurred, greyed.
  const classes = [highlightOnHover ? "image-hover" : "image", className, noShadow ? "" : "shadowy", popover !== null || tooltip !== null || pointer ? "pointer" : "",
    highlighted ? "highlighted" : "", darkened ? "darkened" : "", blurred ? "blurred" : "", greyedOut ? "greyed" : ""]
    .filter((c) => c).join(" ");
  const img = popover !== null
    ? <img ref={ref} alt={alt} className={classes} width={w} src={src} onClick={onClick} {...popoverAttributes(popover, "image")} />
    : <img alt={alt} className={classes} width={w} src={src} onClick={onClick} title={tooltip ?? undefined} />;
  return starred && width > 0 ? <span className="force-token starred" style={{ ["--d" as string]: `${w}px` }}>{img}</span> : img;
}
