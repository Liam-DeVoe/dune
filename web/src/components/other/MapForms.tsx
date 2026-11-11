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
import { cardChoices, cardType, factionChoices, Layout, num, Radio, SubmitError, useFaction, useOtherDraft } from "./parts";

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
    const t = pick.target != null ? territoryOf(pick.target) : -1;
    if (o && num(draft._territoryId) !== t) void update({ _territoryId: t });
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
    const field = member === "TleilaxuLocation" ? "_tleilaxuLocationId" : member === "EcazTerritory" ? "_ecazTerritoryId" : "_moritaniTerritoryId";
    const value = pick.target == null ? -1 : member === "TleilaxuLocation" ? pick.target : territoryOf(pick.target);
    if (num(draft[field]) !== value) void update({ [field]: value });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pick.target, member]);
  if (!o) return null;
  const t = o.tleilaxu;
  const hero = num(draft._tleilaxuHeroId);
  return (
    <Layout type="NexusPlayed" header={<>Nexus: Play <FactionChip faction={o.nexus} /> {o.kind}</>} urgent inPopover={inPopover}>
      {o.aspects && (
        <Radio values={o.aspects.map((a) => ({ value: a.value, label: a.label }))} value={num(draft.AtreidesPrescienceAspect)}
               onChange={(v) => update({ AtreidesPrescienceAspect: v })} label="What should be foreseen?" />
      )}
      {o.text && <div><NexusText parts={o.text} /></div>}
      {t && (
        <>
          {t.maxForces > 0 && (
            <SelectForces value={num(draft.TleilaxuForces)} onChange={(v) => update({ TleilaxuForces: v })} min={0} max={t.maxForces} faction={faction} special={false} />
          )}
          {t.maxSpecialForces > 0 && (
            <SelectForces value={num(draft.TleilaxuSpecialForces)} onChange={(v) => update({ TleilaxuSpecialForces: v })} min={0} max={t.maxSpecialForces} faction={faction} special />
          )}
          {t.maxSpecialForcesOnPlanet > 0 && (
            <div>
              <SelectForces label="Place forces on the planet?" value={num(draft.TleilaxuNumberOfSpecialForcesInLocation)} min={0} max={t.maxSpecialForcesOnPlanet}
                            onChange={(v) => update({ TleilaxuNumberOfSpecialForcesInLocation: v })} faction={faction} special />
            </div>
          )}
          {num(draft.TleilaxuNumberOfSpecialForcesInLocation) > 0 && <div className="small">Use the map to choose where to place them.</div>}
          <SelectFromImage values={heroChoices(skin, view, t.heroes)} value={hero >= 0 ? hero : null} required={false}
                           onChange={(v) => update({ _tleilaxuHeroId: v ?? -1 })} imageWidth={80} />
          {hero >= 0 && t.mayAssignSkill && (
            <YesNo value={draft.TleilaxuAssignSkill === true} onChange={(v) => update({ TleilaxuAssignSkill: v })} label="Do you wish to assign a skill to this leader?" />
          )}
          <div className="mt-1 text-end">Revival cost for you: <SimpleNumber>{t.cost}</SimpleNumber></div>
        </>
      )}
      {o.choamCards && (
        <SelectFromImage values={cardChoices(skin, o.choamCards)} value={num(draft._choamCardId)} onChange={(v) => update({ _choamCardId: v ?? -1 })} imageWidth={80} />
      )}
      {member === "EcazTerritory" && <div className="small">Use the map to choose a territory.</div>}
      {o.ecazFactions && (
        <SelectFromImage values={factionChoices(skin, o.ecazFactions)} value={num(draft.EcazFaction)} onChange={(v) => update({ EcazFaction: v ?? 0 })} imageWidth={40} />
      )}
      {member === "MoritaniTerritory" && <div className="small">Use the map to choose a stronghold.</div>}
      <ButtonRow confirm={() => submit()} confirmError={validation} />
      <SubmitError error={error} />
    </Layout>
  );
}
