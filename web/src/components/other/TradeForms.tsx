// The "other events" forms about allies, gifts, deals and automation (package 7e):
// AllyPermissionComponent, DonatedComponent, DealOfferedComponent (the three Trade.Actions, which
// open from the menu bar's TradePopover), EmperorBidSupportComponent and
// AutomationConfiguredComponent (opened from the menu bar).
import { useEffect, useRef, useState } from "react";
import type { Faction } from "../../generated/enums";
import { useGame } from "../../state/context";
import { ButtonRow } from "../ButtonRow";
import { Checkbox } from "../Checkbox";
import { FactionChip } from "../FactionChip";
import { MonsterIcon } from "../MonsterIcon";
import { SelectResources, SelectResourcesFromFaction } from "../NumberPicker";
import { heroChoices, SelectForces } from "../revival/parts";
import { RichText } from "../RichText";
import { SelectFromImage } from "../SelectFromImage";
import { SpiceComponent } from "../SpiceComponent";
import { Image } from "../Image";
import type { FormProps } from "./ChoiceForms";
import { cardChoices, cardType, CommandButton, factionChoices, Layout, num, Segmented, SelectManyFromImage, SubmitError, useFaction, useOtherDraft } from "./parts";
import { Select } from "../Select";

const yesNo = (b: unknown) => (b === true ? "yes" : "no");

// AllyPermissionComponent: what the player allows their ally. The confirm button is disabled until
// something is edited; the draft starts from the permissions in force.
export function AllyPermissionForm({ action, inPopover = false }: FormProps) {
  const { skin, view } = useGame();
  const faction = useFaction();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "AllyPermission");
  // Set by an edit and cleared by Confirm Changes only (ConfirmAndSetUnedited), not by a new view.
  const [edited, setEdited] = useState(false);
  if (!o) return null;
  const cur = o.current;
  const edit = (changes: Record<string, unknown>) => { setEdited(true); void update(changes); };
  const check = (member: string, label: React.ReactNode, aliases: string[] = []) => (
    <Checkbox value={draft[member] === true} onChange={(v) => edit(Object.fromEntries([member, ...aliases].map((m) => [m, v])))}>
      {label} (currently: <strong>{yesNo(cur[member])}</strong>)
    </Checkbox>
  );
  const karma = cardType(skin, "Karma");
  const currentKarma = o.currentKarmaCard;
  const willingToPay = `How much are you willing to pay for their ${faction === "Choam" ? "battles, " : ""}bids and shipments (currently: ${o.currentResources})? ${faction === "Emperor" ? "Payments for bids flow back to you." : ""}`;
  const c = o.botClaims;
  return (
    <Layout type="AllyPermission" header={<>Manage your Alliance with <FactionChip faction={o.ally} /></>} startCollapsed inPopover={inPopover}>
      {c && (
        <>
          <div>{c.ownsWeapon ? "They claim they own a weapon." : "They claim they don't own weapons."}</div>
          <div>{c.ownsDefense ? "They claim they own a defense." : "They claim they don't own defenses."}</div>
          {c.safeLeaders.length > 0
            ? <div>They claim you can safely use: {heroChoices(skin, view, c.safeLeaders).map((h) => <Image key={h.value} src={h.src} alt={h.alt} width={30} popover={h.popover} />)}</div>
            : <div>They don't know of any safe leaders you could use.</div>}
          <div>They own <SpiceComponent amount={String(c.resources)} />.</div>
        </>
      )}
      <div className="mt-1">
        <SelectResources value={num(draft.PermittedResources)} onChange={(v) => edit({ PermittedResources: v })} min={0} max={o.maxResources} label={willingToPay} />
      </div>
      {o.karmaCards && (
        <SelectFromImage values={cardChoices(skin, o.karmaCards)} value={num(draft._permittedKarmaCardId) >= 0 ? num(draft._permittedKarmaCardId) : null} required={false}
                         onChange={(v) => edit({ _permittedKarmaCardId: v ?? -1 })} imageWidth={80}
                         label={<>Select a {karma} card they may use to ship at <FactionChip faction="Guild" /> rates (currently: {currentKarma == null ? <strong>none</strong> : <RichText parts={[{ t: "treacheryCard", id: currentKarma, label: skin.card(currentKarma) }]} />})?</>} />
      )}
      {faction === "Emperor" && (
        <div className="mt-1">
          <label className="mt-1 mb-0">How many revivals will you pay for them (currently: <strong>{num(cur.EmperorWillPayForExtraRevival)}</strong>)?</label>
          <SelectForces value={num(draft.EmperorWillPayForExtraRevival)} onChange={(v) => edit({ EmperorWillPayForExtraRevival: v })} min={0} max={3} faction={o.ally} special={false} />
        </div>
      )}
      {faction === "Guild" && check("GuildAllowsShippingDiscount", <>Allow shipment at <FactionChip faction="Guild" /> rates and site-to-site</>, ["AllyMayShipAsGuild"])}
      {faction === "Tleilaxu" && check("TleilaxuAllowsRevivalDiscount", <>Allow revival at <FactionChip faction="Tleilaxu" /> rates</>, ["AllyMayReviveAsTleilaxu"])}
      {faction === "Ixian" && check("IxianAllowsReplacingCards", "Allow replacing treachery cards they win", ["AllyMayReplaceCards"])}
      {faction === "Fremen" && (
        <>
          {check("FremenAllowsThreeFreeRevivals", "Allow 3 free revivals")}
          {check("FremenWillProtectFromMonster", <>Protect them from <MonsterIcon /></>)}
          {o.fremenDeterminesStorm && check("FremenSharesPrescience", "Share storm knowledge")}
          {o.fremenRefundsDial && check("FremenRefundsBattleDial", `Refund ${skin.concept("Resource")} dialled in battle`)}
        </>
      )}
      {faction === "Atreides" && check("AtreidesSharesPrescience", "Share your Prescience")}
      {faction === "BeneGesserit" && check("BeneGesseritAllowsUseOfVoice", "Allow use of Voice")}
      {faction === "Richese" && check("RicheseAllowsUseOfNoField", "Allow shipment by No-Field")}
      {faction === "Moritani" && check("MoritaniAllowsKeepingCards", "Allow keeping a treachery card after a lost battle")}
      {faction === "Ecaz" && check("EcazSharesAmbassadors", "Allow them to activate and benefit from ambassadors")}
      <ButtonRow confirm={async () => { await submit(); setEdited(false); }} confirmText="Confirm Changes" confirmError={validation} disabled={!edited} />
      <SubmitError error={error} />
    </Layout>
  );
}

// DonatedComponent: spice and, with Rule.CardsCanBeTraded, a card. Hosts may give from the bank.
export function DonatedForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "Donated");
  if (!o) return null;
  const resource = skin.concept("Resource");
  const card = num(draft._cardId);
  return (
    <Layout type="Donated" header={<>Give {resource} to a faction</>} startCollapsed inPopover={inPopover}>
      <SelectFromImage values={factionChoices(skin, o.targets)} value={num(draft.Target)} onChange={(v) => update({ Target: v ?? 0 })} imageWidth={40} />
      <div className="mt-1">
        <SelectResources value={num(draft.Resources)} onChange={(v) => update({ Resources: v })} min={o.min} max={o.max} />
      </div>
      {o.cards && (
        <div className="mt-1">
          <SelectFromImage values={cardChoices(skin, o.cards)} value={card >= 0 ? card : null} required={false} onChange={(v) => update({ _cardId: v ?? -1 })} imageWidth={80} />
        </div>
      )}
      {o.isHost && (
        <Checkbox value={draft.FromBank === true} onChange={(v) => update({ FromBank: v })}>
          Host only: give from {resource} bank (you may select a negative amount to put that amount of {resource} from target faction in the bank)
        </Checkbox>
      )}
      <ButtonRow confirm={() => submit()} confirmText="Confirm Gift" confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// DealOfferedComponent. Every target starts picked, which offers to everyone (To empty); a click
// drops a faction, shift-click offers to that faction alone.
export function DealOfferedForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "DealOffered");
  // The C# component's fields, kept across events while the form stays open; only the text is
  // cleared after an offer (ConfirmOffer). The draft is kept across views too (formDraft.ts); when
  // the options come back after an answer for another form, the fields are sent again.
  const [to, setTo] = useState<number[] | null>(null);
  const [theyPay, setTheyPay] = useState(true);
  const [amount, setAmount] = useState(0);
  const [type, setType] = useState<number | null>(null);
  const [until, setUntil] = useState<number | null>(null);
  const [text, setText] = useState("");
  const all = o?.targets.map((t) => t.value) ?? [];
  const picked = to == null ? all : all.filter((v) => to.includes(v));
  const toField = (p: number[]) => (all.every((v) => p.includes(v)) ? [] : p);
  const fields = () => ({
    To: toField(picked), Price: theyPay ? amount : 0, Benefit: theyPay ? 0 : amount, Text: text,
    ...(type != null ? { Type: type } : {}), ...(until != null ? { EndPhase: until } : {}),
  });
  const edited = to != null || !theyPay || amount !== 0 || type != null || until != null || text !== "";
  const hadOptions = useRef(o != null);
  useEffect(() => {
    if (o && !hadOptions.current && edited) void update(fields());
    hadOptions.current = o != null;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [o != null]);
  if (!o) return null;
  const setPayment = (pay: boolean, a: number) => {
    setTheyPay(pay);
    setAmount(a);
    void update({ Price: pay ? a : 0, Benefit: pay ? 0 : a });
  };
  const dealType = o.dealTypes.find((t) => t.value === (type ?? num(draft.Type))) ?? o.dealTypes[0];
  return (
    <Layout type="DealOffered" header="Offer a Deal" startCollapsed inPopover={inPopover}>
      <SelectManyFromImage values={factionChoices(skin, o.targets)} selected={picked} imageWidth={40} label="Offer to"
                           onChange={(p) => { setTo(p); void update({ To: toField(p) }); }} />
      <label className="d-block mt-1 mb-0">Payment on accepting</label>
      <div className="d-flex align-items-center flex-wrap gap-2">
        <Segmented values={[{ value: true, label: "They pay you" }, { value: false, label: "You pay them" }]} value={theyPay} className=""
                   onChange={(v) => setPayment(v, amount)} />
        <SelectResources value={amount} onChange={(v) => setPayment(theyPay, v)} min={0} max={50} />
      </div>
      <Select options={o.dealTypes.map((t) => ({ value: t.value, label: t.label }))} value={type ?? num(draft.Type)}
              onChange={(v) => { setType(v); void update({ Type: v }); }} label="Type of deal" />
      {dealType && (
        <Segmented values={dealType.endPhases.map((p) => ({ value: p.value, label: p.label }))} value={until ?? num(draft.EndPhase)}
                   onChange={(v) => { setUntil(v); void update({ EndPhase: v }); }} label="Until" />
      )}
      <textarea className="form-control mt-1" rows={5} maxLength={512} placeholder="Deal description" value={text}
                onChange={(e) => { setText(e.target.value); void update({ Text: e.target.value }); }} />
      <ButtonRow confirm={async () => { await submit(fields()); setText(""); void update({ Text: "" }); }} confirmText="Offer Deal"
                 confirmError={picked.length === 0 ? [{ t: "text", v: "Pick at least one faction" }] : validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// EmperorBidSupportComponent: how much each non-allied opponent may use for bids.
export function EmperorBidSupportForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "EmperorBidSupport");
  if (!o) return null;
  const amounts = (draft.Amounts ?? {}) as Record<string, number>;
  return (
    <Layout type="EmperorBidSupport" header={<>How much of your {skin.concept("Resource")} may non-allied factions use for bidding?</>} inPopover={inPopover}>
      {o.targets.map((t) => (
        <SelectResourcesFromFaction key={t.faction} value={amounts[t.faction] ?? 0} min={0} max={o.max} faction={t.faction}
                                    onChange={(v) => update({ Amounts: { ...amounts, [t.faction]: v } })} />
      ))}
      <div className="text-info small">Your contributions to opponent bids will flow back to you <i>at the end</i> of the bidding phase.</div>
      <ButtonRow confirm={() => submit()} confirmText="Confirm Support" confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// ItemAction.Create and ItemAction.Delete.
const CREATE = 0;
const DELETE = 2;

// AutomationConfiguredComponent (MenuPanels): create a rule, or cancel one in force.
export function AutomationConfiguredForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const { options: o, draft, update, submit, error } = useOtherDraft(action, "AutomationConfigured");
  if (!o) return null;
  const type = o.ruleTypes.find((r) => r.value === num(draft.RuleType))?.name;
  return (
    <Layout type="AutomationConfigured" header="Manage Automation" inPopover={inPopover}>
      <Select options={o.ruleTypes.map((r) => ({ value: r.value, label: r.label }))} value={num(draft.RuleType)} onChange={(v) => update({ RuleType: v })} />
      <div className="mt-1">
        {type === "BiddingPassAboveAmount" && (
          <SelectResources value={num(draft.BiddingAboveAmount)} onChange={(v) => update({ BiddingAboveAmount: v })} min={0} max={40} />
        )}
        {type === "BiddingPassWhenHighestBidByFaction" && (
          <SelectFromImage values={factionChoices(skin, o.biddingFactions)} value={num(draft.BiddingWinningFaction)}
                           onChange={(v) => update({ BiddingWinningFaction: v ?? 0 })} imageWidth={40} />
        )}
      </div>
      <CommandButton margin="mt-1" onClick={() => submit({ Action: CREATE })}>Create rule</CommandButton>
      {o.rules.length > 0 && (
        <>
          <hr className="hr-hairline" />
          <div className="mt-2">Current automation:</div>
          <table className="table table-sm">
            <tbody>
              {o.rules.map((r) => (
                <tr key={r.id} className="table-dark">
                  <td>
                    <CommandButton small onClick={() => submit({ AutomationRuleId: r.id, Action: DELETE })}>
                      <svg width="1em" height="1em" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" aria-label="Cancel" style={{ verticalAlign: "-0.125em" }}>
                        <path d="M3.5 3.5L12.5 12.5M12.5 3.5L3.5 12.5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
                      </svg>
                    </CommandButton>
                  </td>
                  <td><RichText parts={r.description} size={18} /></td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}
      <SubmitError error={error} />
    </Layout>
  );
}

export type { Faction };
