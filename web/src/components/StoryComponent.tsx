// OtherComponents/StoryComponent.razor, StoryPlayerInfoComponent.razor and
// StoryBattleComponent.razor: the story of a finished game in a modal. The server replays the
// game and sends the blocks to draw (rust/view/src/ending.rs); the secrets of factions that hid
// them are not in the view, and their cells read "?".
import { Fragment } from "react";
import type { Faction, TechToken } from "../generated/enums";
import type { HeroRef } from "../generated/HeroRef";
import type { StoryBlock } from "../generated/StoryBlock";
import type { StoryPlan } from "../generated/StoryPlan";
import type { StoryTable } from "../generated/StoryTable";
import { leaderPopup, messiahPopup, techTokenPopup, ambassadorPopup, terrorPopup, treacheryCardPopup } from "../popups";
import type { Skin } from "../skin";
import { useGame } from "../state/context";
import { FactionChip } from "./FactionChip";
import { ForceToken } from "./ForceToken";
import { Image } from "./Image";
import { ModalComponent } from "./ModalComponent";
import { PlaceChip } from "./PlaceChip";
import { PopoverText } from "./PopoverText";
import { RichText } from "./RichText";
import { SpiceComponent } from "./SpiceComponent";

// The razor files' &nbsp; (U+00A0).
const NBSP = " ";

export function StoryComponent({ id, story }: { id: string; story: StoryBlock[] }) {
  return (
    <ModalComponent id={id} title="The story of this game..." size="modal-xl" scrolling printable>
      {story.map((b, i) => <Block key={i} block={b} />)}
    </ModalComponent>
  );
}

function tableImage(skin: Skin, f: Faction): string {
  return skin.url(skin.get("FactionTableImage_URL", f));
}

// Image.razor with Shown set to an IHero: its image, its name as alt, its rich popover.
function HeroImage({ hero, width, className, greyedOut = false }: { hero: HeroRef; width: number; className: string; greyedOut?: boolean }) {
  const { skin } = useGame();
  const [src, alt, popover] =
    hero.t === "leader" ? [skin.personImage(hero.id), skin.person(hero.id), leaderPopup(skin, hero.id, null)]
    : hero.t === "messiah" ? [skin.url(skin.str("Messiah_URL")), skin.concept("Messiah"), messiahPopup(skin)]
    : [skin.cardImage(hero.id), skin.card(hero.id), treacheryCardPopup(skin, hero.id)];
  return <Image src={src} alt={alt} width={width} className={className} popover={popover} greyedOut={greyedOut} />;
}

function CardImage({ id, width }: { id: number; width: number }) {
  const { skin } = useGame();
  return <Image src={skin.cardImage(id)} alt={skin.card(id)} width={width} className="img-fluid p-1" popover={treacheryCardPopup(skin, id)} />;
}

function MessiahImage() {
  const { skin } = useGame();
  return <img className="img-fluid p-1" width={60} src={skin.url(skin.str("Messiah_URL"))} title={skin.concept("Messiah")} />;
}

// Skin.Describe(FactionForce) and Describe(FactionSpecialForce): the force's name for the faction.
function forceName(skin: Skin, f: Faction, special: boolean): string {
  return skin.get(special ? "SpecialForceName_STR" : "ForceName_STR", f);
}

function Block({ block: b }: { block: StoryBlock }) {
  const { skin } = useGame();
  switch (b.kind) {
    case "setup":
      return (
        <>
          <h4 className="h4">Setup</h4>
          <div className="mt-1"><strong>Ruleset:</strong> {b.ruleset}</div>
          {b.customRules !== null && <div className="mt-1"><strong>Custom rules:</strong> {b.customRules}</div>}
          <div className="mt-1"><strong>Game started:</strong> {b.started}</div>
          <PlayerInfo table={b.table} />
        </>
      );
    case "prediction":
      return (
        <div>
          <FactionChip faction="BeneGesserit" /> Predicted that <FactionChip faction={b.toWin} /> win in turn {b.turn}.
        </div>
      );
    case "purchase":
      return (
        <div>
          <FactionChip faction={b.buyer} /> buy{" "}
          {b.card ? <PopoverText popover={treacheryCardPopup(skin, b.card.id)}>{b.card.label}</PopoverText> : <span className="fw-bold">?</span>}
          {" "}for <SpiceComponent amount={String(b.amount)} />
          {b.karmaCard && <span> using <PopoverText popover={treacheryCardPopup(skin, b.karmaCard.id)}>{b.karmaCard.label}</PopoverText></span>}
          {b.auction !== null && <span> ({b.auction})</span>}
        </div>
      );
    case "turn":
      return <h4 className="h4">Turn {b.turn}</h4>;
    case "situation":
      return (
        <>
          <h5 className="mt-2">
            {b.when === "startOfBidding" ? "Situation at the start of Bidding:"
              : b.when === "endOfBidding" ? "Situation at the end of Bidding:"
              : b.when === "endOfShipmentAndMove" ? "Situation at the end of Ship & Move:"
              : <>Situation at the end of turn {b.turn}:</>}
          </h5>
          <PlayerInfo table={b.table} />
        </>
      );
    case "battleFor":
      return <h5 className="mt-2">Battle for <PlaceChip place={{ territory: b.territory }} /></h5>;
    case "milestone":
      return (
        <div className="mb-1">
          <img className="img-fluid" width="24" src={skin.factionImage(b.faction)} /> <RichText parts={b.text} />
        </div>
      );
    case "battle":
      return <BattleTable aggressor={b.aggressor} defender={b.defender} advancedCombat={b.advancedCombat} />;
    case "strongholds":
      return (
        <table className="table table-sm">
          <thead>
            <tr className="table-dark">
              <th>Stronghold</th>
              <th>Forces</th>
              {b.ownerColumn && <th>Owned by</th>}
              {b.terrorColumn && <th>Ambassadors and Terror</th>}
            </tr>
          </thead>
          <tbody>
            {b.rows.map((r) => (
              <tr key={r.location} className="table-dark">
                <td><PlaceChip place={{ location: r.location }} /></td>
                <td>
                  {r.battalions.map((x, i) => (
                    <Fragment key={i}>
                      {x.forces > 0 && (
                        <>
                          <ForceToken faction={x.faction} size={24} title={forceName(skin, x.faction, false)} />
                          <strong>{x.forces}</strong>
                          <span>{NBSP}</span>
                        </>
                      )}
                      {x.specialForces > 0 && (
                        <>
                          <ForceToken faction={x.faction} special size={24} title={forceName(skin, x.faction, true)} />
                          <strong>{x.specialForces}</strong>
                        </>
                      )}
                    </Fragment>
                  ))}
                </td>
                {b.ownerColumn && <td>{r.owner !== null && <FactionChip faction={r.owner} />}</td>}
                {b.terrorColumn && (
                  <td>
                    {r.ambassador !== null && (
                      <Image width={24} src={skin.url(skin.get("AmbassadorImage_URL", r.ambassador))} alt={skin.get("AmbassadorName_STR", r.ambassador)} popover={ambassadorPopup(skin, r.ambassador)} />
                    )}
                    {r.terror.map((t, i) => (
                      // Skin.GetImageUrl has no case for TerrorType: an empty src.
                      <Image key={i} width={24} src="" alt={skin.get("TerrorTokenName_STR", t)} popover={terrorPopup(skin, t)} />
                    ))}
                  </td>
                )}
              </tr>
            ))}
          </tbody>
        </table>
      );
    case "winner":
      return (
        <div>
          <h3 className="h3"><FactionChip faction={b.faction} /> {b.name} wins {b.method}!</h3>
        </div>
      );
    case "ended":
      return <div className="mt-2 mb-0"><strong>Game ended:</strong> {b.time}</div>;
  }
}

// StoryPlayerInfoComponent.razor.
function PlayerInfo({ table: t }: { table: StoryTable }) {
  const { skin } = useGame();
  return (
    <div>
      <table className="table table-sm">
        <thead>
          <tr className="table-dark">
            <th>Faction</th>
            {t.showName && <th>Player</th>}
            {t.showAlly && <th>{t.nexusColumn ? "Ally or Nexus*" : "Ally"}</th>}
            {t.showCards && <th>Cards</th>}
            {t.showTraitors && <th>Traitors or Facedancers</th>}
            {t.showTechTokens && <th>Tech</th>}
            {t.showResources && <th>Spice</th>}
            {t.showForces && <th>Forces on planet</th>}
            {t.showLeaders && <th>Available leaders</th>}
            {t.showTanks && <th>Tanks</th>}
            {t.showPoints && <th>Points</th>}
          </tr>
        </thead>
        <tbody>
          {t.rows.map((p) => (
            <tr key={p.faction} className="table-dark">
              <td><img className="img-fluid p-1" width={60} src={tableImage(skin, p.faction)} /></td>
              {t.showName && <td><h2>{p.name}</h2></td>}
              {t.showAlly && (
                <td>
                  {p.ally !== null ? <img className="img-fluid p-1" width={60} src={tableImage(skin, p.ally)} />
                    : p.nexus !== null && <><img className="img-fluid p-1" width={60} src={tableImage(skin, p.nexus)} /><strong>*</strong></>}
                </td>
              )}
              {t.showCards && <td>{p.secret ? <h2>?</h2> : p.cards.map((c, i) => <CardImage key={i} id={c} width={80} />)}</td>}
              {t.showTraitors && (
                <td>{p.secret ? <h2>?</h2> : p.traitors.map((x, i) => <HeroImage key={i} hero={x.hero} width={60} className="img-fluid p-1" greyedOut={x.discarded} />)}</td>
              )}
              {t.showTechTokens && (
                <td>
                  {p.techTokens.map((tt: TechToken) => (
                    <Image key={tt} src={skin.url(skin.get("TechTokenImage_URL", tt))} alt={skin.get("TechTokenName_STR", tt)} width={60} className="img-fluid p-1" popover={techTokenPopup(skin, tt)} />
                  ))}
                </td>
              )}
              {t.showResources && <td><h2>{p.resources ?? "?"}</h2></td>}
              {t.showForces && (
                <td>
                  {p.forces.map((l, i) => (
                    <div key={i} className={l.contested ? "text-danger" : ""}>
                      {l.contested && <span>! </span>}
                      <PlaceChip place={{ territory: l.territory }} />
                      <span>:{NBSP}</span>
                      {l.forces > 0 && (
                        <>
                          <ForceToken faction={p.faction} size={24} imgClass="img-fluid m-0 p-0" title={forceName(skin, p.faction, false)} />
                          <strong>{NBSP}{l.forces}</strong>
                        </>
                      )}
                      <span>{NBSP}</span>
                      {l.specialForces > 0 && (
                        <>
                          <ForceToken faction={p.faction} special size={24} imgClass="img-fluid m-0 p-0" title={forceName(skin, p.faction, true)} />
                          <strong>{NBSP}{l.specialForces}</strong>
                        </>
                      )}
                    </div>
                  ))}
                </td>
              )}
              {t.showLeaders && (
                <td>
                  {p.leaders.map((h, i) => <HeroImage key={i} hero={h} width={60} className="img-fluid p-1" />)}
                  {p.messiahAvailable && <MessiahImage />}
                </td>
              )}
              {t.showTanks && (
                <td>
                  {p.forcesKilled > 0 && (
                    <>
                      <ForceToken faction={p.faction} size={24} imgClass="img-fluid m-0 p-0" title={forceName(skin, p.faction, false)} />
                      <strong>{NBSP}{p.forcesKilled}</strong>
                    </>
                  )}
                  {p.specialForcesKilled > 0 && (
                    <>
                      <ForceToken faction={p.faction} special size={24} imgClass="img-fluid m-0 p-0" title={forceName(skin, p.faction, true)} />
                      <strong>{NBSP}{p.specialForcesKilled}</strong>
                    </>
                  )}
                  {p.deadLeaders.map((h, i) => <HeroImage key={i} hero={h} width={60} className="img-fluid p-1" />)}
                  {p.messiahDead && <MessiahImage />}
                </td>
              )}
              {t.showPoints && <td><h2>{p.points}</h2></td>}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

const THUMBS_UP = "M6.956 1.745C7.021.81 7.908.087 8.864.325l.261.066c.463.116.874.456 1.012.965.22.816.533 2.511.062 4.51a9.84 9.84 0 0 1 .443-.051c.713-.065 1.669-.072 2.516.21.518.173.994.681 1.2 1.273.184.532.16 1.162-.234 1.733.058.119.103.242.138.363.077.27.113.567.113.856 0 .289-.036.586-.113.856-.039.135-.09.273-.16.404.169.387.107.819-.003 1.148a3.163 3.163 0 0 1-.488.901c.054.152.076.312.076.465 0 .305-.089.625-.253.912C13.1 15.522 12.437 16 11.5 16H8c-.605 0-1.07-.081-1.466-.218a4.82 4.82 0 0 1-.97-.484l-.048-.03c-.504-.307-.999-.609-2.068-.722C2.682 14.464 2 13.846 2 13V9c0-.85.685-1.432 1.357-1.615.849-.232 1.574-.787 2.132-1.41.56-.627.914-1.28 1.039-1.639.199-.575.356-1.539.428-2.59z";

// StoryBattleComponent.razor.
function BattleTable({ aggressor, defender, advancedCombat }: { aggressor: StoryPlan; defender: StoryPlan; advancedCombat: boolean }) {
  return (
    <div>
      <table className="table table-sm">
        <thead>
          <tr className="table-dark">
            <th>Faction</th>
            <th>Leader</th>
            <th>Weapon</th>
            <th>Defense</th>
            <th>Dial</th>
            {advancedCombat && <th>Spice</th>}
          </tr>
        </thead>
        <tbody>
          <PlanRow plan={aggressor} advancedCombat={advancedCombat} />
          <PlanRow plan={defender} advancedCombat={advancedCombat} />
        </tbody>
      </table>
    </div>
  );
}

function PlanRow({ plan: p, advancedCombat }: { plan: StoryPlan; advancedCombat: boolean }) {
  const { skin } = useGame();
  const background = p.winner ? "rgba(121,179,76,.2)" : "rgba(195,50,50,.2)";
  return (
    <tr className="table-dark" style={{ backgroundColor: background }}>
      <td>
        <img className="img-fluid p-1" width="60" src={tableImage(skin, p.faction)} />
        {p.winner && (
          <svg xmlns="http://www.w3.org/2000/svg" width="30" height="30" fill="#e7bf3c" className="bi bi-hand-thumbs-up-fill" viewBox="0 0 16 16">
            <path d={THUMBS_UP} />
          </svg>
        )}
      </td>
      <td>
        {p.hero && <HeroImage hero={p.hero} width={60} className="img-fluid p-1" />}
        {p.messiah && <MessiahImage />}
      </td>
      <td>{p.weapon !== null && <CardImage id={p.weapon} width={80} />}</td>
      <td>{p.defense !== null && <CardImage id={p.defense} width={80} />}</td>
      <td><h2>{p.dial}</h2></td>
      {advancedCombat && <td><h2>{p.cost}</h2></td>}
    </tr>
  );
}
