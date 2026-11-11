// GenericComponents/FactionChip.razor: the sigil and name in a pill, gold when it is the
// viewer's own faction. data-ref-locations lists the locations holding the faction's forces,
// for the hover script that tints them on the map.
import type { Faction } from "../generated/enums";
import { useGame } from "../state/context";

export function FactionChip({ faction }: { faction: Faction }) {
  const { view, skin } = useGame();
  const isYou = !view.viewer.isObserver && view.viewer.faction === faction;
  const locations = view.board.forces.filter((f) => f.faction === faction).map((f) => f.location);
  return (
    <span
      className={`chip faction ${isYou ? "you" : ""}`}
      data-ref-faction={faction}
      data-ref-locations={[...new Set(locations)].join(",")}
      style={{ ["--fc" as string]: skin.factionColor(faction) }}
      title={skin.faction(faction)}
    >
      <img src={skin.factionImage(faction)} alt="" />
      <span className="name">{skin.faction(faction)}</span>
    </span>
  );
}
