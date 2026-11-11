// GenericComponents/SpiceComponent.razor: a spice amount drawn on the harvester icon.
import { useGame } from "../state/context";

export function SpiceComponent({ amount, size = 20 }: { amount: string; size?: number }) {
  const { skin } = useGame();
  const name = skin.concept("Resource");
  return (
    <span className="spice-amt" style={{ ["--s" as string]: `${size}px` }} title={name}>
      <img src={skin.url(skin.str("Harvester_URL"))} alt={name} />
      <b>{amount}</b>
    </span>
  );
}
