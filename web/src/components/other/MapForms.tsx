// The "other events" forms that pick on the map (package 7e): KarmaMonsterComponent and
// NexusPlayedComponent, both MapPickComponents. The confirm button of a map pick sits in the panel
// pinned at the chosen place (PinActions).
import { Fragment, useEffect } from "react";
import type { Part } from "../../generated/Part";
import { useGame } from "../../state/context";
import { useMapPicker } from "../board/mapInteraction";
import { ButtonRow } from "../ButtonRow";
import { FactionChip } from "../FactionChip";
import { MonsterIcon } from "../MonsterIcon";
import { heroChoices, PinActions, SelectForces, YesNo } from "../revival/parts";
import { RichText } from "../RichText";
import { SelectFromImage } from "../SelectFromImage";
import { SimpleNumber } from "../SimpleNumber";
import type { FormProps } from "./ChoiceForms";
import type { PrescienceAspect } from "../../generated/enums";
import { cardChoices, cardType, factionChoices, factionOf, idOf, Layout, nameOf, num, Radio, SubmitError, useFaction, useOtherDraft } from "./parts";

function useTerritoryOf() {
  const { skin } = useGame();
  return (l: number) => skin.locationInfo(l)?.territory ?? -1;
}

// KarmaMonsterComponent: Shai-Hulud appears in a territory picked on the map.
export function KarmaMonsterForm({ action, inPopover = false }: FormProps) {
  const { skin } = useGame();
  const faction = useFaction();
  const territoryOf = useTerritoryOf();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "KarmaMonster");
  const karma = cardType(skin, "Karma");
  const pick = useMapPicker({
    targets: () => o?.locations ?? [],
    byTerritory: true,
    territoryOf,
    faction,
    pin: () => <PinActions label={`Use ${karma}`} validation={validation} cancel={() => pick.clear()} confirm={() => submit()} />,
  });
  useEffect(() => {
    const t = pick.target != null ? territoryOf(pick.target) : null;
    if (o && idOf(draft.territory) !== t) void update({ territory: t });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pick.target]);
  if (!o) return null;
  return (
    <Layout type="KarmaMonster" header={<>Use {karma} to make <MonsterIcon /> appear in a territory of choice</>} inPopover={inPopover}>
      <div className="small">Use the map to choose a territory.</div>
      <SubmitError error={error} />
    </Layout>
  );
}

// The line of a Nexus case. The razor writes its words as bare text between the chips, so text
// parts are not wrapped in RichText's spans.
function NexusText({ parts }: { parts: Part[] }) {
  return <>{parts.map((p, i) => (p.t === "text" ? <Fragment key={i}>{p.v}</Fragment> : <RichText key={i} parts={[p]} />))}</>;
}

// NexusPlayedComponent: the line for the card and the kind of use, the inputs of that case, and the
// map pick for Tleilaxu's revived forces, Ecaz's territory and Moritani's stronghold.
export function NexusPlayedForm({ action, inPopover = false }: FormProps) {
  const { skin, view } = useGame();
  const faction = useFaction();
  const territoryOf = useTerritoryOf();
  const { options: o, draft, validation, update, submit, error } = useOtherDraft(action, "NexusPlayed");
  const member = o?.mapMember ?? null;
  const pick = useMapPicker({
    targets: () => o?.locations ?? [],
    enabled: member != null,
    byTerritory: member === "EcazTerritory" || member === "MoritaniTerritory",
    territoryOf,
    faction,
    pin: () => <PinActions label="Confirm" validation={validation} cancel={() => pick.clear()} confirm={() => submit()} />,
  });
  useEffect(() => {
    if (!o || member == null) return;
    const field = member === "TleilaxuLocation" ? "tleilaxuLocation" : member === "EcazTerritory" ? "ecazTerritory" : "moritaniTerritory";
    const value = pick.target == null ? null : member === "TleilaxuLocation" ? pick.target : territoryOf(pick.target);
    if (idOf(draft[field]) !== value) void update({ [field]: value });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pick.target, member]);
  if (!o) return null;
  const t = o.tleilaxu;
  const hero = idOf(draft.tleilaxuHero);
  return (
    <Layout type="NexusPlayed" header={<>Nexus: Play <FactionChip faction={o.nexus} /> {o.kind}</>} urgent inPopover={inPopover}>
      {o.aspects && (
        <Radio values={o.aspects.map((a) => ({ value: a.name as PrescienceAspect, label: a.label }))} value={nameOf(draft.atreidesPrescienceAspect) as PrescienceAspect}
               onChange={(v) => update({ atreidesPrescienceAspect: v })} label="What should be foreseen?" />
      )}
      {o.text && <div><NexusText parts={o.text} /></div>}
      {t && (
        <>
          {t.maxForces > 0 && (
            <SelectForces value={num(draft.tleilaxuForces)} onChange={(v) => update({ tleilaxuForces: v })} min={0} max={t.maxForces} faction={faction} special={false} />
          )}
          {t.maxSpecialForces > 0 && (
            <SelectForces value={num(draft.tleilaxuSpecialForces)} onChange={(v) => update({ tleilaxuSpecialForces: v })} min={0} max={t.maxSpecialForces} faction={faction} special />
          )}
          {t.maxSpecialForcesOnPlanet > 0 && (
            <div>
              <SelectForces label="Place forces on the planet?" value={num(draft.tleilaxuNumberOfSpecialForcesInLocation)} min={0} max={t.maxSpecialForcesOnPlanet}
                            onChange={(v) => update({ tleilaxuNumberOfSpecialForcesInLocation: v })} faction={faction} special />
            </div>
          )}
          {num(draft.tleilaxuNumberOfSpecialForcesInLocation) > 0 && <div className="small">Use the map to choose where to place them.</div>}
          <SelectFromImage values={heroChoices(skin, view, t.heroes)} value={hero} required={false}
                           onChange={(v) => update({ tleilaxuHero: v })} imageWidth={80} />
          {hero != null && t.mayAssignSkill && (
            <YesNo value={draft.tleilaxuAssignSkill === true} onChange={(v) => update({ tleilaxuAssignSkill: v })} label="Do you wish to assign a skill to this leader?" />
          )}
          <div className="mt-1 text-end">Revival cost for you: <SimpleNumber>{t.cost}</SimpleNumber></div>
        </>
      )}
      {o.choamCards && (
        <SelectFromImage values={cardChoices(skin, o.choamCards)} value={idOf(draft.choamCard)} onChange={(v) => update({ choamCard: v })} imageWidth={80} />
      )}
      {member === "EcazTerritory" && <div className="small">Use the map to choose a territory.</div>}
      {o.ecazFactions && (
        <SelectFromImage values={factionChoices(skin, o.ecazFactions)} value={factionOf(draft.ecazFaction)} onChange={(v) => update({ ecazFaction: v })} imageWidth={40} />
      )}
      {member === "MoritaniTerritory" && <div className="small">Use the map to choose a stronghold.</div>}
      <ButtonRow confirm={() => submit()} confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}
