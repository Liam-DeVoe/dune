// GenericComponents/FactionChip.razor: the sigil and name in a pill, gold when it is the
// viewer's own faction. data-ref-locations lists the locations holding the faction's forces,
// for the hover script that tints them on the map.
import type { Faction } from "../generated/enums";
import { useGame } from "../state/context";

// `className` mixes in an element of the block that places the chip (player-panel__chip).
export function FactionChip({ faction, className = "" }: { faction: Faction; className?: string }) {
  const { view, skin } = useGame();
  const isYou = !view.viewer.isObserver && view.viewer.faction === faction;
  const locations = view.board.forces.filter((f) => f.faction === faction).map((f) => f.location);
  return (
    <span
      className={`chip chip--faction ${isYou ? "chip--you" : ""} ${className}`}
      data-ref-faction={faction}
      data-ref-locations={[...new Set(locations)].join(",")}
      style={{ ["--fc" as string]: skin.factionColor(faction) }}
      title={skin.faction(faction)}
    >
      <img className="chip__img" src={skin.factionImage(faction)} alt="" />
      <span className="chip__name">{skin.faction(faction)}</span>
    </span>
  );
}
