// GenericComponents/ForceToken.razor: a faction's force or special force token. Starred special
// forces (Support.HasStarredSpecialForce) get a star badge on the edge.
import type { CSSProperties } from "react";
import type { Faction } from "../generated/enums";
import { useGame } from "../state/context";

export function hasStarredSpecialForce(faction: Faction): boolean {
  return faction === "Fremen" || faction === "Emperor" || faction === "Ixian";
}

export function ForceToken({ faction, special = false, size = 24, title = "", className = "", style, imgClass = "", imgStyle }: {
  faction: Faction;
  special?: boolean;
  size?: number;
  title?: string;
  className?: string;
  style?: CSSProperties;
  imgClass?: string;
  imgStyle?: CSSProperties;
}) {
  const { skin } = useGame();
  const starred = special && hasStarredSpecialForce(faction);
  const src = skin.url(skin.get(special ? "FactionSpecialForceImage_URL" : "FactionForceImage_URL", faction));
  const cls = ["force-token", starred ? "starred" : "", className].filter((c) => c).join(" ");
  return (
    <span className={cls} style={{ ["--d" as string]: `${size}px`, ...style }} title={title}>
      <img src={src} width={size} height={size} alt={title} className={imgClass || undefined} style={imgStyle} />
    </span>
  );
}
