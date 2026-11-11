// OtherComponents/MentatComponent.razor: the menu's "List of Cards", a modal with every card and
// token this game uses. The cards in play come from the view (`mentat`); the other sections
// follow from the rules and the factions playing.
import type { Ambassador, DiscoveryToken, Faction, LeaderSkill, Rule, StrongholdAdvantage, TerrorType } from "../generated/enums";
import { ambassadorPopup, leaderSkillPopup, resourceCardPopup, treacheryCardPopup, withSpiceIcons } from "../popups";
import type { Skin } from "../skin";
import { htmlEncode } from "../text";
import { useGame } from "../state/context";
import { FactionChip } from "./FactionChip";
import { Hoverable } from "./Hoverable";
import { Image } from "./Image";
import { ModalComponent } from "./ModalComponent";

// Enumerations.GetValuesExceptDefault, in the C# enums' order.
const LEADER_SKILLS: LeaderSkill[] = ["Bureaucrat", "Diplomat", "Decipherer", "Smuggler", "Graduate", "Planetologist", "Warmaster", "Adept", "Swordmaster", "KillerMedic", "MasterOfAssassins", "Sandmaster", "Thinker", "Banker"];
const ADVANTAGES: StrongholdAdvantage[] = ["FreeResourcesForBattles", "CollectResourcesForUseless", "CountDefensesAsAntidote", "WinTies", "CollectResourcesForDial", "AnyOtherAdvantage"];
// Nexus: one card per faction.
const NEXUS: Faction[] = ["Fremen", "Atreides", "Harkonnen", "Emperor", "Guild", "BeneGesserit", "Ixian", "Tleilaxu", "Choam", "Richese", "Ecaz", "Moritani"];
const AMBASSADORS: Ambassador[] = ["Fremen", "Atreides", "Harkonnen", "Emperor", "Guild", "BeneGesserit", "Ixian", "Tleilaxu", "Choam", "Richese", "Ecaz", "Moritani"];
const TERROR: TerrorType[] = ["Assassination", "Atomics", "Extortion", "Robbery", "Sabotage", "SneakAttack"];
const DISCOVERIES: DiscoveryToken[] = ["Jacurutu", "Shrine", "TestingStation", "Cistern", "ProcessingStation", "CardStash", "ResourceStash", "Flight"];

// Skin.GetFramedCardHtml and GetImageHoverHtml.
const framedCard = (url: string) => `<div class='pop-art'><img src='${url}'/></div>`;
const imageHover = (url: string) => `<img src='${url}' width=300 class='img-fluid' style='filter:drop-shadow(-3px 3px 2px black);'/>`;

// Skin.Describe(ResourceCard).
function resourceCardName(skin: Skin, skinId: number): string {
  const c = skin.resourceCardInfo(skinId);
  if (!c) return "?";
  if (c.isShaiHulud) return skin.concept("Monster");
  if (c.isGreatMaker) return skin.concept("GreatMonster");
  if (c.isSandTrout) return skin.concept("BabyMonster");
  const location = c.location ?? c.discoveryLocation;
  const territory = location != null ? skin.locationInfo(location)?.territory : undefined;
  return territory != null ? skin.territory(territory) : "?";
}

// A token's line: its name in bold, then its description with spice icons (WithSpiceIcons).
const tokenLine = (skin: Skin, name: string, description: string) =>
  `<strong>${htmlEncode(name)} -</strong> ${withSpiceIcons(skin, description)}`;

function Card({ popover, src, alt }: { popover: string; src: string; alt: string }) {
  return (
    <div className="card">
      <Hoverable className="card-img-top" popover={popover}>
        <img src={src} className="card-img" alt={alt} />
      </Hoverable>
    </div>
  );
}

export function MentatComponent({ id }: { id: string }) {
  const { view, skin } = useGame();
  const m = view.mentat;
  const applicable = (r: Rule) => view.rules.includes(r) || view.rulesForBots.includes(r);
  const isPlaying = (f: Faction) => view.players.some((p) => p.faction === f);
  return (
    <ModalComponent id={id} title="Card and Token Reference" size="modal-xl">
      <h5>Treachery Cards</h5>
      <div className="row row-cols-3 row-cols-md-6 ms-1 me-1">
        {m.treacheryCards.map((c) => <Card key={c} popover={treacheryCardPopup(skin, c)} src={skin.get("TreacheryCardImage_URL", skin.cardSkinId(c))} alt={skin.card(c)} />)}
      </div>

      {isPlaying("Richese") && (
        <>
          <h5 className="mt-2"><FactionChip faction="Richese" /> Cards</h5>
          <div className="row row-cols-3 row-cols-md-6 ms-1 me-1">
            {m.richeseCards.map((c) => <Card key={c} popover={treacheryCardPopup(skin, c)} src={skin.get("TreacheryCardImage_URL", skin.cardSkinId(c))} alt={skin.card(c)} />)}
          </div>
        </>
      )}

      <h5 className="mt-2">{skin.concept("Resource")} Cards</h5>
      <div className="row row-cols-3 row-cols-md-6 ms-1 me-1">
        {m.resourceCards.map((c) => <Card key={c} popover={resourceCardPopup(skin, c)} src={skin.get("ResourceCardImage_URL", c)} alt={resourceCardName(skin, c)} />)}
      </div>

      {applicable("LeaderSkills") && (
        <>
          <h5 className="mt-2">Leader Skill Cards</h5>
          <div className="row row-cols-3 row-cols-md-6 ms-1 me-1">
            {LEADER_SKILLS.map((s) => <Card key={s} popover={leaderSkillPopup(skin, s)} src={skin.get("LeaderSkillCardImage_URL", s)} alt={skin.get("LeaderSkillCardName_STR", s)} />)}
          </div>
        </>
      )}

      {applicable("StrongholdBonus") && (
        <>
          <h5 className="mt-2">Stronghold Cards</h5>
          <div className="row row-cols-3 row-cols-md-6 ms-1 me-1">
            {ADVANTAGES.map((a) => <Card key={a} popover={imageHover(skin.get("StrongholdCardImage_URL", a))} src={skin.get("StrongholdCardImage_URL", a)} alt={skin.get("StrongholdCardName_STR", a)} />)}
          </div>
        </>
      )}

      {applicable("NexusCards") && (
        <>
          <h5 className="mt-2">Nexus Cards</h5>
          <div className="row row-cols-3 row-cols-md-6 ms-1 me-1">
            {NEXUS.map((n) => <Card key={n} popover={framedCard(skin.get("NexusCardImage_URL", n))} src={skin.get("NexusCardImage_URL", n)} alt={n} />)}
          </div>
        </>
      )}

      {applicable("Homeworlds") && (
        <>
          <h5 className="mt-2">Homeworld Cards</h5>
          <div className="row row-cols-2 row-cols-md-3 ms-1 me-1">
            {skin.statics.homeworlds.map((w) => (
              <Card key={w.world} popover={framedCard(skin.get("HomeWorldCardImage_URL", w.world))} src={skin.get("HomeWorldCardImage_URL", w.world)} alt={skin.location(w.location)} />
            ))}
          </div>
        </>
      )}

      {isPlaying("Ecaz") && (
        <>
          <h5 className="mt-2">Ambassador Tokens</h5>
          <div className="row row-cols-6 row-cols-md-12 ms-1 me-1">
            {AMBASSADORS.filter((a) => a !== "Moritani").map((a) => (
              <div key={a} className="card">
                <Image src={skin.url(skin.get("AmbassadorImage_URL", a))} alt={skin.get("AmbassadorName_STR", a)} width={0} className="card-img" popover={ambassadorPopup(skin, a)} />
              </div>
            ))}
          </div>
        </>
      )}

      {isPlaying("Moritani") && (
        <>
          <h5 className="mt-2">Terror Tokens</h5>
          <div>
            <ul>
              {TERROR.map((t) => (
                <li key={t} dangerouslySetInnerHTML={{ __html: tokenLine(skin, skin.get("TerrorTokenName_STR", t), skin.get("TerrorTokenDescription_STR", t)) }} />
              ))}
            </ul>
          </div>
        </>
      )}

      {applicable("DiscoveryTokens") && (
        <>
          <h5 className="mt-2">Discovery Tokens</h5>
          <div>
            <ul>
              {DISCOVERIES.map((d) => (
                <li key={d} dangerouslySetInnerHTML={{ __html: tokenLine(skin, skin.get("DiscoveryTokenName_STR", d), skin.get("DiscoveryTokenDescription_STR", d)) }} />
              ))}
            </ul>
          </div>
        </>
      )}
    </ModalComponent>
  );
}
