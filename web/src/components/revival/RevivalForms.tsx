// The revival forms (client-plan.md package 7d), one per C# component in
// Client/GameEventComponents. The options come from rust/view/src/actions/revival.rs; each form
// keeps the server's adjusted draft (options.draft) and sends a draft query after every change.
import { useEffect, type MouseEvent } from "react";
import type { ActionView } from "../../generated/ActionView";
import type { Faction } from "../../generated/enums";
import { useGame } from "../../state/context";
import { ButtonRow } from "../ButtonRow";
import { Checkbox } from "../Checkbox";
import { FactionChip } from "../FactionChip";
import { ForceToken } from "../ForceToken";
import { Image } from "../Image";
import { SelectResources } from "../NumberPicker";
import { RichText } from "../RichText";
import { SelectFromImage } from "../SelectFromImage";
import { SimpleNumber } from "../SimpleNumber";
import { useMapPicker } from "../board/mapInteraction";
import { factionChoices, GameEventLayout, heroChoices, heroImage, PinActions, SelectForces, SubmitError, useRevivalDraft, YesNo } from "./parts";

// The event types drawn here.
export const revivalForms = new Set([
  "Revival", "RaiseDeadPlayed", "KarmaFreeRevival", "KarmaRevivalPrevention", "ChoamFreeRevivalPrevention",
  "RequestTleilaxuRevival", "AcceptOrCancelTleilaxuRevival", "SetIncreasedRevivalLimits",
]);

const MAX_INT = 2147483647;

function num(v: unknown): number {
  return typeof v === "number" ? v : 0;
}

function heroId(v: unknown): number | null {
  return typeof v === "number" && v >= 0 ? v : null;
}

// The viewer's faction, for force tokens.
function useFaction(): Faction {
  const { view } = useGame();
  return (view.me?.faction ?? view.viewer.faction ?? "None") as Faction;
}

// Skin.Describe(Player.Force): the force name used as the token's title.
function useForceName(faction: Faction): string {
  const { skin } = useGame();
  return skin.get("ForceName_STR", faction);
}

function territoryOf(skin: ReturnType<typeof useGame>["skin"], l: number): number {
  return skin.locationInfo(l)?.territory ?? -1;
}

// RevivalComponent.razor.
function RevivalEventForm({ action, inPopover }: { action: ActionView; inPopover: boolean }) {
  const { skin, view } = useGame();
  const faction = useFaction();
  const forceName = useForceName(faction);
  const { options: o, draft, validation, update, submit, error } = useRevivalDraft(action, "Revival");
  const toLocation = num(draft.NumberOfForcesInLocation) + num(draft.NumberOfSpecialForcesInLocation);
  const pick = useMapPicker({
    targets: () => o?.locations ?? [],
    enabled: toLocation > 0,
    territoryOf: (l) => territoryOf(skin, l),
    faction,
    pin: () => <PinActions label="Revive" validation={validation} cancel={() => pick.clear()} confirm={() => submit()} />,
  });
  useEffect(() => {
    const target = pick.target ?? -1;
    if (o && toLocation > 0 && num(draft._locationId) !== target) void update({ _locationId: target });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pick.target]);
  if (!o) return null;
  return (
    <GameEventLayout type="Revival" header="Select forces and/or leaders to revive" inPopover={inPopover}>
      {o.mayUseSecretAlly && (
        <Checkbox value={draft.UsesEmperorSecretAlly === true} onChange={(v) => update({ UsesEmperorSecretAlly: v })}>
          Use your Nexus card to get 3 additional free revivals beyond your revival limit?
        </Checkbox>
      )}
      {o.maxForces > 0 && (
        <SelectForces value={num(draft.AmountOfForces)} onChange={(v) => update({ AmountOfForces: v })} min={0} max={o.maxForces} faction={faction} special={false} />
      )}
      {o.maxSpecialForces > 0 && (
        <SelectForces value={num(draft.AmountOfSpecialForces)} onChange={(v) => update({ AmountOfSpecialForces: v })} min={0} max={o.maxSpecialForces} faction={faction} special />
      )}
      {o.maxByEmperor > 0 && (
        <>
          <div className="mt-2">You may select up to <SimpleNumber>{o.maxByEmperor}</SimpleNumber> <ForceToken faction={faction} size={20} title={forceName} /> to be revived by your ally:</div>
          {o.maxForces > 0 && (
            <SelectForces value={num(draft.ExtraForcesPaidByEmperor)} onChange={(v) => update({ ExtraForcesPaidByEmperor: v })} min={0} max={o.maxByEmperor} faction={faction} special={false} />
          )}
          {o.maxSpecialForces > 0 && (
            <SelectForces value={num(draft.ExtraSpecialForcesPaidByEmperor)} onChange={(v) => update({ ExtraSpecialForcesPaidByEmperor: v })} min={0} max={o.maxByEmperor} faction={faction} special />
          )}
        </>
      )}
      {o.maxForcesOnPlanet > 0 && (
        <div>
          <SelectForces label="Place forces on the planet?" value={num(draft.NumberOfForcesInLocation)} onChange={(v) => update({ NumberOfForcesInLocation: v })} min={0} max={o.maxForcesOnPlanet} faction={faction} special={false} />
        </div>
      )}
      {o.maxSpecialForcesOnPlanet > 0 && (
        <div>
          <SelectForces label="Place forces on the planet?" value={num(draft.NumberOfSpecialForcesInLocation)} onChange={(v) => update({ NumberOfSpecialForcesInLocation: v })} min={0} max={o.maxSpecialForcesOnPlanet} faction={faction} special />
        </div>
      )}
      {toLocation > 0 && <div className="small">Use the map to choose where to place them.</div>}
      {o.heroes.length > 0 && (
        <SelectFromImage values={heroChoices(skin, view, o.heroes)} value={heroId(draft._heroId)} onChange={(v) => update({ _heroId: v ?? -1 })} required={false} imageWidth={80} />
      )}
      {o.mayAssignSkill && (
        <YesNo value={draft.AssignSkill === true} onChange={(v) => update({ AssignSkill: v })} label="Do you wish to assign a skill to this leader?" />
      )}
      <div className="mt-1 text-end">Revival cost for you: <SimpleNumber>{o.costForPlayer}</SimpleNumber></div>
      {o.costForEmperor != null && (
        <div className="mt-1 text-end">Revival cost for <FactionChip faction="Emperor" />: <SimpleNumber>{o.costForEmperor}</SimpleNumber></div>
      )}
      <ButtonRow confirm={() => submit()} confirmText="Revive" confirmError={validation} />
      <SubmitError error={error} />
    </GameEventLayout>
  );
}

// RaiseDeadComponent.razor.
function RaiseDeadForm({ action, inPopover }: { action: ActionView; inPopover: boolean }) {
  const { skin, view } = useGame();
  const faction = useFaction();
  const forceName = useForceName(faction);
  const { options: o, draft, validation, update, submit, error } = useRevivalDraft(action, "RaiseDeadPlayed");
  const toLocation = num(draft.NumberOfSpecialForcesInLocation);
  const confirmLabel = `Use ${o?.cardName ?? ""}`;
  const pick = useMapPicker({
    targets: () => o?.locations ?? [],
    enabled: toLocation > 0,
    territoryOf: (l) => territoryOf(skin, l),
    faction,
    pin: () => <PinActions label={confirmLabel} validation={validation} cancel={() => pick.clear()} confirm={() => submit()} />,
  });
  useEffect(() => {
    const target = pick.target ?? -1;
    if (o && toLocation > 0 && num(draft._locationId) !== target) void update({ _locationId: target });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pick.target]);
  if (!o) return null;
  const hasHeroes = o.heroes.length > 0;
  return (
    <GameEventLayout type="RaiseDeadPlayed" inPopover={inPopover}
                     header={<>Use {o.cardName} to revive up to 5 <ForceToken faction={faction} size={20} title={forceName} /> or one leader for free</>}>
      {o.maxForces > 0 && (
        <SelectForces value={num(draft.AmountOfForces)} onChange={(v) => update({ AmountOfForces: v })} min={0} max={o.maxForces} faction={faction} special={false} />
      )}
      {o.maxSpecialForces > 0 && (
        <SelectForces value={num(draft.AmountOfSpecialForces)} onChange={(v) => update({ AmountOfSpecialForces: v })} min={0} max={o.maxSpecialForces} faction={faction} special />
      )}
      {o.maxSpecialForcesOnPlanet > 0 && (
        <div>
          <SelectForces label="Place forces on the planet?" value={num(draft.NumberOfSpecialForcesInLocation)} onChange={(v) => update({ NumberOfSpecialForcesInLocation: v })} min={0} max={o.maxSpecialForcesOnPlanet} faction={faction} special />
        </div>
      )}
      {toLocation > 0 && <div className="small">Use the map to choose where to place them.</div>}
      {hasHeroes && (
        <SelectFromImage values={heroChoices(skin, view, o.heroes)} value={heroId(draft._heroId)} onChange={(v) => update({ _heroId: v ?? -1 })} required={false} imageWidth={80} />
      )}
      {o.mayAssignSkill && (
        <YesNo value={draft.AssignSkill === true} onChange={(v) => update({ AssignSkill: v })} label="Do you wish to assign a skill to this leader?" />
      )}
      {!(o.maxForces > 0 || o.maxSpecialForces > 0 || hasHeroes) && <div className="text-info small">You currently have nothing to revive</div>}
      <ButtonRow confirm={() => submit()} confirmText={confirmLabel} confirmError={validation} />
      <SubmitError error={error} />
    </GameEventLayout>
  );
}

// KarmaFreeRevivalComponent.razor.
function KarmaFreeRevivalForm({ action, inPopover }: { action: ActionView; inPopover: boolean }) {
  const { skin, view } = useGame();
  const faction = useFaction();
  const { options: o, draft, validation, update, submit, error } = useRevivalDraft(action, "KarmaFreeRevival");
  if (!o) return null;
  return (
    <GameEventLayout type="KarmaFreeRevival" header={`Use ${o.cardName} to revive forces or a leader`} inPopover={inPopover}>
      {o.maxForces > 0 && (
        <SelectForces value={num(draft.AmountOfForces)} onChange={(v) => update({ AmountOfForces: v })} min={0} max={o.maxForces} faction={faction} special={false} />
      )}
      {o.maxSpecialForces > 0 && (
        <SelectForces value={num(draft.AmountOfSpecialForces)} onChange={(v) => update({ AmountOfSpecialForces: v })} min={0} max={o.maxSpecialForces} faction={faction} special />
      )}
      <SelectFromImage values={heroChoices(skin, view, o.heroes)} value={heroId(draft._heroId)} onChange={(v) => update({ _heroId: v ?? -1 })} required={false} imageWidth={80} />
      {o.mayAssignSkill && (
        <YesNo value={draft.AssignSkill === true} onChange={(v) => update({ AssignSkill: v })} label="Do you wish to assign a skill to this leader?" />
      )}
      <ButtonRow confirm={() => submit()} confirmText={`Use ${o.cardName}`} confirmError={validation} />
      <SubmitError error={error} />
    </GameEventLayout>
  );
}

// KarmaRevivalPreventionComponent.razor.
function KarmaRevivalPreventionForm({ action, inPopover }: { action: ActionView; inPopover: boolean }) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useRevivalDraft(action, "KarmaRevivalPrevention");
  if (!o) return null;
  return (
    <GameEventLayout type="KarmaRevivalPrevention" header={`Use ${o.cardName} to prevent revival`} inPopover={inPopover}>
      <SelectFromImage values={factionChoices(skin, o.targets)} value={num(draft.Target)} onChange={(v) => update({ Target: v ?? 0 })} imageWidth={40} label="Target" />
      <ButtonRow confirm={() => submit()} confirmText="Prevent Revival" confirmError={validation} />
      <SubmitError error={error} />
    </GameEventLayout>
  );
}

// ChoamFreeRevivalPreventionComponent.razor.
function ChoamFreeRevivalPreventionForm({ action, inPopover }: { action: ActionView; inPopover: boolean }) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useRevivalDraft(action, "ChoamFreeRevivalPrevention");
  if (!o) return null;
  return (
    <GameEventLayout type="ChoamFreeRevivalPrevention" inPopover={inPopover}
                     header={<>Use {o.card ? <RichText parts={o.card} /> : "Nexus Cunning"} to prevent a faction from taking Free Revival?</>}>
      <SelectFromImage values={factionChoices(skin, o.targets)} value={num(draft.Target)} onChange={(v) => update({ Target: v ?? 0 })} imageWidth={40} />
      <ButtonRow confirm={() => submit()} confirmText="Prevent Free Revival" confirmError={validation} />
      <SubmitError error={error} />
    </GameEventLayout>
  );
}

// RequestTleilaxuRevivalComponent.razor.
function RequestTleilaxuRevivalForm({ action, inPopover }: { action: ActionView; inPopover: boolean }) {
  const { skin, view } = useGame();
  const { options: o, draft, validation, update, submit, error } = useRevivalDraft(action, "RequestTleilaxuRevival");
  if (!o) return null;
  return (
    <GameEventLayout type="RequestTleilaxuRevival" inPopover={inPopover}
                     header={<>Request <FactionChip faction="Tleilaxu" /> Revival of one of your leaders?</>}>
      <SelectFromImage values={heroChoices(skin, view, o.heroes)} value={heroId(draft._heroId)} onChange={(v) => update({ _heroId: v ?? -1 })} imageWidth={80} />
      <ButtonRow confirm={() => submit()} confirmText="Request" confirmError={validation} />
      <SubmitError error={error} />
    </GameEventLayout>
  );
}

// AcceptOrCancelTleilaxuRevivalComponent.razor.
function AcceptOrCancelTleilaxuRevivalForm({ action, inPopover }: { action: ActionView; inPopover: boolean }) {
  const { skin, view } = useGame();
  const { options: o, draft, validation, update, submit, error } = useRevivalDraft(action, "AcceptOrCancelTleilaxuRevival");
  if (!o) return null;
  const hero = heroId(draft._heroId) ?? -1;
  const base = { $type: action.eventType };
  return (
    <GameEventLayout type="AcceptOrCancelTleilaxuRevival" header="Manage Revival Requests" urgent={o.requests.length > 0} inPopover={inPopover}>
      {o.requests.length > 0 && (
        <>
          {o.botAwaits && <div className="text-warning mb-1">Bots are awaiting your decision before starting to revive...</div>}
          <div>Early revival requests:</div>
          <SelectFromImage values={heroChoices(skin, view, o.requests)} value={heroId(draft._heroId)} onChange={(v) => update({ _heroId: v ?? -1 })} imageWidth={80} />
          <SelectResources value={num(draft.Price)} onChange={(v) => update({ Price: v })} min={0} max={50} label="Price" />
          <ButtonRow alignment=""
                     pass={() => submit({ ...base, Cancel: false, _heroId: -1 })} passText="Deny all"
                     confirm={() => submit()} confirmText={o.offerLabel} confirmError={validation}
                     other={() => submit({ ...base, Cancel: false, _heroId: hero, Price: MAX_INT })} otherText={o.denyLabel} />
        </>
      )}
      {o.offers.length > 0 && (
        <>
          {o.requests.length > 0 && <hr className="hr-hairline" />}
          <div>Existing early revival offers:</div>
          <table className="table table-sm">
            <tbody>
              {o.offers.map((offer) => (
                <tr key={offer.hero.id} className="table-dark">
                  <td><Image src={heroImage(skin, offer.hero.hero)} alt="" width={32} /></td>
                  <td>{offer.price == null ? <span>Denied</span> : <SimpleNumber size={22}>{offer.price}</SimpleNumber>}</td>
                  <td>
                    <button className="btn btn-primary btn-sm" onClick={() => submit({ ...base, Cancel: true, _heroId: offer.hero.id })}>Cancel</button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}
      <SubmitError error={error} />
    </GameEventLayout>
  );
}

// SetIncreasedRevivalLimitsComponent.razor: a multiple choice; a click toggles a faction,
// shift-click picks only that one.
function SetIncreasedRevivalLimitsForm({ action, inPopover }: { action: ActionView; inPopover: boolean }) {
  const { skin } = useGame();
  const { options: o, draft, validation, update, submit, error } = useRevivalDraft(action, "SetIncreasedRevivalLimits");
  if (!o) return null;
  const selected = Array.isArray(draft.Factions) ? (draft.Factions as number[]) : [];
  const click = (value: number, shift: boolean) => {
    const next = shift ? [value] : selected.includes(value) ? selected.filter((f) => f !== value) : [...selected, value];
    void update({ Factions: next });
  };
  return (
    <GameEventLayout type="SetIncreasedRevivalLimits" header="Which factions may have their revival limit increased to 5?" inPopover={inPopover}>
      <div className="img-choices">
        {factionChoices(skin, o.targets).map((c) => (
          // Image's onClick gets React's click event, for the shift key.
          <Image key={c.value} src={c.src} alt={c.alt} width={40} tooltip={c.alt} pointer highlightOnHover darkened={!selected.includes(c.value)}
                 onClick={(e?: MouseEvent) => click(c.value, e?.shiftKey === true)} />
        ))}
      </div>
      <ButtonRow confirm={() => submit()} confirmText="Set increased limits" confirmError={validation} />
      <SubmitError error={error} />
    </GameEventLayout>
  );
}

// ActionPanel.razor's cases for the revival events. ActionPanel leaves out events played from a
// card in hand (CardPlays.PlayedFromHand); the hand's popover draws them with `inPopover`.
export function RevivalForm({ action, inPopover = false }: { action: ActionView; inPopover?: boolean }) {
  const props = { action, inPopover };
  switch (action.eventType) {
    case "Revival": return <RevivalEventForm {...props} />;
    case "RaiseDeadPlayed": return <RaiseDeadForm {...props} />;
    case "KarmaFreeRevival": return <KarmaFreeRevivalForm {...props} />;
    case "KarmaRevivalPrevention": return <KarmaRevivalPreventionForm {...props} />;
    case "ChoamFreeRevivalPrevention": return <ChoamFreeRevivalPreventionForm {...props} />;
    case "RequestTleilaxuRevival": return <RequestTleilaxuRevivalForm {...props} />;
    case "AcceptOrCancelTleilaxuRevival": return <AcceptOrCancelTleilaxuRevivalForm {...props} />;
    case "SetIncreasedRevivalLimits": return <SetIncreasedRevivalLimitsForm {...props} />;
    default: return null;
  }
}

// For ActionPanel's dispatch: the form of a revival event, or null for other events.
export function revivalForm(action: ActionView, inPopover = false) {
  return revivalForms.has(action.eventType) ? <RevivalForm key={action.eventType} action={action} inPopover={inPopover} /> : null;
}
