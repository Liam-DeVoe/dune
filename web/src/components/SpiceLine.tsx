// Support.SpiceLineHtml: each number in the text as a spice amount (the first one larger), the
// rest in bold. The faction panel and the notekeeping assistant draw spice with it.
import { useGame } from "../state/context";

export function SpiceLine({ text, size }: { text: string; size: number }) {
  const { skin } = useGame();
  const name = skin.concept("Resource");
  let first = true;
  return (
    <>
      {text.split(/(\d+)/).filter((p) => p.length > 0).map((part, i) => {
        if (!/^\d/.test(part)) return <strong key={i}>{part}</strong>;
        const s = first ? size : size - 6;
        first = false;
        return (
          <span key={i} className="spice-amt" style={{ ["--s" as string]: `${s}px` }} title={name}>
            <img src={skin.url(skin.str("Harvester_URL"))} alt={name} />
            <b>{part}</b>
          </span>
        );
      })}
    </>
  );
}
