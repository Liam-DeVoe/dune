// OtherComponents/TrackerComponent.razor, CardPickerComponent.razor and AtreidesIntelligence.cs:
// Atreides's card tracker. Atreides notes which cards each player holds, which traitors were
// revealed or discarded, the top spice card and the discarded treachery cards. The notes stay in
// the browser's local storage per game, in AtreidesIntelligence's text format; the game data
// they choose from is the view's `tracker`.
//
// C# keys the notes by Game.Seed, which the view must not carry (it would give away the decks), so
// they are keyed by the game's start time.
import { useEffect, useMemo, useState } from "react";
import type { Faction } from "../generated/enums";
import type { HeroRef } from "../generated/HeroRef";
import type { TrackerView } from "../generated/TrackerView";
import { leaderPopup, messiahPopup, resourceCardPopup, treacheryCardPopup } from "../popups";
import type { Skin } from "../skin";
import { useGame } from "../state/context";
import { CollapseButton } from "./CollapseButton";
import { Hoverable } from "./Hoverable";
import { Image } from "./Image";
import { ModalComponent } from "./ModalComponent";
import { PickFromImageComponent, type PickItem } from "./PickFromImageComponent";
import { PopoverText } from "./PopoverText";

// TreacheryCard.Unknown and TreacheryCard.None.
const UNKNOWN = -1;
const NONE = -2;

// AtreidesIntelligence: the notes.
interface Notes {
  cards: Map<Faction, number[]>;
  traitors: Map<string, number>; // "<faction>,<nr>" -> hero id
  discardedTraitors: Map<number, number>;
  spiceCard: string | null;
  discarded: number[];
  removed: number[];
}

function fresh(t: TrackerView): Notes {
  const cards = new Map<Faction, number[]>();
  for (const p of t.players) {
    // DefaultSelectedCard: the first card is unknown (Harkonnen's first two), the rest none.
    cards.set(p.faction, Array.from({ length: p.maximumNumberOfCards }, (_, i) => (i === 0 || (i === 1 && p.faction === "Harkonnen") ? UNKNOWN : NONE)));
  }
  return { cards, traitors: new Map(), discardedTraitors: new Map(), spiceCard: null, discarded: [], removed: [] };
}

// AtreidesIntelligence.ToString.
function write(n: Notes): string {
  let r = "";
  const w = (x: unknown) => { r += x === null ? ";" : `${String(x)};`; };
  for (const cards of n.cards.values()) for (const c of cards) w(c);
  w(n.traitors.size);
  for (const [k, v] of n.traitors) { const [f, nr] = k.split(","); w(f); w(nr); w(v); }
  w(n.discardedTraitors.size);
  for (const [k, v] of n.discardedTraitors) { w(k); w(v); }
  w(n.spiceCard);
  w(n.discarded.length);
  for (const c of n.discarded) w(c);
  return r;
}

// AtreidesIntelligence.Parse: fresh notes when the text doesn't parse.
function parse(t: TrackerView, data: string): Notes {
  const e = data.split(";");
  let i = 0;
  const int = () => { const v = Number.parseInt(e[i++], 10); if (Number.isNaN(v)) throw new Error("bad tracker data"); return v; };
  try {
    const n = fresh(t);
    for (const cards of n.cards.values()) for (let k = 0; k < cards.length; k++) cards[k] = int();
    let count = int();
    for (let k = 0; k < count; k++) { const f = e[i++]; const nr = int(); n.traitors.set(`${f},${nr}`, int()); }
    count = int();
    for (let k = 0; k < count; k++) { const nr = int(); n.discardedTraitors.set(nr, int()); }
    n.spiceCard = e[i++] || null;
    if (i < e.length - 1) {
      count = int();
      for (let k = 0; k < count; k++) n.discarded.push(int());
    }
    return n;
  } catch {
    return fresh(t);
  }
}

const storageKey = (started: string) => `treachery.online;currentgame;${started};trackerdata`;

type Shown = { faction: Faction } | "discarded" | "spice" | null;

// `open`: opened up, as when shown in a popover from the menu bar (InPopover).
export function TrackerComponent({ open = false }: { open?: boolean }) {
  const { view, skin } = useGame();
  const t = view.tracker;
  const key = storageKey(view.timing.started);
  const [notes, setNotes] = useState<Notes | null>(null);
  const [collapsed, setCollapsed] = useState(!open);
  const [shown, setShown] = useState<Shown>(null);
  const [picking, setPicking] = useState<[Faction, number] | null>(null);

  useEffect(() => {
    if (!t) return;
    const saved = localStorage.getItem(key);
    setNotes(saved ? parse(t, saved) : fresh(t));
    // Loaded once per game.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, t === null]);

  const traitors = useMemo(() => (t ? [...t.traitorsInPlay].sort((a, b) => heroName(skin, a).localeCompare(heroName(skin, b))) : []), [t, skin]);
  const spiceCards = useMemo(() => (t ? [...t.resourceCards].sort((a, b) => resourceName(skin, a).localeCompare(resourceName(skin, b))) : []), [t, skin]);

  if (!t || !notes) return null;
  const save = (n: Notes) => { setNotes({ ...n }); localStorage.setItem(key, write(n)); };
  const players = [...t.players].sort((a, b) => skin.faction(a.faction).localeCompare(skin.faction(b.faction)));
  const toggle = (s: Shown) => setShown(same(s, shown) ? null : s);
  const style = (s: Shown) => (same(s, shown) ? "btn-primary" : "btn-outline-primary");
  const heroItem = (id: number): PickItem<number> | null => {
    const h = traitors.find((x) => heroId(x) === id);
    return h ? heroPick(skin, h) : null;
  };
  const heroItems = traitors.map((h) => heroPick(skin, h));
  const shownPlayer = shown !== null && typeof shown === "object" ? t.players.find((p) => p.faction === shown.faction) : undefined;

  // The card picker: AvailableDistinctCards for the place being picked.
  const current = picking ? notes.cards.get(picking[0])?.[picking[1]] ?? NONE : NONE;
  const elsewhere = picking ? [...notes.cards.entries()].flatMap(([f, cs]) => cs.filter((_, i) => f !== picking[0] || i !== picking[1])) : [];
  const toPick: number[] = [];
  for (const c of t.cardsInPlay) {
    if (toPick.every((x) => skin.card(x) !== skin.card(c)) && !notes.removed.includes(c) && !notes.discarded.includes(c) && !elsewhere.includes(c)) toPick.push(c);
  }
  const justWon = view.me?.cardJustWon != null ? skin.card(view.me.cardJustWon) : "";
  const pick = (card: number) => {
    if (!picking) return;
    notes.cards.get(picking[0])![picking[1]] = card;
    save(notes);
    hideModal("picker");
  };
  const discard = () => {
    if (!picking || current === NONE || current === UNKNOWN) return;
    notes.cards.get(picking[0])![picking[1]] = NONE;
    if (skin.cardType(current) === "Metheor") notes.removed.push(current);
    else notes.discarded.push(current);
    save(notes);
    hideModal("picker");
  };
  const notDiscarded = (c: number) => {
    if (skin.cardType(c) === "Metheor") notes.removed = notes.removed.filter((x) => x !== c);
    else notes.discarded = notes.discarded.filter((x) => x !== c);
    save(notes);
  };
  const setTraitor = (map: Map<string, number> | Map<number, number>, k: string | number, hero: number | null) => {
    (map as Map<typeof k, number>).delete(k);
    if (hero !== null) (map as Map<typeof k, number>).set(k, hero);
    save(notes);
  };
  const discardedCards = [...new Set([...notes.discarded, ...notes.removed])];
  const resourceItem = (skinId: number): PickItem<number> => ({ key: skinId, src: skin.url(skin.get("ResourceCardImage_URL", skinId)), alt: resourceName(skin, skinId), popover: resourceCardPopup(skin, skinId) });
  const trackedSpice = spiceCards.find((c) => resourceName(skin, c) === notes.spiceCard);
  const pileText = (label: string, pile: number[]) => pile.length > 0 && (
    <div className="small"><strong>{label}</strong>{" "}
      {pile.map((c, i) => <span key={i}>{i > 0 && <span>, </span>}<PopoverText popover={resourceCardPopup(skin, c)}>{resourceName(skin, c)}</PopoverText></span>)}
    </div>
  );

  return (
    <>
      <ModalComponent id="picker" title="Pick a card">
        <div className="row row-cols-3 row-cols-md-6 m-0">
          <div className="card p-1 m-0 border-0">
            <img src={skin.url(skin.str("CardBack_TreacheryCard_URL"))} className="card-img" alt="Unknown" onClick={() => pick(UNKNOWN)} title="Set as unknown card" style={{ cursor: "pointer" }} />
          </div>
          {current !== NONE && (
            <div className="card p-1 m-0 border-0" title="Set to 'none'" style={{ cursor: "pointer" }}>
              <svg viewBox="0 0 16 22.5" xmlns="http://www.w3.org/2000/svg" onClick={() => pick(NONE)}>
                <rect width="16" height="22.5" rx="2" style={{ fill: "rgb(64,64,64)", strokeWidth: 1, stroke: "rgb(200,200,200)" }} />
              </svg>
            </div>
          )}
          {current !== NONE && current !== UNKNOWN && (
            <div className="card p-1 m-0 border-0" title="Discard this card" style={{ cursor: "pointer" }}>
              <svg viewBox="0 0 16 22.5" xmlns="http://www.w3.org/2000/svg" onClick={discard}>
                <rect width="16" height="22.5" rx="2" style={{ fill: "rgb(64,64,64)", strokeWidth: 1, stroke: "rgb(200,200,200)" }} />
                <path transform="translate(0,4)" fill="white" d="M5.5 5.5A.5.5 0 0 1 6 6v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5zm2.5 0a.5.5 0 0 1 .5.5v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5zm3 .5a.5.5 0 0 0-1 0v6a.5.5 0 0 0 1 0V6z" />
                <path transform="translate(0,4)" fill="white" fillRule="evenodd" d="M14.5 3a1 1 0 0 1-1 1H13v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V4h-.5a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1H6a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1h3.5a1 1 0 0 1 1 1v1zM4.118 4L4 4.059V13a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1V4.059L11.882 4H4.118zM2.5 3V2h11v1h-11z" />
              </svg>
            </div>
          )}
          {toPick.map((c) => {
            const won = justWon === skin.card(c);
            return (
              <div key={c} className={`card p-1 m-0 ${won ? "border-success" : "border-0"} ${won ? "bg-light" : ""}`}>
                <div className="card-img-top">
                  <Image className="card-img" src={skin.cardImage(c)} alt={skin.card(c)} width={0} popover={treacheryCardPopup(skin, c)} onClick={() => pick(c)} pointer />
                </div>
              </div>
            );
          })}
        </div>
      </ModalComponent>

      <div className="card mb-2" style={{ animation: "fadeIn 1s" }}>
        <div className="card-header" onClick={() => setCollapsed(!collapsed)} style={{ cursor: "pointer" }}>
          Card Tracker{" "}
          <Hoverable className="d-inline" popover={`<div class='pop'>${infoMessage(skin)}</div>`}>
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="var(--ink-0)" className="bi bi-info-circle" viewBox="0 0 16 16">
              <circle cx="8" cy="8" r="8" fill="var(--gold)" />
              <path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14zm0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16z" />
              <path d="M8.93 6.588l-2.29.287-.082.38.45.083c.294.07.352.176.288.469l-.738 3.468c-.194.897.105 1.319.808 1.319.545 0 1.178-.252 1.465-.598l.088-.416c-.2.176-.492.246-.686.246-.275 0-.375-.193-.304-.533L8.93 6.588zM9 4.5a1 1 0 1 1-2 0 1 1 0 0 1 2 0z" />
            </svg>
          </Hoverable>{" "}
          <CollapseButton collapsed={collapsed} />
        </div>
        {!collapsed && (
          <div className="card-body">
            <div className="mb-1">
              {players.map((p) => (
                <button key={p.faction} className={`btn ${style({ faction: p.faction })} p-1`} onClick={() => toggle({ faction: p.faction })} data-bs-toggle="tooltip" title={skin.faction(p.faction)}>
                  <img className="img-fluid" width="29" height="29" src={skin.url(skin.get("FactionTableImage_URL", p.faction))} alt={skin.faction(p.faction)} />
                </button>
              ))}
              <button className={`btn ${style("discarded")} p-1`} onClick={() => toggle("discarded")} data-bs-toggle="tooltip" title="Discarded Treachery Cards">
                <img className="img-fluid ms-1 me-1" width="21" height="29" src={skin.url(skin.str("CardBack_TreacheryCard_URL"))} alt="" />
              </button>
              <button className={`btn ${style("spice")} p-1`} onClick={() => toggle("spice")} data-bs-toggle="tooltip" title="Discarded Spice Cards">
                <img className="img-fluid ms-1 me-1" width="21" height="29" src={skin.url(skin.str("CardBack_ResourceCard_URL"))} alt="" />
              </button>
            </div>

            {shownPlayer && (
              <>
                <div>Treachery cards</div>
                <div className="row row-cols-4 m-0">
                  {(notes.cards.get(shownPlayer.faction) ?? []).map((c, i) => (
                    <a key={i} className="btn btn-link p-1" data-bs-toggle="modal" data-bs-target="#picker" onMouseDown={() => setPicking([shownPlayer.faction, i])}
                       title={c === UNKNOWN ? "Unknown" : c === NONE ? "None" : ""}>
                      {c === NONE
                        ? (
                          <svg className="img-fluid w-100" viewBox="0 0 268 372" xmlns="http://www.w3.org/2000/svg">
                            <rect width="268" height="372" rx="20" style={{ fill: "rgb(64,64,64)", strokeWidth: 3, stroke: "rgb(200,200,200)" }} />
                          </svg>
                        )
                        : c === UNKNOWN
                          ? <img src={skin.url(skin.str("CardBack_TreacheryCard_URL"))} className="img-fluid" alt="" />
                          : <Image src={skin.cardImage(c)} alt={skin.card(c)} width={0} className="img-fluid" popover={treacheryCardPopup(skin, c)} pointer />}
                    </a>
                  ))}
                </div>
                {shownPlayer.faction === "Atreides"
                  ? (
                    <>
                      <div>Traitors discarded at start</div>
                      {[0, 1, 2].map((nr) => (
                        <PickFromImageComponent key={nr} value={heroItem(notes.discardedTraitors.get(nr) ?? -1)} values={heroItems}
                          onChanged={(h) => setTraitor(notes.discardedTraitors, nr, h)} title="Select Discarded Traitor" imageWidth={60} />
                      ))}
                    </>
                  )
                  : (
                    <>
                      <div>{shownPlayer.faction === "Tleilaxu" ? "Revealed Face Dancers" : `Revealed ${shownPlayer.numberOfTraitors > 1 ? "Traitors" : "Traitor"}`}</div>
                      {Array.from({ length: shownPlayer.faction === "Tleilaxu" ? shownPlayer.numberOfFaceDancers : shownPlayer.numberOfTraitors }, (_, nr) => (
                        <PickFromImageComponent key={nr} value={heroItem(notes.traitors.get(`${shownPlayer.faction},${nr}`) ?? -1)} values={heroItems}
                          onChanged={(h) => setTraitor(notes.traitors, `${shownPlayer.faction},${nr}`, h)}
                          title={shownPlayer.faction === "Tleilaxu" ? "Select Revealed Face Dancer" : "Select Revealed Traitor"} imageWidth={60} />
                      ))}
                    </>
                  )}
              </>
            )}

            {shown === "discarded" && (
              <>
                <ul className="list-inline mt-0">
                  {discardedCards.map((c) => (
                    <li key={c} className="list-inline-item">
                      <p className="mb-0">
                        <PopoverText popover={treacheryCardPopup(skin, c)} className="badge bg-primary rounded-pill">{skin.card(c)}</PopoverText>{" "}
                        <button className="btn-close btn-sm" onClick={() => notDiscarded(c)}></button>
                      </p>
                    </li>
                  ))}
                  {discardedCards.length === 0 && <li className="list-inline-item text-info">No treachery cards discarded yet.</li>}
                </ul>
                <button className="btn btn-primary btn-sm" onClick={() => { notes.discarded = []; save(notes); }}>Reshuffle (clear) discarded cards</button>
              </>
            )}

            {shown === "spice" && (
              <>
                <div>Top {skin.concept("Resource")} Card</div>
                <PickFromImageComponent value={trackedSpice !== undefined ? resourceItem(trackedSpice) : null} values={spiceCards.map(resourceItem)}
                  onChanged={(c) => { notes.spiceCard = c === null ? null : resourceName(skin, c); save(notes); }} title="Select Card" imageWidth={60} />
                {view.rules.includes("IncreasedResourceFlow") || view.rulesForBots.includes("IncreasedResourceFlow")
                  ? <>{pileText("Discard pile A:", t.resourceDiscardA)}{pileText("Discard pile B:", t.resourceDiscardB)}</>
                  : pileText("Discarded:", t.resourceDiscardA)}
              </>
            )}
          </div>
        )}
      </div>
    </>
  );
}

function same(a: Shown, b: Shown): boolean {
  if (a === null || b === null || typeof a === "string" || typeof b === "string") return a === b;
  return a.faction === b.faction;
}

function hideModal(id: string) {
  const el = document.getElementById(id);
  const bs = (window as unknown as { bootstrap?: { Modal: { getInstance(e: Element): { hide(): void } | null } } }).bootstrap;
  if (el && bs) bs.Modal.getInstance(el)?.hide();
}

// The id LeaderManager.HeroLookup finds a hero by: a leader's id, the Messiah's, a card's.
function heroId(h: HeroRef): number {
  return h.t === "messiah" ? 1000 : h.id;
}

function heroName(skin: Skin, h: HeroRef): string {
  return h.t === "leader" ? skin.person(h.id) : h.t === "messiah" ? skin.concept("Messiah") : skin.card(h.id);
}

function heroPick(skin: Skin, h: HeroRef): PickItem<number> {
  const [src, popover] = h.t === "leader" ? [skin.personImage(h.id), leaderPopup(skin, h.id, null)]
    : h.t === "messiah" ? [skin.url(skin.str("Messiah_URL")), messiahPopup(skin)]
    : [skin.cardImage(h.id), treacheryCardPopup(skin, h.id)];
  return { key: heroId(h), src, alt: heroName(skin, h), popover };
}

// Skin.Describe(ResourceCard).
function resourceName(skin: Skin, skinId: number): string {
  const c = skin.resourceCardInfo(skinId);
  if (!c) return "?";
  if (c.isShaiHulud) return skin.concept("Monster");
  if (c.isGreatMaker) return skin.concept("GreatMonster");
  if (c.isSandTrout) return skin.concept("BabyMonster");
  return c.location !== null ? skin.territory(skin.locationInfo(c.location)?.territory ?? -1) : "?";
}

function infoMessage(skin: Skin): string {
  return `Welcome to the ${skin.faction("Atreides")} Tracker! Use it to take note of (discarded) traitors, the top spice card and treachery cards.<br>When a (possibly unknown) card is won, click on an empty card placeholder and select it.<br>To indicate that a card was discarded, click it and then select the 'trash can' which will move the card to the Discarded section. When an unknown card becomes known and discarded, select it first, then discard it.<br>When the discard pile is shuffled into a new draw pile, click 'reshuffle' to clear the Discarded section.<br>If you sent a card to the Discarded section by mistake, click on the X next to it.`;
}
