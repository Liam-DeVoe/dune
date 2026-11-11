// MapComponent.Intel: the popover of a location on the map (MapComponents/MapComponent.razor).
import * as shared from "../../popups";
import type { Board } from "./geometry";
import { forceTokenHtml } from "./primitives";
import { strongholdAdvantage } from "./popups";

const STORM_GLYPH =
  "<svg viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round'>" +
  "<path d='M3 7h9.5a2.5 2.5 0 1 0-2.5-2.5'/><path d='M3 12h15a2.5 2.5 0 1 1-2.5 2.5'/><path d='M3 17h7a2 2 0 1 1-2 2'/></svg>";

function feature(title: string, body: string, icon = ""): string {
  return `<div class='feat'><div class='ic'>${icon}</div><div><b>${title}</b><span>${body}</span></div></div>`;
}

function hazard(protectedFrom: boolean, icon: string): string {
  const classes = protectedFrom ? "chip hazard protected" : "chip hazard exposed";
  const word = protectedFrom ? "Protected" : "Exposed";
  return `<span class='${classes}'>${icon}<span class='name'>${word}</span></span>`;
}

export function locationIntel(b: Board, id: number): string {
  const skin = b.skin;
  const v = b.view;
  const board = v.board;
  const map = skin.statics.map;
  const l = b.loc(id);
  const t = b.terr(l.territory);
  let result = "<div style='width:300px'>";
  const owner = board.strongholds.find((s) => s.location === id)?.owner;
  if (owner !== undefined) {
    result += strongholdAdvantage(b, t.advantage, owner);
  } else {
    const sector = l.orientation !== "" ? ` <span class='sector'>${l.orientation}</span>` : "";
    result += `<h5>${skin.territory(l.territory)}${sector}</h5>`;
  }
  if (b.d.ShowVerboseToolipsOnMap) {
    if (b.sector(id) === board.sectorInStorm) result += `<div class='in-storm'>${STORM_GLYPH}<span>In storm</span></div>`;
    result += "<div class='feats'>";
    if (id === map.polarSink) result += feature("Safe haven", "No battles are fought here.");
    if (t.isStronghold || board.specialStrongholds.includes(l.territory)) result += feature("Stronghold", "Yields a victory point at end of turn.");
    if (l.kind === "discovered" && l.discovery) {
      result += feature(skin.get("DiscoveryTokenName_STR", l.discovery), shared.withSpiceIcons(skin, skin.get("DiscoveryTokenDescription_STR", l.discovery) + "."));
    }
    if (board.atomicsAftermath === l.territory) result += feature("Atomics aftermath", "No shipping here.");
    if (id === map.arrakeen || id === map.carthag) {
      result += feature("Harvesters", `When occupied, collect ${shared.spiceHtml(skin, "3")} per force.`);
      result += feature("Ornithopters", "When occupied, your forces move up to 3 territories.");
    }
    const resource = skin.concept("Resource");
    if (b.applicable("IncreasedResourceFlow") && (id === map.arrakeen || id === map.carthag || id === map.tueksSietch)) {
      const bonus = id === map.arrakeen || id === map.carthag ? 2 : 1;
      result += feature(`${resource} flow`, `Yields ${shared.spiceHtml(skin, String(bonus), 18)} at end of turn when occupied.`);
    }
    if (l.spiceBlowAmount > 0) {
      result += feature(`${resource} blow`, `May contain ${shared.spiceHtml(skin, String(l.spiceBlowAmount), 18)} after ${skin.enumValue("MainPhase", "Blow")}.`);
    }
    result += "</div><div class='hazards'>";
    result += hazard(t.isProtectedFromWorm, `<img src='${b.s("Monster_URL")}' />`);
    result += hazard(board.protectedFromStorm.includes(id), STORM_GLYPH);
    result += "</div>";
  }
  result += spiceAndForceAndTokenInfo(b, id);
  return result + "</div>";
}

function spiceAndForceAndTokenInfo(b: Board, id: number): string {
  const skin = b.skin;
  const board = b.view.board;
  const territory = b.territoryOf(id);
  let result = "<div class=\"row m-0 bg-dark d-flex justify-content-center\">";
  const spice = board.spice.find((s) => s.location === id);
  if (spice) result += `<div class="col col-auto">${shared.spiceHtml(skin, String(spice.amount), 32)}</div>`;
  for (const bt of board.forces.filter((x) => x.location === id)) {
    if (bt.forces > 0) {
      result += `<div class="col col-auto">${forceTokenHtml(skin.get("FactionForceImage_URL", bt.faction), bt.faction, false, 32)}<strong>&nbsp;${bt.forces}</strong></div>`;
    }
    if (bt.specialForces > 0) {
      const me = b.view.viewer.faction;
      // C# parity: the conditional binds before the concatenation, so Richese seen by anyone else reads "?".
      const amount = bt.faction === "Richese" && me !== "Richese" ? "?" : String(bt.faction === "Richese" ? board.noFieldValue : bt.specialForces);
      result += `<div class="col col-auto">${forceTokenHtml(skin.get("FactionSpecialForceImage_URL", bt.faction), bt.faction, true, 32)}<strong>&nbsp;${amount}</strong></div>`;
    }
  }
  const terror = board.terror.filter((x) => x.territory === territory).length;
  if (terror > 0) {
    result += `<div class="col col-auto"><img class="img-fluid m-0 p-0" width="32" height="32" src="${skin.get("FactionImage_URL", "Moritani")}" style="filter:hue-rotate(180deg);" /><strong>&nbsp;${terror}</strong></div>`;
  }
  const ambassador = board.ambassadors.find((a) => a.territory === territory);
  if (ambassador) result += `<div class="col col-auto"><img class="img-fluid m-0 p-0" width="32" height="32" src="${skin.get("AmbassadorImage_URL", ambassador.ambassador)}" /></div>`;
  const discovery = board.discoveries.find((d) => d.location === id);
  if (discovery) result += `<div class="col col-auto"><img class="img-fluid m-0 p-0" width="32" height="32" src="${skin.get("DiscoveryTokenTypeImage_URL", discovery.tokenType)}" /></div>`;
  for (const h of board.leaders.filter((x) => x.territory === territory)) {
    const src = h.hero.t === "leader" ? skin.get("PersonImage_URL", h.hero.id) : h.hero.t === "messiah" ? b.s("Messiah_URL") : skin.get("TreacheryCardImage_URL", skin.cardSkinId(h.hero.id));
    result += `<div class="col col-auto"><img class="img-fluid m-0 p-0" width="64" height="64" src="${src}" /></div>`;
  }
  return result + "</div>";
}
