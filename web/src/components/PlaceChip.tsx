// GenericComponents/PlaceChip.razor (Support.PlaceChipHtml): a location or territory as a pill
// with a glyph of the planet showing where it lies, its territory name and its compass word.
// data-ref-locations is for the map's hover tinting. The markup is popups.ts's placeChipHtml,
// as JSX.
import { useMemo } from "react";
import { placeChipData, type Place } from "../popups";
import { useGame } from "../state/context";

export function PlaceChip({ place }: { place: Place }) {
  const { skin, view } = useGame();
  // A Hidden Mobile Stronghold or discovered stronghold takes its host's sector (Location.Sector).
  const attached = "location" in place ? view.board?.attached?.find((a) => a.location === place.location) : undefined;
  const key = JSON.stringify(place) + (attached?.sector ?? "");
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const c = useMemo(() => placeChipData(skin, attached ? { ...place, sector: attached.sector } : place), [skin, key]);
  return (
    <span className="chip place" data-ref-locations={c.locations.join(",")} title={c.title}>
      <svg className="wedge" viewBox={c.view} aria-hidden="true">
        <defs>
          <clipPath id={`${c.id}-disc`}><circle cx={c.cx} cy={c.cy} r={c.r} /></clipPath>
          {c.wedge !== null && <clipPath id={`${c.id}-sector`}><path d={c.wedge} /></clipPath>}
        </defs>
        <g clipPath={`url(#${c.id}-disc)`}>
          <circle className="disc" cx={c.cx} cy={c.cy} r={c.r} />
          {c.border !== "" && (
            <g transform={c.transform}>
              <g clipPath={c.wedge !== null ? `url(#${c.id}-sector)` : undefined}><path className="w" d={c.border} /></g>
            </g>
          )}
        </g>
        <circle className="ring" cx={c.cx} cy={c.cy} r={c.r} vectorEffect="non-scaling-stroke" />
      </svg>
      <span className="name">{c.name}</span>
      {c.orientation !== "" && <span className="sector">{c.orientation}</span>}
    </span>
  );
}
