// The "other events" forms that pick cards, factions, heroes or a number (package 7e): one
// component per C# GameEventComponents/<Event>Component.razor, with its layout and labels.
// Options and the adjusted draft come from rust/view/src/actions/other.rs.
import { useRef, useState } from "react";
import type { ActionView } from "../../generated/ActionView";
import { FactionValue } from "../../generated/enums";
import type { Part } from "../../generated/Part";
import type { GameEventDraft } from "../../state/source";
import { useGame } from "../../state/context";
import { ButtonRow } from "../ButtonRow";
import { FactionChip } from "../FactionChip";
import { Hoverable } from "../Hoverable";
import { NumberPicker, SelectResources } from "../NumberPicker";
import { PlaceChip } from "../PlaceChip";
import { heroChoices, heroName } from "../revival/parts";
import { RichText } from "../RichText";
import { useViewChanges } from "../formDraft";
import { SelectFromImage } from "../SelectFromImage";
import { SpiceComponent } from "../SpiceComponent";
import {
  cardChoices, cardType, factionChoices, fmt, idList, idString, Layout, num, Radio, SelectManyFromImage, SelectNumber,
  SubmitError, useOtherDraft,
} from "./parts";
import { Select } from "../Select";

export interface FormProps {
  action: ActionView;
  inPopover?: boolean;
}

const cardName = (skin: ReturnType<typeof useGame>["skin"], id: number) => (id >= 0 ? skin.card(id) : "");

// A form whose only input is one card: Discarded, EmperorDiscarded, ChoamDiscarded, DiscardedSearched,
// DiscardedTaken, IxianSelectedStartingCard, KarmaRicheseBuy, RicheseGaveCard.
function CardForm({ action, inPopover = false, form, header, urgent = false, confirm }: FormProps & {
  form: "Discarded" | "EmperorDiscarded" | "ChoamDiscarded" | "DiscardedSearched" | "DiscardedTaken" | "IxianSelectedStartingCard" | "KarmaRicheseBuy" | "RicheseGaveCard";
  header: React.ReactNode;
  urgent?: boolean;
  confirm: (card: string) => string;
}) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, form);
  if (!o) return null;
  const card = num(draft._cardId);
  return (
    <Layout type={form} header={header} urgent={urgent} inPopover={inPopover}>
      <SelectFromImage values={cardChoices(skin, o.cards)} value={card} onChange={(v) => update({ _cardId: v ?? -1 })} imageWidth={80} />
      <ButtonRow confirm={() => submit()} confirmText={confirm(cardName(skin, card))} confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

export function DiscardedForm(p: FormProps) {
  return <CardForm {...p} form="Discarded" header="Select a card to discard" urgent confirm={(c) => fmt("Discard {0}", c)} />;
}

export function EmperorDiscardedForm(p: FormProps) {
  return <CardForm {...p} form="EmperorDiscarded" header={<>You may discard cards for <SpiceComponent amount="2" /> each</>} urgent confirm={(c) => fmt("Discard {0}", c)} />;
}

export function ChoamDiscardedForm(p: FormProps) {
  const { skin } = useGame();
  return (
    <CardForm {...p} form="ChoamDiscarded"
      header={<>You may discard surplus duplicate cards for <SpiceComponent amount="3" /> and {cardType(skin, "Useless")} cards for <SpiceComponent amount="2" /> each</>}
      confirm={(c) => fmt("Discard {0}", c)} />
  );
}

export function DiscardedSearchedForm(p: FormProps) {
  const { skin } = useGame();
  const name = cardType(skin, "SearchDiscarded");
  return <CardForm {...p} form="DiscardedSearched" header={<>Use {name}?</>} urgent confirm={() => fmt("Use {0}", name)} />;
}

export function DiscardedTakenForm(p: FormProps) {
  const { skin } = useGame();
  const name = cardType(skin, "TakeDiscarded");
  return <CardForm {...p} form="DiscardedTaken" header={<>Use {name}?</>} confirm={() => fmt("Use {0}", name)} />;
}

export function IxianSelectedStartingCardForm(p: FormProps) {
  return (
    <CardForm {...p} form="IxianSelectedStartingCard" urgent
      header="Select your starting Treachery Card. The remaining cards will be shuffled and dealt to the other factions."
      confirm={(c) => fmt("Select {0}", c)} />
  );
}

export function KarmaRicheseBuyForm(p: FormProps) {
  const { skin } = useGame();
  const karma = cardType(skin, "Karma");
  return (
    <CardForm {...p} form="KarmaRicheseBuy" header={<>Use {karma} to buy one of your special Treachery cards for <SpiceComponent amount="3" /></>}
      confirm={() => fmt("Use {0}", karma)} />
  );
}

export function RicheseGaveCardForm(p: FormProps) {
  return <CardForm {...p} form="RicheseGaveCard" header="Give your ally a card?" confirm={(c) => fmt("Give {0}", c)} />;
}

// IxianSwappedCardOnBidComponent: Pass sends `Passed`.
export function IxianSwappedCardOnBidForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "IxianSwappedCardOnBid");
  if (!o) return null;
  const card = num(draft._cardId);
  return (
    <Layout type="IxianSwappedCardOnBid" header="Swap the next card on auction with a card from your hand? You can do this once per Bidding round." urgent inPopover={inPopover}>
      <SelectFromImage values={cardChoices(skin, o.cards)} value={card} onChange={(v) => update({ _cardId: v ?? -1 })} imageWidth={80} />
      <ButtonRow confirm={() => submit()} confirmText={fmt("Swap {0}", cardName(skin, card))} confirmError={validation}
                 pass={() => submit({ Passed: true })} passText="Pass" />
      <SubmitError error={error} />
    </Layout>
  );
}

// IxianRemovedCardFromAuctionComponent: the second button puts the card at the bottom.
export function IxianRemovedCardFromAuctionForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "IxianRemovedCardFromAuction");
  if (!o) return null;
  const card = num(draft._cardId);
  return (
    <Layout type="IxianRemovedCardFromAuction" header="Put one card on top or on the bottom of the Treachery Card deck" urgent inPopover={inPopover}>
      <SelectFromImage values={cardChoices(skin, o.cards)} value={card} onChange={(v) => update({ _cardId: v ?? -1 })} imageWidth={80} />
      <ButtonRow confirm={() => submit({ PutOnTop: true })} confirmText="Put on top" confirmError={validation}
                 pass={() => submit({ PutOnTop: false })} passText="Put at bottom" />
      <SubmitError error={error} />
    </Layout>
  );
}

// A form whose only input is one faction.
function FactionForm({ action, inPopover = false, form, header, member, confirm, urgent = false, startCollapsed = false, label }: FormProps & {
  form: "ResourcesAudited" | "FactionSelected" | "KarmaHandSwapInitiated" | "FactionTradeOffered" | "ChoamKarmaPrevention";
  header: React.ReactNode;
  member: string;
  confirm: (faction: string) => string;
  urgent?: boolean;
  startCollapsed?: boolean;
  label?: React.ReactNode;
}) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, form);
  if (!o) return null;
  const value = num(draft[member]);
  const chosen = o.targets.find((t) => t.value === value)?.faction;
  return (
    <Layout type={form} header={header} urgent={urgent} startCollapsed={startCollapsed} inPopover={inPopover}>
      <SelectFromImage values={factionChoices(skin, o.targets)} value={value} onChange={(v) => update({ [member]: v ?? 0 })} imageWidth={40} label={label} />
      <ButtonRow confirm={() => submit()} confirmText={confirm(chosen ?? "")} confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

export function ResourcesAuditedForm(p: FormProps) {
  const { skin } = useGame();
  return (
    <FactionForm {...p} form="ResourcesAudited" member="Target" confirm={() => "Confirm"}
      header={<>Force a faction to reveal the amount of {skin.concept("Resource")}, weapons and defenses?</>} />
  );
}

export function FactionSelectedForm(p: FormProps) {
  return <FactionForm {...p} form="FactionSelected" member="Faction" confirm={() => "Choose Faction"} header="Choose your faction. If you don't, it will be assigned at random." />;
}

export function KarmaHandSwapInitiatedForm(p: FormProps) {
  const { skin } = useGame();
  return (
    <FactionForm {...p} form="KarmaHandSwapInitiated" member="Target" confirm={() => "Initiate Swap"} label="Target"
      header={<>Use {cardType(skin, "Karma")} to perform a Hand Swap</>} />
  );
}

export function ChoamKarmaPreventionForm(p: FormProps) {
  const { skin } = useGame();
  const o = p.action.options?.form === "ChoamKarmaPrevention" ? p.action.options : null;
  const karma = cardType(skin, "Karma");
  return (
    <FactionForm {...p} form="ChoamKarmaPrevention" member="Target" confirm={() => fmt("Prevent use of {0}", karma)}
      header={<>Use {o?.card ? <RichText parts={o.card} /> : "Nexus Cunning"} to prevent a faction from playing {karma} this phase?</>} />
  );
}

export function FactionTradeOfferedForm(p: FormProps) {
  const { skin } = useGame();
  const o = p.action.options?.form === "FactionTradeOffered" ? p.action.options : null;
  return (
    <FactionForm {...p} form="FactionTradeOffered" member="Target" header="Offer to trade factions"
      confirm={(f) => (o?.inPlay.some((x) => x === f) ? fmt("Offer Trade with {0}", skin.faction(f as never)) : fmt("Pick {0}", f === "" ? "" : skin.faction(f as never)))} />
  );
}

// DistransUsedComponent: a faction and a card.
export function DistransUsedForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "DistransUsed");
  if (!o) return null;
  const name = cardType(skin, "Distrans");
  return (
    <Layout type="DistransUsed" header={<>Use {name}?</>} inPopover={inPopover}>
      <SelectFromImage values={factionChoices(skin, o.targets)} value={num(draft.Target)} onChange={(v) => update({ Target: v ?? 0 })} imageWidth={40} />
      <div className="mt-1">
        <SelectFromImage values={cardChoices(skin, o.cards)} value={num(draft._cardId)} onChange={(v) => update({ _cardId: v ?? -1 })} imageWidth={80} />
      </div>
      <ButtonRow confirm={() => submit()} confirmText={fmt("Use {0}", name)} confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// CardTradedComponent: with a bot ally, a card to ask in return (not required).
export function CardTradedForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "CardTraded");
  if (!o) return null;
  const requested = num(draft._requestedCardId);
  return (
    <Layout type="CardTraded" header="Trade one of your cards with your ally?" inPopover={inPopover}>
      <SelectFromImage values={cardChoices(skin, o.cards)} value={num(draft._cardId)} onChange={(v) => update({ _cardId: v ?? -1 })} imageWidth={80} />
      {o.allyCards && (
        <>
          <div className="mt-1">Because you're playing with a Bot, you may select one of their cards to receive in return:</div>
          <SelectFromImage values={cardChoices(skin, o.allyCards)} value={requested >= 0 ? requested : null} required={false}
                           onChange={(v) => update({ _requestedCardId: v ?? -1 })} imageWidth={80} />
        </>
      )}
      <ButtonRow confirm={() => submit()} confirmText="Trade Card" confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// TraitorsSelectedComponent (no header) and TraitorDiscardedComponent.
function HeroForm({ action, inPopover = false, form, header, confirm }: FormProps & {
  form: "TraitorsSelected" | "TraitorDiscarded"; header: React.ReactNode | null; confirm: string;
}) {
  const { skin, view } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, form);
  if (!o) return null;
  const id = num(draft._traitorId);
  const hero = o.heroes.find((h) => h.id === id);
  return (
    <Layout type={form} header={header} urgent inPopover={inPopover}>
      <SelectFromImage values={heroChoices(skin, view, o.heroes)} value={id} onChange={(v) => update({ _traitorId: v ?? -1 })} imageWidth={80} />
      <ButtonRow confirm={() => submit()} confirmText={fmt(confirm, hero ? heroName(skin, hero.hero) : "")} confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

export function TraitorsSelectedForm(p: FormProps) {
  return <HeroForm {...p} form="TraitorsSelected" header={null} confirm="Keep {0}" />;
}

export function TraitorDiscardedForm(p: FormProps) {
  return <HeroForm {...p} form="TraitorDiscarded" header="Select a traitor to discard" confirm="Discard {0}" />;
}

// SkillAssignedComponent: a skill card and a leader.
export function SkillAssignedForm({ action, inPopover = false }: FormProps) {
  const { skin, view } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "SkillAssigned");
  if (!o) return null;
  const skills = o.skills.map((s) => ({ value: s.value, src: skin.url(skin.get("LeaderSkillCardImage_URL", s.name)), alt: s.label, popover: null, tooltip: s.label }));
  return (
    <Layout type="SkillAssigned" header="Assign a skill to a one of your leaders" urgent inPopover={inPopover}>
      <SelectFromImage values={skills} value={num(draft.Skill)} onChange={(v) => update({ Skill: v ?? 0 })} imageWidth={80} />
      <SelectFromImage values={heroChoices(skin, view, o.leaders)} value={num(draft._leaderId)} onChange={(v) => update({ _leaderId: v ?? -1 })} imageWidth={80} />
      <ButtonRow confirm={() => submit()} confirmText="Assign Skill" confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// BeneGesseritPredictionComponent (no header): the winner and the turn.
export function BeneGesseritPredictionForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "BeneGesseritPrediction");
  if (!o) return null;
  return (
    <Layout type="BeneGesseritPrediction" header={null} urgent inPopover={inPopover}>
      <SelectFromImage values={factionChoices(skin, o.targets)} value={num(draft.ToWin)} onChange={(v) => update({ ToWin: v ?? 0 })} imageWidth={40} />
      <SelectNumber value={num(draft.Turn)} onChange={(v) => update({ Turn: v })} min={1} max={o.maxTurn} />
      <ButtonRow confirm={() => submit()} confirmText="Confirm Prediction" confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// StormSpellPlayedComponent.
export function StormSpellPlayedForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "StormSpellPlayed");
  if (!o) return null;
  const name = cardType(skin, "StormSpell");
  return (
    <Layout type="StormSpellPlayed" header={<>Use {name}?</>} inPopover={inPopover}>
      <SelectNumber value={num(draft.MoveAmount)} onChange={(v) => update({ MoveAmount: v })} min={0} max={10} label="Number of sectors" />
      <ButtonRow confirm={() => submit()} confirmText={fmt("Use {0}", name)} confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// ResourcesTransferredComponent.
export function ResourcesTransferredForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "ResourcesTransferred");
  if (!o) return null;
  return (
    <Layout type="ResourcesTransferred" header={<>Transfer {skin.concept("Resource")} to your ally?</>} inPopover={inPopover}>
      <div className="mt-1">
        <SelectResources value={num(draft.Resources)} onChange={(v) => update({ Resources: v })} min={0} max={o.max} />
      </div>
      <ButtonRow confirm={() => submit()} confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// DivideResourcesComponent.
export function DivideResourcesForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "DivideResources");
  if (!o) return null;
  const portion = num(draft.PortionToFirstPlayer);
  return (
    <Layout type="DivideResources" header={<>Please choose how to divide collected {skin.concept("Resource")}</>} urgent inPopover={inPopover}>
      <div className="mt-1">
        <SpiceComponent amount={String(o.amount)} /> was collected from <PlaceChipTerritory id={o.territory} /> by you and <FactionChip faction={o.otherFaction} />
      </div>
      <div className="mt-1">
        <SelectResources value={portion} onChange={(v) => update({ PortionToFirstPlayer: v })} min={0} max={o.amount} label="How much should go to you?" />
      </div>
      <div className="mt-1">
        If <FactionChip faction={o.otherFaction} /> agree, they will receive the remaining <SpiceComponent amount={String(o.amount - portion)} />
      </div>
      <div className="mt-1">
        If you pass, you get <SpiceComponent amount={String(o.gainedIfPassed)} /> and <FactionChip faction={o.otherFaction} /> get <SpiceComponent amount={String(o.otherGainsIfPassed)} />
      </div>
      <ButtonRow pass={() => submit({ Passed: true })} confirm={() => submit()} confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

function PlaceChipTerritory({ id }: { id: number }) {
  return <PlaceChip place={{ territory: id }} />;
}

// ChoamEconomicsComponent: Double or Cancel.
export function ChoamEconomicsForm({ action, inPopover = false }: FormProps) {
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "ChoamEconomics");
  if (!o) return null;
  return (
    <Layout type="ChoamEconomics" header="Play Inflation?" inPopover={inPopover}>
      <Radio values={o.statuses.map((s) => ({ value: s.value, label: s.label }))} value={num(draft.Status)} onChange={(v) => update({ Status: v })} />
      <ButtonRow confirm={() => submit()} confirmText="Confirm Inflation" confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// KarmaComponent: the Karma card and the advantage to prevent.
export function KarmaForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "Karma");
  if (!o) return null;
  const karma = cardType(skin, "Karma");
  return (
    <Layout type="Karma" header={<>Use {karma} to prevent a faction from using one of their advantages</>} inPopover={inPopover}>
      <SelectFromImage values={cardChoices(skin, o.cards)} value={num(draft._cardId)} onChange={(v) => update({ _cardId: v ?? -1 })} imageWidth={80} />
      <Select options={o.advantages.map((a) => ({ value: a.value, label: a.label }))} value={num(draft.Prevented)} onChange={(v) => update({ Prevented: v })}
              label={<>Advantage to prevent <InfoIcon contents="Prevention of an advantages with a * cannot be done automatically and must be done verbally between players. This might require an undo-action by the host after the action you want to prevent has occured." /></>} />
      <ButtonRow confirm={() => submit()} confirmText={fmt("Use {0}", karma)} confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// GenericComponents/InfoComponent.razor: the gold "i" with a popover.
function InfoIcon({ contents }: { contents: string }) {
  return (
    <Hoverable popover={`<div class='pop'>${contents}</div>`} className="d-inline">
      <svg xmlns="http://www.w3.org/2000/svg" width={16} height={16} fill="var(--ink-0)" className="bi bi-info-circle" viewBox="0 0 16 16">
        <circle cx="8" cy="8" r="8" fill="var(--gold)" />
        <path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14zm0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16z" />
        <path d="M8.93 6.588l-2.29.287-.082.38.45.083c.294.07.352.176.288.469l-.738 3.468c-.194.897.105 1.319.808 1.319.545 0 1.178-.252 1.465-.598l.088-.416c-.2.176-.492.246-.686.246-.275 0-.375-.193-.304-.533L8.93 6.588zM9 4.5a1 1 0 1 1-2 0 1 1 0 0 1 2 0z" />
      </svg>
    </Hoverable>
  );
}

// KarmaChoamDiscardComponent: the Karma card (when there are several) and the cards to discard.
export function KarmaChoamDiscardForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "KarmaChoamDiscard");
  if (!o) return null;
  const karma = cardType(skin, "Karma");
  return (
    <Layout type="KarmaChoamDiscard" header={<>Use {karma} to discard any cards from your hand to gain <SpiceComponent amount="3" /> each</>} inPopover={inPopover}>
      {o.karmaCards.length > 1 && (
        <SelectFromImage values={cardChoices(skin, o.karmaCards)} value={num(draft._cardToUse)} imageWidth={80} label={<>{karma} card to use</>}
                         onChange={(v) => update({ _cardToUse: v ?? -1, _cardIds: "" })} />
      )}
      <SelectManyFromImage values={cardChoices(skin, o.cards)} selected={idList(draft._cardIds)} onChange={(v) => update({ _cardIds: idString(v) })}
                           imageWidth={80} label="Cards to discard" />
      <ButtonRow confirm={() => submit()} confirmText="Discard" confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// KarmaHandSwapComponent: the cards to give back.
export function KarmaHandSwapForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "KarmaHandSwap");
  if (!o) return null;
  return (
    <Layout type="KarmaHandSwap" header={<>Return {o.number} cards to <FactionChip faction={o.target} /></>} urgent inPopover={inPopover}>
      <SelectManyFromImage values={cardChoices(skin, o.cards)} selected={idList(draft._cardIds)} onChange={(v) => update({ _cardIds: idString(v) })} imageWidth={80} />
      <ButtonRow confirm={() => submit()} confirmText="Return Cards" confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// Forms with only buttons.
export function ReplacedCardWonForm({ action, inPopover = false }: FormProps) {
  const { options: o, validation, submit, error } = useOtherDraft(action, "ReplacedCardWon");
  if (!o) return null;
  return (
    <Layout type="ReplacedCardWon" header="Replace the card you just won by discarding it and drawing a new card from the Treachery Card deck?" inPopover={inPopover}>
      <ButtonRow confirm={() => submit()} confirmText="Yes" confirmError={validation} pass={() => submit({ Passed: true })} passText="No" />
      <SubmitError error={error} />
    </Layout>
  );
}

export function RicheseKeepsUnsoldCardForm({ action, inPopover = false }: FormProps) {
  const { options: o, validation, submit, error } = useOtherDraft(action, "RicheseKeepsUnsoldCard");
  if (!o) return null;
  return (
    <Layout type="RicheseKeepsUnsoldCard" header="Do you wish to get the card no faction bid on for free or should it be removed from the game?" urgent inPopover={inPopover}>
      <ButtonRow confirm={() => submit()} confirmText="Get for free" confirmError={validation} pass={() => submit({ Passed: true })} passText="Remove from game" />
      <SubmitError error={error} />
    </Layout>
  );
}

export function RicheseAnnouncesAuctionForm({ action, inPopover = false }: FormProps) {
  const { options: o, validation, submit, error } = useOtherDraft(action, "RicheseAnnouncesAuction");
  if (!o) return null;
  return (
    <Layout type="RicheseAnnouncesAuction" header={<>Auction a card from your <FactionChip faction="Richese" /> card cache First or Last?</>} urgent inPopover={inPopover}>
      <ButtonRow confirm={() => submit({ First: true })} confirmText="First" confirmError={validation} other={() => submit({ First: false })} otherText="Last" />
      <SubmitError error={error} />
    </Layout>
  );
}

// MulliganPerformedComponent and NexusCardDrawnComponent: without permission only "No".
function MayForm({ action, inPopover = false, form, header, noText }: FormProps & { form: "MulliganPerformed" | "NexusCardDrawn"; header: string; noText: string }) {
  const { options: o, validation, submit, error } = useOtherDraft(action, form);
  if (!o) return null;
  return (
    <Layout type={form} header={header} urgent inPopover={inPopover}>
      {o.may
        ? <ButtonRow confirm={() => submit()} confirmText="Yes" confirmError={validation} pass={() => submit({ Passed: true })} passText={noText} />
        : <ButtonRow pass={() => submit({ Passed: true })} passText="No" />}
      <SubmitError error={error} />
    </Layout>
  );
}

export function MulliganPerformedForm(p: FormProps) {
  return <MayForm {...p} form="MulliganPerformed" header="Draw a new selection of four traitors?" noText="No (proceed with current traitors)" />;
}

export function NexusCardDrawnForm(p: FormProps) {
  return <MayForm {...p} form="NexusCardDrawn" header="Draw a Nexus Card?" noText="No" />;
}

const directions = [{ value: 1, label: "Counter-clockwise" }, { value: -1, label: "Clockwise" }];

// RicheseAnnouncesBlackMarketComponent.
export function RicheseAnnouncesBlackMarketForm({ action, inPopover = false }: FormProps) {
  const { skin, view } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "RicheseAnnouncesBlackMarket");
  if (!o) return null;
  const card = num(draft._cardId);
  const onceAround = o.auctionTypes.find((t) => t.name === "BlackMarketOnceAround")?.value;
  void view;
  return (
    <Layout type="RicheseAnnouncesBlackMarket" header="Select a card to put on the Black Market, or pass" urgent inPopover={inPopover}>
      <SelectFromImage values={cardChoices(skin, o.cards)} value={card} onChange={(v) => update({ _cardId: v ?? -1 })} imageWidth={80} />
      <Radio values={o.auctionTypes.map((t) => ({ value: t.value, label: t.label }))} value={num(draft.AuctionType)} onChange={(v) => update({ AuctionType: v })}
             label="Type of auction" />
      {num(draft.AuctionType) === onceAround && (
        <Radio values={directions} value={num(draft.Direction)} onChange={(v) => update({ Direction: v })} label="Bidding direction" />
      )}
      <ButtonRow confirm={() => submit()} confirmText={fmt("Put {0} on Black Market", cardName(skin, card))} confirmError={validation}
                 pass={() => submit({ Passed: true })} passText="Pass" />
      <SubmitError error={error} />
    </Layout>
  );
}

// RicheseSpecifiesAuctionComponent.
export function RicheseSpecifiesAuctionForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "RicheseSpecifiesAuction");
  if (!o) return null;
  const onceAround = o.auctionTypes.find((t) => t.name === "RicheseOnceAround")?.value;
  return (
    <Layout type="RicheseSpecifiesAuction" header="Define the auction" urgent inPopover={inPopover}>
      {o.maySpecifyCard && (
        <SelectFromImage values={cardChoices(skin, o.cards)} value={num(draft._cardId)} onChange={(v) => update({ _cardId: v ?? -1 })} imageWidth={80} />
      )}
      {o.maySpecifyType && (
        <>
          <Radio values={o.auctionTypes.map((t) => ({ value: t.value, label: t.label }))} value={num(draft.AuctionType)}
                 onChange={(v) => update({ AuctionType: v })} label="Type of auction" />
          {num(draft.AuctionType) === onceAround && (
            <Radio values={directions} value={num(draft.Direction)} onChange={(v) => update({ Direction: v })} label="Bidding direction" />
          )}
        </>
      )}
      <ButtonRow confirm={() => submit()} confirmText="Start Auction" confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// StormDialledComponent.
export function StormDialledForm({ action, inPopover = false }: FormProps) {
  const { source } = useGame();
  const o = action.options?.form === "StormDialled" ? action.options : null;
  return o ? <StormDial action={action} min={o.min} max={o.max} inPopover={inPopover} submit={source.submit.bind(source)} /> : null;
}

function StormDial({ action, min, max, inPopover, submit }: { action: ActionView; min: number; max: number; inPopover: boolean; submit: (e: GameEventDraft) => Promise<string | null> }) {
  const { source } = useGame();
  // The field starts at 0, which the number picker clamps into [Min, Max].
  const [amount, setAmount] = useState(() => (0 < min ? min : 0 > max ? max : 0));
  const [validation, setValidation] = useState<Part[] | null>(action.validation);
  const [error, setError] = useState<string | null>(null);
  const latest = useRef(0);
  const ask = async (v: number) => {
    const n = ++latest.current;
    const r = await source.query({ $type: action.eventType, Amount: v });
    if (n === latest.current) setValidation(r.validation);
  };
  const change = (v: number) => {
    setAmount(v);
    void ask(v);
  };
  // The dial is kept across views, as the C# field is; a new view validates it again (formDraft.ts).
  useViewChanges(() => { setError(null); void ask(amount); });
  return (
    <Layout type="StormDialled" header="Please dial a number to determine storm movement" urgent inPopover={inPopover}>
      <div className="d-flex align-items-center">
        <NumberPicker value={amount} onChange={change} min={min} max={max} symbolFill="white" />
      </div>
      <ButtonRow confirm={async () => setError(await submit({ $type: action.eventType, Amount: amount }))} confirmText={`Dial ${amount}`} confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// AllianceOfferedComponent. The view's validation is for the first target, which the required
// image choice starts with (rust/view/src/actions.rs).
export function AllianceOfferedForm({ action, inPopover = false }: FormProps) {
  const { skin, source } = useGame();
  const o = action.options?.form === "AllianceOffered" ? action.options : null;
  const targets = (o?.targets ?? []).map((f) => ({ faction: f, value: FactionValue[f] }));
  const [target, setTarget] = useState<number | null>(null);
  const [validation, setValidation] = useState<Part[] | null>(action.validation);
  const [error, setError] = useState<string | null>(null);
  const chosen = target != null && targets.some((t) => t.value === target) ? target : (targets[0]?.value ?? null);
  const latest = useRef(0);
  const ask = async (v: number | null) => {
    const n = ++latest.current;
    const r = await source.query({ $type: action.eventType, Target: v ?? 0 });
    if (n === latest.current) setValidation(r.validation);
  };
  const change = (v: number | null) => {
    setTarget(v);
    void ask(v);
  };
  // The target is kept across views, as the C# field is; a new view validates it again
  // (formDraft.ts).
  useViewChanges(() => { setError(null); void ask(chosen); });
  if (!o) return null;
  return (
    <Layout type="AllianceOffered" header="Offer a faction to form an Alliance" inPopover={inPopover}>
      <SelectFromImage values={factionChoices(skin, targets)} value={chosen} onChange={change} imageWidth={40} />
      <ButtonRow confirm={async () => setError(await source.submit({ $type: action.eventType, Target: chosen ?? 0 }))} confirmText="Offer Alliance"
                 confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}
