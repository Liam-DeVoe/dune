// The "other events" forms about allies, gifts, deals and automation (package 7e):
// AllyPermissionComponent, DonatedComponent, DealOfferedComponent (the three Trade.Actions, which
// open from the menu bar's TradePopover), EmperorBidSupportComponent and
// AutomationConfiguredComponent (opened from the menu bar).
import { useEffect, useRef, useState } from "react";
import { type AutomationRuleType, type DealType, type Faction, type Phase } from "../../generated/enums";
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
import { type Fields, fieldsOf } from "../formDraft";
import { cardChoices, cardType, CommandButton, factionChoices, factionOf, idOf, Layout, nameOf, num, Segmented, SelectManyFromImage, SubmitError, useFaction, useOtherDraft } from "./parts";
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
  const cur = fieldsOf(o.current);
  const edit = (changes: Fields) => { setEdited(true); void update(changes); };
  const check = (member: keyof Fields, label: React.ReactNode) => (
    <Checkbox value={draft[member] === true} onChange={(v) => edit({ [member]: v })}>
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
      <div className="event-form__field">
        <SelectResources value={num(draft.permittedResources)} onChange={(v) => edit({ permittedResources: v })} min={0} max={o.maxResources} label={willingToPay} />
      </div>
      {o.karmaCards && (
        <SelectFromImage values={cardChoices(skin, o.karmaCards)} value={idOf(draft.permittedKarmaCard)} required={false}
                         onChange={(v) => edit({ permittedKarmaCard: v })} imageWidth={80}
                         label={<>Select a {karma} card they may use to ship at <FactionChip faction="Guild" /> rates (currently: {currentKarma == null ? <strong>none</strong> : <RichText parts={[{ t: "treacheryCard", id: currentKarma, label: skin.card(currentKarma) }]} />})?</>} />
      )}
      {faction === "Emperor" && (
        <div className="event-form__field">
          <label className="event-form__label event-form__label--inline">How many revivals will you pay for them (currently: <strong>{num(cur.emperorWillPayForExtraRevival)}</strong>)?</label>
          <SelectForces value={num(draft.emperorWillPayForExtraRevival)} onChange={(v) => edit({ emperorWillPayForExtraRevival: v })} min={0} max={3} faction={o.ally} special={false} />
        </div>
      )}
      {faction === "Guild" && check("allyMayShipAsGuild", <>Allow shipment at <FactionChip faction="Guild" /> rates and site-to-site</>)}
      {faction === "Tleilaxu" && check("allyMayReviveAsTleilaxu", <>Allow revival at <FactionChip faction="Tleilaxu" /> rates</>)}
      {faction === "Ixian" && check("allyMayReplaceCards", "Allow replacing treachery cards they win")}
      {faction === "Fremen" && (
        <>
          {check("fremenAllowsThreeFreeRevivals", "Allow 3 free revivals")}
          {check("fremenWillProtectFromMonster", <>Protect them from <MonsterIcon /></>)}
          {o.fremenDeterminesStorm && check("fremenSharesPrescience", "Share storm knowledge")}
          {o.fremenRefundsDial && check("fremenRefundsBattleDial", `Refund ${skin.concept("Resource")} dialled in battle`)}
        </>
      )}
      {faction === "Atreides" && check("atreidesSharesPrescience", "Share your Prescience")}
      {faction === "BeneGesserit" && check("beneGesseritAllowsUseOfVoice", "Allow use of Voice")}
      {faction === "Richese" && check("richeseAllowsUseOfNoField", "Allow shipment by No-Field")}
      {faction === "Moritani" && check("moritaniAllowsKeepingCards", "Allow keeping a treachery card after a lost battle")}
      {faction === "Ecaz" && check("ecazSharesAmbassadors", "Allow them to activate and benefit from ambassadors")}
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
  const card = idOf(draft.card);
  return (
    <Layout type="Donated" header={<>Give {resource} to a faction</>} startCollapsed inPopover={inPopover}>
      <SelectFromImage values={factionChoices(skin, o.targets)} value={factionOf(draft.target)} onChange={(v) => update({ target: v })} imageWidth={40} />
      <div className="event-form__field">
        <SelectResources value={num(draft.resources)} onChange={(v) => update({ resources: v })} min={o.min} max={o.max} />
      </div>
      {o.cards && (
        <div className="event-form__field">
          <SelectFromImage values={cardChoices(skin, o.cards)} value={card} required={false} onChange={(v) => update({ card: v })} imageWidth={80} />
        </div>
      )}
      {o.isHost && (
        <Checkbox value={draft.fromBank === true} onChange={(v) => update({ fromBank: v })}>
          Host only: give from {resource} bank (you may select a negative amount to put that amount of {resource} from target faction in the bank)
        </Checkbox>
      )}
      <ButtonRow confirm={() => submit()} confirmText="Confirm Gift" confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// DealOfferedComponent. Every target starts picked, which offers to everyone (`to` empty); a click
// drops a faction, shift-click offers to that faction alone.
export function DealOfferedForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "DealOffered");
  // The C# component's fields, kept across events while the form stays open; only the text is
  // cleared after an offer (ConfirmOffer). The draft is kept across views too (formDraft.ts); when
  // the options come back after an answer for another form, the fields are sent again.
  const [to, setTo] = useState<Faction[] | null>(null);
  const [theyPay, setTheyPay] = useState(true);
  const [amount, setAmount] = useState(0);
  const [type, setType] = useState<DealType | null>(null);
  const [until, setUntil] = useState<Phase | null>(null);
  const [text, setText] = useState("");
  const all = o?.targets.map((t) => t.faction) ?? [];
  const picked = to == null ? all : all.filter((v) => to.includes(v));
  const toField = (p: Faction[]) => (all.every((v) => p.includes(v)) ? [] : p);
  const fields = (): Fields => ({
    to: toField(picked), price: theyPay ? amount : 0, benefit: theyPay ? 0 : amount, text,
    ...(type != null ? { dealType: type } : {}), ...(until != null ? { endPhase: until } : {}),
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
    void update({ price: pay ? a : 0, benefit: pay ? 0 : a });
  };
  const typeName = type ?? (nameOf(draft.dealType) as DealType | null) ?? "None";
  const dealType = o.dealTypes.find((t) => t.name === typeName) ?? o.dealTypes[0];
  return (
    <Layout type="DealOffered" header="Offer a Deal" startCollapsed inPopover={inPopover}>
      <SelectManyFromImage values={factionChoices(skin, o.targets)} selected={picked} imageWidth={40} label="Offer to"
                           onChange={(p) => { setTo(p); void update({ to: toField(p) }); }} />
      <label className="event-form__label">Payment on accepting</label>
      <div className="event-form__payment">
        <Segmented values={[{ value: true, label: "They pay you" }, { value: false, label: "You pay them" }]} value={theyPay} className=""
                   onChange={(v) => setPayment(v, amount)} />
        <SelectResources value={amount} onChange={(v) => setPayment(theyPay, v)} min={0} max={50} />
      </div>
      <Select options={o.dealTypes.map((t) => ({ value: t.name, label: t.label }))} value={typeName}
              onChange={(v) => { setType(v); void update({ dealType: v }); }} label="Type of deal" />
      {dealType && (
        <Segmented values={dealType.endPhases.map((p) => ({ value: p.name as Phase, label: p.label }))} value={until ?? (nameOf(draft.endPhase) as Phase)}
                   onChange={(v) => { setUntil(v); void update({ endPhase: v }); }} label="Until" />
      )}
      <textarea className="input event-form__field" rows={5} maxLength={512} placeholder="Deal description" value={text}
                onChange={(e) => { setText(e.target.value); void update({ text: e.target.value }); }} />
      <ButtonRow confirm={async () => { await submit(fields()); setText(""); void update({ text: "" }); }} confirmText="Offer Deal"
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
  const amounts = (draft.amounts ?? {}) as Record<string, number>;
  return (
    <Layout type="EmperorBidSupport" header={<>How much of your {skin.concept("Resource")} may non-allied factions use for bidding?</>} inPopover={inPopover}>
      {o.targets.map((t) => (
        <SelectResourcesFromFaction key={t.faction} value={amounts[t.faction] ?? 0} min={0} max={o.max} faction={t.faction}
                                    onChange={(v) => update({ amounts: { ...amounts, [t.faction]: v } })} />
      ))}
      <div className="event-form__note event-form__note--info">Your contributions to opponent bids will flow back to you <i>at the end</i> of the bidding phase.</div>
      <ButtonRow confirm={() => submit()} confirmText="Confirm Support" confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}

// AutomationConfiguredComponent (MenuPanels): create a rule, or cancel one in force.
export function AutomationConfiguredForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const { options: o, draft, update, submit, error } = useOtherDraft(action, "AutomationConfigured");
  if (!o) return null;
  const type = nameOf(draft.ruleType) as AutomationRuleType | null;
  return (
    <Layout type="AutomationConfigured" header="Manage Automation" inPopover={inPopover}>
      <Select options={o.ruleTypes.map((r) => ({ value: r.name as AutomationRuleType, label: r.label }))} value={type} onChange={(v) => update({ ruleType: v })} />
      <div className="event-form__field">
        {type === "BiddingPassAboveAmount" && (
          <SelectResources value={num(draft.biddingAboveAmount)} onChange={(v) => update({ biddingAboveAmount: v })} min={0} max={40} />
        )}
        {type === "BiddingPassWhenHighestBidByFaction" && (
          <SelectFromImage values={factionChoices(skin, o.biddingFactions)} value={factionOf(draft.biddingWinningFaction)}
                           onChange={(v) => update({ biddingWinningFaction: v })} imageWidth={40} />
        )}
      </div>
      <CommandButton margin="event-form__field" onClick={() => submit({ action: "Create" })}>Create rule</CommandButton>
      {o.rules.length > 0 && (
        <>
          <hr className="hr-hairline" />
          <div className="event-form__section">Current automation:</div>
          <table className="table table--small table--dark">
            <tbody>
              {o.rules.map((r) => (
                <tr key={r.id}>
                  <td>
                    <CommandButton small onClick={() => submit({ automationRuleId: r.id, action: "Delete" })}>
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
