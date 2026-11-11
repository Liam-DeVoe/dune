// GenericComponents/MonsterIcon.razor: the worm.
import { useGame } from "../state/context";

export function MonsterIcon({ size = 20 }: { size?: number }) {
  const { skin } = useGame();
  const label = skin.concept("Monster");
  return (
    <img src={skin.url(skin.str("Monster_URL"))} alt={label} width={size} height={size} title={label}
      style={{ filter: "drop-shadow(2px 2px 2px black)", verticalAlign: "middle" }} />
  );
}
