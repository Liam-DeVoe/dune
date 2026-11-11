// GenericComponents/FactionComponent.razor: a faction's sigil as an image.
import type { Faction } from "../generated/enums";
import { useGame } from "../state/context";

export function FactionComponent({ faction, title = "", diameter = 30, hasShadow = true, marginClass = "", onClick }: {
  faction: Faction;
  title?: string;
  diameter?: number;
  hasShadow?: boolean;
  marginClass?: string;
  onClick?: () => void;
}) {
  const { skin } = useGame();
  const t = title || skin.faction(faction);
  const style = { ...(hasShadow ? { filter: "drop-shadow(1px 1px 1px black)" } : {}), ...(onClick ? { cursor: "pointer" } : {}) };
  return (
    <img className={`img-fluid ${marginClass}`.trim()} src={skin.factionImage(faction)} width={diameter} height={diameter}
         alt={t} title={t} style={style} onClick={onClick} />
  );
}
