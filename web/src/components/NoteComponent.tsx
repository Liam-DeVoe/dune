// OtherComponents/NoteComponent.razor: the notekeeping assistant. A button per other faction; the
// chosen one's spice, reserves, the treachery cards they own (face up when the viewer knows them),
// revealed traitors and face dancers, victory points and, for Atreides, the Messiah. The data is
// the view's `notes`, which the server fills only with assisted notekeeping.
import { useState } from "react";
import type { Faction, LeaderSkill } from "../generated/enums";
import type { HeroRef } from "../generated/HeroRef";
import type { NotesPlayer } from "../generated/NotesPlayer";
import { leaderPopup, messiahPopup, treacheryCardPopup } from "../popups";
import type { Skin } from "../skin";
import { useGame } from "../state/context";
import { CollapseButton } from "./CollapseButton";
import { ForceToken } from "./ForceToken";
import { Hoverable } from "./Hoverable";
import { Image } from "./Image";
import { SimpleNumber } from "./SimpleNumber";
import { SpiceLine } from "./SpiceLine";

const unknownPopover = "<div class='p-1 small' style='color:var(--parchment);background-color:var(--ink-0);border: 1px solid rgba(255,255,255,.16);'>Unknown</div>";

// `open`: opened up, as when shown in a popover from the menu bar (InPopover).
export function NoteComponent({ open = false }: { open?: boolean }) {
  const { view, skin } = useGame();
  const [collapsed, setCollapsed] = useState(!open);
  const [shown, setShown] = useState<Faction | null>(null);
  const notes = view.notes;
  if (!notes) return null;
  // Players, ordered by the skin's faction names.
  const players = [...notes.players].sort((a, b) => skin.faction(a.faction).localeCompare(skin.faction(b.faction)));
  const p = notes.players.find((x) => x.faction === shown);
  return (
    <div className="card mb-2">
      <div className="card-header" onClick={() => setCollapsed(!collapsed)} style={{ cursor: "pointer" }}>
        Mentat: Notekeeping Assistant{" "}
        <CollapseButton collapsed={collapsed} />
      </div>
      {!collapsed && (
        <div className="card-body">
          <div className="mb-1">
            {players.map((x) => (
              <button key={x.faction} className={`btn ${x.faction === shown ? "btn-primary" : "btn-outline-primary"} p-1`}
                      onClick={() => setShown(x.faction === shown ? null : x.faction)} data-bs-toggle="tooltip" title={skin.faction(x.faction)}>
                <img className="img-fluid" width="29" height="29" src={skin.url(skin.get("FactionTableImage_URL", x.faction))} />
              </button>
            ))}
          </div>
          {p && <Noted p={p} />}
        </div>
      )}
    </div>
  );
}

function Noted({ p }: { p: NotesPlayer }) {
  const { view, skin } = useGame();
  const pub = view.players.find((x) => x.faction === p.faction);
  const messiah = <img className="img-fluid m-0 p-0" width="32" height="32" src={skin.url(skin.str("Messiah_URL"))} title={skin.concept("Messiah")} />;
  return (
    <>
      <div className="row m-0">
        {/* Resources */}
        <div className="col col-auto ps-0">
          <SpiceLine text={resourcesText(skin, p)} size={24} />
        </div>
        {/* Forces */}
        <div className="col col-auto ps-0">
          <ForceToken faction={p.faction} size={24} imgClass="img-fluid m-0 p-0" title={skin.get("ForceName_STR", p.faction)} />
          <strong>{"\u00a0"}{pub?.forcesInReserve}</strong>
        </div>
        {/* Special Forces */}
        {pub?.hasSpecialForces && (
          <div className="col col-auto ps-0">
            <ForceToken faction={p.faction} special size={24} imgClass="img-fluid m-0 p-0" title={skin.get("SpecialForceName_STR", p.faction)} />
            <strong>{"\u00a0"}{pub.specialForcesInReserve}</strong>
          </div>
        )}
      </div>

      {p.cards.length > 0
        ? (
          <>
            <div>Treachery cards</div>
            <div className="row row-cols-4 m-0">
              {p.cards.map((c, i) => (
                <Hoverable key={i} className="card border-0 p-1" popover={c !== null ? treacheryCardPopup(skin, c) : unknownPopover}>
                  <img src={c !== null ? skin.cardImage(c) : skin.url(skin.str("CardBack_TreacheryCard_URL"))} className="img-fluid" />
                </Hoverable>
              ))}
            </div>
          </>
        )
        : <div>They don't own any Treachery cards</div>}

      {p.revealedTraitors.length > 0 && (
        <>
          <div className="mt-1">{p.revealedTraitors.length > 1 ? "Revealed Traitors" : "Revealed Traitor"}</div>
          <div className="row row-cols-4 m-0">
            {p.revealedTraitors.map((h, i) => <HeroImage key={i} hero={h} />)}
          </div>
        </>
      )}

      {p.revealedFaceDancers.length > 0 && (
        <>
          <div className="mt-1">Revealed Face Dancers</div>
          <div className="row row-cols-4 m-0">
            {p.revealedFaceDancers.map((h, i) => <HeroImage key={i} hero={h} />)}
          </div>
        </>
      )}

      <div>Current victory points: <strong>{p.victoryPoints}</strong></div>

      {p.messiah && (
        p.messiah.forcesKilledInBattle >= 7
          ? <div>{messiah}{p.messiah.available ? " is available." : " is dead."}</div>
          : <div>{messiah} available after <SimpleNumber>{7 - p.messiah.forcesKilledInBattle}</SimpleNumber> more forces are killed.</div>
      )}
    </>
  );
}

// <Image Class="img-fluid pe-1" Shown=hero Width=60/>: the hero with its popover. The popover's
// skill card is left out: the notes carry no skills.
function HeroImage({ hero }: { hero: HeroRef }) {
  const { skin } = useGame();
  const [src, alt, popover] = heroImage(skin, hero);
  return <Image className="img-fluid pe-1" src={src} alt={alt} width={60} popover={popover} />;
}

function heroImage(skin: Skin, h: HeroRef): [string, string, string] {
  switch (h.t) {
    case "leader": return [skin.personImage(h.id), skin.person(h.id), leaderPopup(skin, h.id, null as LeaderSkill | null)];
    case "messiah": return [skin.url(skin.str("Messiah_URL")), skin.concept("Messiah"), messiahPopup(skin)];
    case "treacheryCard": return [skin.cardImage(h.id), skin.card(h.id), treacheryCardPopup(skin, h.id)];
  }
}

// NoteComponent.ResourcesText: spice, then bribes at the end of the turn and spice after bidding.
function resourcesText(skin: Skin, p: NotesPlayer): string {
  const phase = (ph: string) => skin.enumValue("MainPhase", ph);
  if (p.bribes > 0) {
    if (p.resourcesAfterBidding > 0) return `${p.resources}, ${p.bribes} at end of turn, ${p.resourcesAfterBidding} at end of bidding`;
    return `${p.resources}, ${p.bribes} at ${phase("Contemplate")}`;
  }
  if (p.resourcesAfterBidding > 0) return `${p.resources}, ${p.resourcesAfterBidding} at end of ${phase("Bidding")}`;
  return `${p.resources}`;
}
