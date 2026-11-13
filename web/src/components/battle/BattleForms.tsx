// The battle area's action forms: one React component per event type, each named after the
// C# component it replaced. Options and
// validation come from rust/view/src/actions/battle.rs; every form starts from the server's
// `draft` and sends its next draft from it.
import { useEffect, useRef } from "react";
import type { ActionView } from "../../generated/ActionView";
import type { BattleFormParam } from "../../generated/BattleFormParam";
import type { Faction, StrongholdAdvantage } from "../../generated/enums";
import { strongholdAdvantagePopup, techTokenPopup, treacheryCardPopup } from "../../popups";
import { useGame } from "../../state/context";
import { ButtonRow } from "../ButtonRow";
import { Checkbox } from "../Checkbox";
import { FactionChip } from "../FactionChip";
import { ForceToken } from "../ForceToken";
import { Image } from "../Image";
import { PlaceChip } from "../PlaceChip";
import { RichText } from "../RichText";
import { SelectFromImage } from "../SelectFromImage";
import { SimpleNumber } from "../SimpleNumber";
import { SpiceComponent } from "../SpiceComponent";
import { BattleComponent } from "./BattleComponent";
import { useMapPicker } from "../board/mapInteraction";
import { cardChoices, heroChoices, idOrNull, Layout, MapForceStepper, PinActions, Radio, SelectForces, SubmitError, useBattleDraft, useTerritoryOf } from "./parts";
import { Select } from "../Select";

type Props = { action: ActionView; inPopover?: boolean };

// ActionPanel.razor's switch for the battle forms; null for other event types.
export function battleForm(a: ActionView, inPopover = false) {
  const p = { key: a.eventType, action: a, inPopover };
  switch (a.options?.form) {
    case "AuditCancelled": return <AuditCancelledComponent {...p} />;
    case "Audited": return <AuditedComponent {...p} />;
    case "Battle": return <BattleComponent {...p} />;
    case "BattleConcluded": return <BattleConcludedComponent {...p} />;
    case "BattleInitiated": return <BattleInitiatedComponent {...p} />;
    case "BeneGesseritBattleAnnouncement": return <BeneGesseritBattleAnnouncementComponent {...p} />;
    case "CardsDetermined": return <CardsDeterminedComponent {...p} />;
    case "ClairVoyanceAnswered": return <ClairVoyanceAnsweredComponent {...p} />;
    case "ClairVoyancePlayed": return <ClairVoyancePlayedComponent {...p} />;
    case "Diplomacy": return <DiplomacyComponent {...p} />;
    case "FaceDanced": return <FaceDancedComponent {...p} />;
    case "FaceDancerReplaced": return <FaceDancerReplacedComponent {...p} />;
    case "FaceDancerRevealed": return <FaceDancerRevealedComponent {...p} />;
    case "HMSAdvantageChosen": return <HMSAdvantageChosenComponent {...p} />;
    case "JuicePlayed": return <JuicePlayedComponent {...p} />;
    case "LoserConcluded": return <LoserConcludedComponent {...p} />;
    case "Planetology": return <PlanetologyComponent {...p} />;
    case "Prescience": return <PrescienceComponent {...p} />;
    case "Retreat": return <RetreatComponent {...p} />;
    case "RockWasMelted": return <RockWasMeltedComponent {...p} />;
    case "TakeLosses": return <TakeLossesComponent {...p} />;
    case "ThoughtAnswered": return <ThoughtAnsweredComponent {...p} />;
    case "Thought": return <ThoughtComponent {...p} />;
    case "TreacheryCalled": return <TreacheryCalledComponent {...p} />;
    case "Voice": return <VoiceComponent {...p} />;
    default: return null;
  }
}

// Skin.Describe of a treachery card type.
function cardTypeName(skin: ReturnType<typeof useGame>["skin"], type: string): string {
  return skin.enumValue("TreacheryCardType", type);
}

function AuditCancelledComponent({ action, inPopover }: Props) {
  const { options: o, validation, submit, error } = useBattleDraft(action, "AuditCancelled");
  if (!o) return null;
  return (
    <Layout type={action.eventType} urgent inPopover={inPopover}
            header={<>Do you wish to pay <FactionChip faction="Choam" /> <SpiceComponent amount={String(o.cost)} /> to cancel a scheduled audit of {o.cards} of your treachery cards?</>}>
      <ButtonRow confirm={() => submit({ Cancelled: true })} confirmText="Yes" confirmError={validation} pass={() => submit({ Cancelled: false })} passText="No" />
      <SubmitError error={error} />
    </Layout>
  );
}

function AuditedComponent({ action, inPopover }: Props) {
  const { skin } = useGame();
  const { options: o, validation, submit, error } = useBattleDraft(action, "Audited");
  if (!o) return null;
  return (
    <Layout type={action.eventType} urgent inPopover={inPopover} header="Perform your audit">
      <div>
        {o.cards.map((c, i) => <Image key={i} src={skin.cardImage(c)} alt={skin.card(c)} width={100} className="p-1" popover={treacheryCardPopup(skin, c)} />)}
      </div>
      <ButtonRow confirm={() => submit()} confirmText="Finish Audit" confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

function BattleInitiatedComponent({ action, inPopover }: Props) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, autoSubmit, error } = useBattleDraft(action, "BattleInitiated");
  // With one battle to fight, the C# form confirms it after half a second.
  const confirmed = useRef<string | null>(null);
  const single = o?.fights.length === 1;
  useEffect(() => {
    if (!single || confirmed.current === JSON.stringify(draft)) return;
    confirmed.current = JSON.stringify(draft);
    const t = setTimeout(() => { void autoSubmit(); }, 500);
    return () => clearTimeout(t);
  });
  if (!o) return null;
  const opponents = [...new Map(o.fights.map((f) => [f.faction.value, f.faction])).values()];
  const target = draft.Target as number;
  const territories = o.fights.filter((f) => f.faction.value === target);
  return (
    <Layout type={action.eventType} urgent inPopover={inPopover} header="Select whom and where to battle">
      <SelectFromImage values={opponents.map((f) => ({ value: f.value, src: skin.factionImage(f.name as Faction), alt: f.label, popover: null, tooltip: f.label }))}
                       value={target} required
                       onChange={(v) => {
                         const first = o.fights.find((f) => f.faction.value === v);
                         if (v !== null) void update({ Target: v, _territoryId: first?.territory ?? -1 });
                       }} />
      <Select options={territories.map((f) => ({ value: f.territory, label: f.territoryLabel }))} value={draft._territoryId as number}
              onChange={(v) => void update({ _territoryId: v })} />
      <ButtonRow confirm={() => submit()} confirmText="Initiate Battle" confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

function BeneGesseritBattleAnnouncementComponent({ action, inPopover }: Props) {
  const { view } = useGame();
  const territoryOf = useTerritoryOf();
  const { options: o, draft, validation, update, submit, error } = useBattleDraft(action, "BeneGesseritBattleAnnouncement");
  // MapPickComponent: the sectors of the valid territories are outlined; the pin confirms.
  const pick = useMapPicker({
    targets: () => o?.locations ?? [],
    byTerritory: true,
    territoryOf,
    faction: (view.viewer.faction ?? "BeneGesserit") as Faction,
    pin: () => <div className="pin-body"><PinActions label="Flip to fighters" validation={validation} cancel={() => pick.clear()} confirm={() => void submit()} /></div>,
  });
  useEffect(() => {
    const t = pick.target != null ? territoryOf(pick.target) : -1;
    if (o && draft._territoryId !== t) void update({ _territoryId: t });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pick.target]);
  if (!o) return null;
  return (
    <Layout type={action.eventType} inPopover={inPopover} header="Flip advisors to fighters">
      <div className="small">Use the map to choose a territory.</div>
      <SubmitError error={error} />
    </Layout>
  );
}

function CardsDeterminedComponent({ action, inPopover }: Props) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useBattleDraft(action, "CardsDetermined");
  if (!o) return null;
  const ids = (s: unknown) => (typeof s === "string" && s !== "" ? s.split(";").map(Number) : []);
  const treachery = ids(draft._treacheryCardIds);
  const richese = ids(draft._richeseCardIds);
  // A multi-select row: clicking swaps a card in or out; shift-click keeps only that card.
  const row = (cards: number[], selected: number[], key: string, width: number) => (
    <div className="img-choices">
      {cards.map((c) => (
        <span key={c} onClick={(e) => {
          const next = e.shiftKey ? [c] : selected.includes(c) ? selected.filter((x) => x !== c) : [...selected, c];
          void update({ [key]: next.join(";") });
        }}>
          <Image src={skin.cardImage(c)} alt={skin.card(c)} width={width} pointer highlightOnHover darkened={!selected.includes(c)}
                 popover={treacheryCardPopup(skin, c)} />
        </span>
      ))}
    </div>
  );
  return (
    <Layout type={action.eventType} urgent inPopover={inPopover} header="Select cards in play">
      <h4>Treachery Cards</h4>
      {row(o.treacheryCards, treachery, "_treacheryCardIds", 80)}
      {o.richeseCards.length > 0 && (
        <>
          <h4><FactionChip faction="Richese" /> Cards</h4>
          {row(o.richeseCards, richese, "_richeseCardIds", 60)}
        </>
      )}
      <ButtonRow confirm={() => submit()} confirmText="Confirm Cards" confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

function ClairVoyanceAnsweredComponent({ action, inPopover }: Props) {
  const { skin } = useGame();
  const { options: o, validation, submit, error } = useBattleDraft(action, "ClairVoyanceAnswered");
  if (!o) return null;
  // ClairVoyanceAnswer: Unknown 0, Yes 1, No 2.
  return (
    <Layout type={action.eventType} urgent inPopover={inPopover}
            header={<><FactionChip faction={o.questioner} /> questions you with {cardTypeName(skin, "Clairvoyance")}!</>}>
      <div className="font-italic bg-dark"><RichText parts={o.question} /></div>
      <ButtonRow confirm={() => submit({ Answer: 1 })} confirmText="Yes" confirmError={validation}
                 pass={() => submit({ Answer: 2 })} passText="No" other={() => submit({ Answer: 0 })} otherText="I can't answer that..." />
      <SubmitError error={error} />
    </Layout>
  );
}

const cardTypeQuestions = ["CardTypeInBattle", "CardTypeAsWeaponInBattle", "CardTypeAsDefenseInBattle", "HasCardTypeInHand"];
const leaderQuestions = ["LeaderAsFacedancer", "LeaderAsTraitor", "LeaderInBattle"];

// The dials of DialOfMoreThanXInBattle: 0 to 25 in steps of a half.
const dials: BattleFormParam[] = Array.from({ length: 51 }, (_, i) => ({ value: String(i / 2), label: String(i / 2) }));

function ClairVoyancePlayedComponent({ action, inPopover }: Props) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useBattleDraft(action, "ClairVoyancePlayed");
  if (!o) return null;
  const question = o.questions.find((q) => q.value === draft.Question)?.name ?? "";
  // Each select takes its first value when the current one is not among its values.
  const param = (values: BattleFormParam[], key: string, label: string) => (
    <Select options={values.map((v) => ({ value: v.value, label: v.label }))} value={(draft[key] as string | null) ?? null}
            onChange={(v) => void update({ [key]: v })} label={label} />
  );
  const paramsFor = (q: string) =>
    cardTypeQuestions.includes(q) ? [o.cardTypes] : leaderQuestions.includes(q) ? [o.leaders] : q === "Prediction" ? [o.factions, o.turns]
      : q === "DialOfMoreThanXInBattle" ? [dials] : q === "WillAttackX" ? [o.territories] : [];
  const firstParams = (q: string, current: Record<string, unknown>) => {
    const lists = paramsFor(q);
    const keep = (l: BattleFormParam[] | undefined, v: unknown) => (l === undefined ? null : l.some((p) => p.value === v) ? v : l[0]?.value ?? null);
    return { QuestionParameter1: keep(lists[0], current.QuestionParameter1), QuestionParameter2: keep(lists[1], current.QuestionParameter2) };
  };
  return (
    <Layout type={action.eventType} inPopover={inPopover} header={`Use ${cardTypeName(skin, "Clairvoyance")}?`}>
      <SelectFromImage label="Target Faction" imageWidth={40} required value={draft.Target as number}
                       values={o.targets.map((f) => ({ value: f.value, src: skin.factionImage(f.name as Faction), alt: f.label, popover: null, tooltip: f.label }))}
                       onChange={(v) => v !== null && v !== draft.Target && void update({ Target: v })} />
      <Select options={o.questions.map((q) => ({ value: q.value, label: q.label }))} value={draft.Question as number} label="Question"
              onChange={(v) => {
                const name = o.questions.find((q) => q.value === v)?.name ?? "";
                void update({ Question: v, ...firstParams(name, draft) });
              }} />
      {cardTypeQuestions.includes(question) && param(o.cardTypes, "QuestionParameter1", "Card Type")}
      {leaderQuestions.includes(question) && param(o.leaders, "QuestionParameter1", "Leader")}
      {question === "Prediction" && <>{param(o.factions, "QuestionParameter1", "Faction")}{param(o.turns, "QuestionParameter2", "Turn")}</>}
      {question === "DialOfMoreThanXInBattle" && param(dials, "QuestionParameter1", "Dial")}
      {question === "WillAttackX" && param(o.territories, "QuestionParameter1", "Territory")}
      <ButtonRow confirm={() => submit()} confirmText={`Use ${cardTypeName(skin, "Clairvoyance")}`} confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// Diplomacy, Thought and ThoughtAnswered: pick one card.
function CardForm({ action, inPopover, form, header, confirmText }: Props & {
  form: "Diplomacy" | "Thought" | "ThoughtAnswered";
  header: React.ReactNode;
  confirmText: (card: number | null) => string;
}) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useBattleDraft(action, form);
  if (!o) return null;
  const card = idOrNull(draft._cardId);
  return (
    <Layout type={action.eventType} inPopover={inPopover} header={header}>
      <SelectFromImage values={cardChoices(skin, o.cards)} value={card} required imageWidth={80}
                       onChange={(v) => v !== null && v !== card && void update({ _cardId: v })} />
      <ButtonRow confirm={() => submit()} confirmText={confirmText(card)} confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

function DiplomacyComponent(p: Props) {
  return <CardForm {...p} form="Diplomacy" header="Use Diplomacy?" confirmText={() => "Use Diplomacy"} />;
}

function ThoughtComponent(p: Props) {
  const { skin } = useGame();
  return <CardForm {...p} form="Thought" header={<>Use your <span>{skin.enumValue("LeaderSkill", "Thinker")}</span> to ask if your opponent has a specific weapon</>}
                   confirmText={(c) => `Ask about ${c === null ? "" : skin.card(c)}`} />;
}

function ThoughtAnsweredComponent(p: Props) {
  const { skin } = useGame();
  return <CardForm {...p} form="ThoughtAnswered"
                   header={<>Select which card (if any) to show in response to your opponent's <span>{skin.enumValue("LeaderSkill", "Thinker")}</span> question</>}
                   confirmText={(c) => `Show ${c === null ? "" : skin.card(c)}`} />;
}

// One location's force pickers in ForceSelectionComponent: "locId:forces|special" entries.
type Placement = Map<number, [number, number]>;

function parsePlacement(s: unknown): Placement {
  const m: Placement = new Map();
  if (typeof s !== "string" || s === "") return m;
  for (const pair of s.split(",")) {
    const [l, amounts] = pair.split(":");
    const [f, sf] = (amounts ?? "0|0").split("|");
    m.set(Number(l), [Number(f), Number(sf)]);
  }
  return m;
}

function placementString(m: Placement): string {
  return [...m.entries()].map(([l, [f, s]]) => `${l}:${f}|${s}`).join(",");
}

// ForceSelectionComponent: its pickers step the placement (OnUp/OnDown); a typed number changes
// only what the picker shows, as in C#.
function ForceSelection({ faction, locations, value, onChange, fixedMax = -1 }: {
  faction: Faction;
  locations: { location: number; forces: number; specialForces: number }[];
  value: Placement;
  onChange: (m: Placement) => void;
  fixedMax?: number;
}) {
  const set = (l: number, special: boolean, v: number) => {
    const m = new Map(value);
    const [f, s] = m.get(l) ?? [0, 0];
    m.set(l, special ? [f, v] : [v, s]);
    onChange(m);
  };
  return (
    <table className="table table-borderless table-sm m-0 w-auto">
      <tbody>
        {locations.map((l) => {
          const max = fixedMax > 0 ? fixedMax : l.forces;
          const [f, s] = value.get(l.location) ?? [0, 0];
          return (
            <tr key={l.location} className="small bg-transparent">
              <td className="ps-0 bg-transparent align-middle"><PlaceChip place={{ location: l.location }} /></td>
              {max > 0 && <td className="ps-0 bg-transparent"><SelectForces value={f} onChange={(v) => set(l.location, false, v)} onStep={(d) => set(l.location, false, f + d)} min={0} max={max} faction={faction} special={false} /></td>}
              {l.specialForces > 0 && <td className="ps-0 bg-transparent"><SelectForces value={s} onChange={(v) => set(l.location, true, v)} onStep={(d) => set(l.location, true, s + d)} min={0} max={l.specialForces} faction={faction} special /></td>}
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

function FaceDancedComponent({ action, inPopover }: Props) {
  const { view } = useGame();
  const { options: o, draft, validation, update, submit, error } = useBattleDraft(action, "FaceDanced");
  if (!o) return null;
  const faction = (view.viewer.faction ?? "Tleilaxu") as Faction;
  const sources = parsePlacement(draft._forceLocations);
  const targets = parsePlacement(draft._targetForceLocations);
  const pickers = (
    <>
      <div>
        <SelectForces value={draft.ForcesFromReserve as number} onChange={(v) => void update({ ForcesFromReserve: v })} min={0} max={o.forcesInReserve}
                      faction={faction} special={false} label="From reserves:" />
      </div>
      {o.sources.length > 0 && (
        <>
          <div className="mt-2 mb-0">From somewhere on the planet:</div>
          <ForceSelection faction={faction} locations={o.sources} value={sources} onChange={(m) => void update({ _forceLocations: placementString(m) })} />
        </>
      )}
      <div className="mt-2 mb-0">Select where to put your replacements:</div>
      <ForceSelection faction={faction} locations={o.targets} value={targets} fixedMax={o.maximumForces}
                      onChange={(m) => void update({ _targetForceLocations: placementString(m) })} />
    </>
  );
  return (
    <Layout type={action.eventType} urgent inPopover={inPopover} header="Choose which forces to replace by your own">
      <div className="mb-2">You may place up to <strong>{o.maximumForces}</strong> <ForceToken faction={faction} size={20} /> in <PlaceChip place={{ territory: o.territory }} /> from your reserves and/or from anywhere on the planet.</div>
      {o.maximumForces > 0 && pickers}
      <ButtonRow confirm={() => submit()} confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

function FaceDancerReplacedComponent({ action, inPopover }: Props) {
  const { skin, view } = useGame();
  const { options: o, draft, validation, update, submit, error } = useBattleDraft(action, "FaceDancerReplaced");
  if (!o) return null;
  const dancer = idOrNull(draft.dancerId);
  const label = o.heroes.find((h) => h.id === dancer)?.label;
  return (
    <Layout type={action.eventType} urgent inPopover={inPopover} header="Replace a Face Dancer?">
      <SelectFromImage values={heroChoices(skin, view, o.heroes)} value={dancer} required imageWidth={80}
                       onChange={(v) => v !== null && v !== dancer && void update({ dancerId: v })} />
      <ButtonRow confirm={() => submit({ Passed: false })} confirmText={label ? `Replace ${label}` : "Replace"} confirmError={validation}
                 pass={() => submit({ Passed: true })} passText="Pass" />
      <SubmitError error={error} />
    </Layout>
  );
}

function FaceDancerRevealedComponent({ action, inPopover }: Props) {
  const { options: o, validation, submit, error } = useBattleDraft(action, "FaceDancerRevealed");
  if (!o) return null;
  return (
    <Layout type={action.eventType} urgent inPopover={inPopover} header="Reveal a Face Dancer?">
      {o.mayCall
        ? <ButtonRow confirm={() => submit({ Passed: false })} confirmText="Yes" confirmError={validation} pass={() => submit({ Passed: true })} passText="No" />
        : <ButtonRow pass={() => submit({ Passed: true })} passText="No" />}
      <SubmitError error={error} />
    </Layout>
  );
}

function HMSAdvantageChosenComponent({ action, inPopover }: Props) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useBattleDraft(action, "HMSAdvantageChosen");
  if (!o) return null;
  const chosen = o.choices.find((c) => c.value === draft.Advantage);
  return (
    <Layout type={action.eventType} inPopover={inPopover}
            header={<>Select which advantage of another stronghold you control to use for the <PlaceChip place={{ location: skin.statics.map.hiddenMobileStronghold }} /></>}>
      <SelectFromImage values={o.choices.map((c) => ({ value: c.value, src: skin.url(skin.get("StrongholdCardImage_URL", c.name)), alt: c.label, popover: strongholdAdvantagePopup(skin, c.name as StrongholdAdvantage) }))}
                       value={draft.Advantage as number} required imageWidth={100}
                       onChange={(v) => v !== null && v !== draft.Advantage && void update({ Advantage: v })} />
      <ButtonRow confirm={() => submit()} confirmText={`Select ${chosen?.label ?? ""}`} confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

function JuicePlayedComponent({ action, inPopover }: Props) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useBattleDraft(action, "JuicePlayed");
  if (!o) return null;
  return (
    <Layout type={action.eventType} inPopover={inPopover} header={`Use ${cardTypeName(skin, "Juice")}?`}>
      <Radio values={o.choices.map((c) => c.value)} labels={o.choices.map((c) => c.label)} value={draft.Type as number} onChange={(v) => void update({ Type: v })} />
      <ButtonRow confirm={() => submit()} confirmText={`Use ${cardTypeName(skin, "Juice")}`} confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// LoserConcluded.KARMA_NO, KARMA_KEEP, KARMA_DISCARD.
const karmaDecisions = [0, 1, 2];

function LoserConcludedComponent({ action, inPopover }: Props) {
  const { skin, view } = useGame();
  const { options: o, draft, validation, update, submit, error } = useBattleDraft(action, "LoserConcluded");
  if (!o) return null;
  const kept = idOrNull(draft._keptCardId);
  const forced = typeof draft._forcedKeptOrDiscardedCardIds === "string" && draft._forcedKeptOrDiscardedCardIds !== ""
    ? draft._forcedKeptOrDiscardedCardIds.split(";").map(Number) : [];
  const decision = draft.KarmaForcedKeptCardDecision as number;
  return (
    <Layout type={action.eventType} inPopover={inPopover} header="Please conclude the battle you lost">
      {o.cardsToKeep.length > 0 && (
        <SelectFromImage label="You may select one card to keep:" values={cardChoices(skin, o.cardsToKeep)} value={kept} required={false} imageWidth={80}
                         onChange={(v) => void update({ _keptCardId: v ?? -1 })} />
      )}
      {o.assassination && (
        <Radio values={[true, false]} labels={["Yes", "No"]} value={draft.Assassinate as boolean} onChange={(v) => void update({ Assassinate: v })}
               label={(
                 <>
                   <div>Execute an assassination mission by revealing <Image src={skin.personImage(o.assassination.target)} alt={skin.person(o.assassination.target)}
                     width={50} className="img-float" popover={heroChoices(skin, view, [{ id: o.assassination.target, hero: { t: "leader", id: o.assassination.target }, label: "" }])[0].popover} /> as your traitor?</div>
                   {o.assassination.reward === null
                     ? <div>You will not collect {skin.concept("Resource")} as this leader is in the tanks.</div>
                     : <div>You will collect <SpiceComponent amount={String(o.assassination.reward)} />.</div>}
                 </>
               )} />
      )}
      {o.mayUseKarma && (
        <>
          <Radio values={karmaDecisions} labels={["No", "Force keeping...", "Force discarding..."]} value={decision}
                 onChange={(v) => void update({ KarmaForcedKeptCardDecision: v, ...(v === 0 ? { _forcedKeptOrDiscardedCardIds: "" } : {}) })}
                 label={<>Use {cardTypeName(skin, "Karma")} to force your opponent to discard or keep any or all cards used in battle?</>} />
          {decision !== 0 && (
            <div className="img-choices">
              {o.cardsKeptOrDiscarded.map((c) => (
                <span key={c} onClick={(e) => {
                  const next = e.shiftKey ? [c] : forced.includes(c) ? forced.filter((x) => x !== c) : [...forced, c];
                  void update({ _forcedKeptOrDiscardedCardIds: next.join(";") });
                }}>
                  <Image src={skin.cardImage(c)} alt={skin.card(c)} width={80} pointer highlightOnHover darkened={!forced.includes(c)} popover={treacheryCardPopup(skin, c)} />
                </span>
              ))}
            </div>
          )}
        </>
      )}
      <ButtonRow confirm={() => submit()} confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

function PlanetologyComponent({ action, inPopover }: Props) {
  const { skin } = useGame();
  const { options: o, validation, submit, error } = useBattleDraft(action, "Planetology");
  if (!o) return null;
  return (
    <Layout type={action.eventType} inPopover={inPopover}
            header={<>Use <span>{skin.enumValue("LeaderSkill", "Planetologist")}</span> to move 1 extra territory or move two force groups to the same territory?</>}>
      <ButtonRow confirm={() => submit({ AddOneToMovement: true })} confirmText="1 extra territory" confirmError={validation}
                 other={() => submit({ AddOneToMovement: false })} otherText="2 force groups" />
      <SubmitError error={error} />
    </Layout>
  );
}

function PrescienceComponent({ action, inPopover }: Props) {
  const { options: o, draft, validation, update, submit, error } = useBattleDraft(action, "Prescience");
  if (!o) return null;
  return (
    <Layout type={action.eventType} inPopover={inPopover} header="Use Prescience">
      {o.voiceNote && <div className="text-info small mb-2">Note: Voice should be used before Prescience.</div>}
      <Radio values={o.aspects.map((a) => a.value)} labels={o.aspects.map((a) => a.label)} value={draft.Aspect as number}
             onChange={(v) => void update({ Aspect: v })} label="What should be foreseen?" />
      <ButtonRow confirm={() => submit()} confirmText="Use Prescience" confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

function RetreatComponent({ action, inPopover }: Props) {
  const { view } = useGame();
  const territoryOf = useTerritoryOf();
  const { options: o, draft, validation, update, submit, error } = useBattleDraft(action, "Retreat");
  const faction = (view.viewer.faction ?? "None") as Faction;
  // MapPickComponent: the location is picked on the map; the pin holds the force steppers.
  const pick = useMapPicker({
    targets: () => o?.targets ?? [],
    territoryOf,
    faction,
    pin: () => o && (
      <div className="pin-body">
        <div className="pin-row">
          {o.maxForces > 0 && <MapForceStepper value={draft.Forces as number} onChange={(v) => void update({ Forces: v })} min={0} max={o.maxForces} faction={faction} special={false} />}
          {o.maxSpecialForces > 0 && <MapForceStepper value={draft.SpecialForces as number} onChange={(v) => void update({ SpecialForces: v })} min={0} max={o.maxSpecialForces} faction={faction} special />}
        </div>
        <PinActions label="Retreat" validation={validation} cancel={() => pick.clear()} confirm={() => void submit()} />
      </div>
    ),
  });
  useEffect(() => {
    const l = pick.target ?? -1;
    if (o && draft._targetId !== l) void update({ _targetId: l });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pick.target]);
  if (!o) return null;
  // Passing still names a location, because the event insists on a valid one.
  const target = pick.target ?? o.targets[0] ?? -1;
  return (
    <Layout type={action.eventType} urgent inPopover={inPopover} header="You may retreat from the battle any number of undialed forces to an adjacent empty non-stronghold territory">
      <div className="small">Use the map to choose a location.</div>
      <ButtonRow pass={() => submit({ _targetId: target, Forces: 0, SpecialForces: 0 })} passText="Pass" />
      <SubmitError error={error} />
    </Layout>
  );
}

function RockWasMeltedComponent({ action, inPopover }: Props) {
  const { skin } = useGame();
  const { options: o, validation, submit, error } = useBattleDraft(action, "RockWasMelted");
  if (!o) return null;
  return (
    <Layout type={action.eventType} urgent inPopover={inPopover} header={`Use ${cardTypeName(skin, "Rockmelter")} to kill or to reduce leader strength to zero?`}>
      <ButtonRow confirm={() => submit({ Kill: true })} confirmText="Kill" confirmError={validation} other={() => submit({ Kill: false })} otherText="Reduce to 0" />
      <SubmitError error={error} />
    </Layout>
  );
}

function TakeLossesComponent({ action, inPopover }: Props) {
  const { view } = useGame();
  const { options: o, draft, validation, update, submit, error } = useBattleDraft(action, "TakeLosses");
  if (!o) return null;
  const faction = (view.viewer.faction ?? "None") as Faction;
  const forces = o.maxForces > 0 ? (draft.ForceAmount as number) : 0;
  const specials = o.maxSpecialForces > 0 ? (draft.SpecialForceAmount as number) : 0;
  return (
    <Layout type={action.eventType} urgent inPopover={inPopover}
            header={<>Please choose how to distribute a storm loss total of at least <strong>{o.amount}</strong> over your forces in {o.location !== null && <PlaceChip place={{ location: o.location }} />}</>}>
      {o.maxForces > 0 && <SelectForces value={forces} onChange={(v) => void update({ ForceAmount: v })} min={0} max={o.maxForces} faction={faction} special={false} />}
      {o.maxSpecialForces > 0 && <SelectForces value={specials} onChange={(v) => void update({ SpecialForceAmount: v })} min={0} max={o.maxSpecialForces} faction={faction} special />}
      <p>Total losses: <SimpleNumber>{String(forces + specials)}</SimpleNumber></p>
      {o.uselessLabel !== null
        ? <ButtonRow confirm={() => submit({ UseUselessCard: false })} confirmText="Confirm Losses" confirmError={validation} other={() => submit({ UseUselessCard: true })} otherText={o.uselessLabel} />
        : <ButtonRow confirm={() => submit({ UseUselessCard: false })} confirmText="Confirm Losses" confirmError={validation} />}
      <SubmitError error={error} />
    </Layout>
  );
}

function TreacheryCalledComponent({ action, inPopover }: Props) {
  const { options: o, validation, submit, autoSubmit, error } = useBattleDraft(action, "TreacheryCalled");
  // A player who cannot call treachery has nothing to decide, so the form passes for them
  // without showing itself, once per battle.
  const passedFor = useRef<string | null>(null);
  const { view } = useGame();
  const battleKey = `${view.battle?.aggressor}-${view.battle?.defender}-${view.battle?.territory}-${view.turn}`;
  useEffect(() => {
    if (o && !o.mayCall && view.battle && passedFor.current !== battleKey) {
      passedFor.current = battleKey;
      void autoSubmit({ TraitorCalled: false });
    }
  });
  if (!o || !o.mayCall) return null;
  return (
    <Layout type={action.eventType} urgent inPopover={inPopover} header="Call TREACHERY?">
      <ButtonRow confirm={() => submit({ TraitorCalled: true })} confirmText="TREACHERY!" confirmError={validation} pass={() => submit({ TraitorCalled: false })} passText="Pass" />
      <SubmitError error={error} />
    </Layout>
  );
}

function VoiceComponent({ action, inPopover }: Props) {
  const { options: o, draft, validation, update, submit, error } = useBattleDraft(action, "Voice");
  if (!o) return null;
  return (
    <Layout type={action.eventType} inPopover={inPopover} header="Use Voice">
      {o.prescienceNote && <div className="text-info small mb-2">Note: Voice should be used before Prescience.</div>}
      <Radio values={[true, false]} labels={["Force the use of...", "Deny the use of..."]} value={draft.Must as boolean} onChange={(v) => void update({ Must: v })} />
      <Select options={o.types.map((t) => ({ value: t.value, label: t.label }))} value={draft.Type as number} onChange={(v) => void update({ Type: v })} />
      <ButtonRow confirm={() => submit()} confirmText="Use Voice" confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

function BattleConcludedComponent({ action, inPopover }: Props) {
  const { skin, view } = useGame();
  const { options: o, draft, validation, update, submit, error } = useBattleDraft(action, "BattleConcluded");
  if (!o) return null;
  const discarded = typeof draft._cardIds === "string" && draft._cardIds !== "" ? draft._cardIds.split(";").map(Number) : [];
  // DetermineSelectedCards keeps the order mercenary, weapon, defense.
  const toggle = (card: number, on: boolean) => {
    const order = [o.discardableMercenary, o.discardableWeapon, o.discardableDefense].filter((c): c is number => c !== null);
    const set = new Set(discarded);
    if (on) set.add(card); else set.delete(card);
    void update({ _cardIds: order.filter((c) => set.has(c)).join(";") });
  };
  const traitorToReplace = idOrNull(draft._traitorToReplaceId);
  const replaceLabel = o.traitorsToReplace?.find((h) => h.id === traitorToReplace)?.label;
  // CaptureDecision is stored as Capture and Kill: DontCapture 0, Capture 1, Kill 2 (the C# enum).
  const decision = !draft.Capture ? valueOf(o.capture?.decisions, "DontCapture") : draft.Kill ? valueOf(o.capture?.decisions, "Kill") : valueOf(o.capture?.decisions, "Capture");
  const discardable = [o.discardableMercenary, o.discardableWeapon, o.discardableDefense].filter((c): c is number => c !== null);
  const loserTokens = o.techTokens;
  return (
    <Layout type={action.eventType} urgent inPopover={inPopover} header="Conclude the battle you just won">
      {o.winner && (
        <>
          {o.maxReplacementForces > 0 && (
            <>
              <label>How many <ForceToken faction={o.forceSupplier} special size={20} /> losses should be exchanged with <ForceToken faction={o.forceSupplier} size={20} />?</label>
              <SelectForces value={draft.SpecialForceLossesReplaced as number} onChange={(v) => void update({ SpecialForceLossesReplaced: v })} min={0}
                            max={o.maxReplacementForces} faction={o.forceSupplier} special />
            </>
          )}
          {discardable.length > 0 && (
            <>
              <div className="mt-2">Select any cards you wish to discard:</div>
              {discardable.map((c) => (
                <Checkbox key={c} value={discarded.includes(c)} onChange={(v) => toggle(c, v)}>
                  <RichText parts={[{ t: "treacheryCard", id: c, label: skin.card(c) }]} />
                </Checkbox>
              ))}
            </>
          )}
          {o.capture && (
            <>
              <div>Our troops captured <Image src={skin.personImage(o.capture.victim)} alt={skin.person(o.capture.victim)} width={50} className="img-float"
                popover={heroChoices(skin, view, [{ id: o.capture.victim, hero: { t: "leader", id: o.capture.victim }, label: "" }])[0].popover} /></div>
              <Radio values={o.capture.decisions.map((d) => d.value)} labels={o.capture.decisions.map((d) => d.label)} value={decision}
                     label="What are your orders, keep or kill?"
                     onChange={(v) => {
                       const name = o.capture?.decisions.find((d) => d.value === v)?.name;
                       void update({ Capture: name !== "DontCapture", Kill: name === "Kill" });
                     }} />
            </>
          )}
          {loserTokens.length > 0 && (
            <SelectFromImage label={loserTokens.length === 1 ? <span>You will steal the following tech token:</span> : <span>Choose a tech token to steal:</span>}
                             values={loserTokens.map((t) => ({ value: t.value, src: skin.url(skin.get("TechTokenImage_URL", t.name)), alt: t.label, popover: techTokenPopup(skin, t.name as never) }))}
                             value={draft.StolenToken as number} required onChange={(v) => v !== null && v !== draft.StolenToken && void update({ StolenToken: v })} />
          )}
          {o.deciphererTraitors.length > 0 && (
            <>
              <SelectFromImage label={<><span>{skin.enumValue("LeaderSkill", "Decipherer")}</span> shows you the following leaders from the traitor deck:</>}
                               values={heroChoices(skin, view, o.deciphererTraitors)} value={idOrNull(draft._newTraitorId)} required={false} imageWidth={80}
                               onChange={(v) => void update({ _newTraitorId: v ?? -1 })} />
              {o.traitorsToReplace && (
                <SelectFromImage label={`If you wish, you may select one of the above leaders to reveal and replace one of your current ${view.viewer.faction === "Tleilaxu" ? "facedancers" : "traitors"}:`}
                                 values={heroChoices(skin, view, o.traitorsToReplace)} value={traitorToReplace} required={false} imageWidth={80}
                                 onChange={(v) => void update({ _traitorToReplaceId: v ?? -1 })} />
              )}
            </>
          )}
          {o.addExtraForceIn !== null && (
            <Checkbox value={draft.AddExtraForce as boolean} onChange={(v) => void update({ AddExtraForce: v })}>
              Do you wish to add one force from reserves in <PlaceChip place={{ territory: o.addExtraForceIn }} />?
            </Checkbox>
          )}
        </>
      )}
      <ButtonRow confirm={() => submit()} confirmText={replaceLabel ? `Conclude Battle and Reveal & Replace ${replaceLabel}` : "Conclude Battle"} confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

function valueOf(list: { value: number; name: string }[] | undefined, name: string): number {
  return list?.find((d) => d.name === name)?.value ?? -1;
}
