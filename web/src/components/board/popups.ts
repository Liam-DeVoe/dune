// The map's popover markup. Most of it is Skin.GetPopup, ported in src/popups.ts; this file adds
// what only the map shows: the faction on a seat (Support.FactionPopupHtml, short form), the
// stronghold advantage with its owner, homeworld cards, the discard piles and Richese's cache
// (CardPiles.razor), the tanks (Tanks.razor PanelHtml) and the turn (Phases.razor TurnPopover).
import type { Ambassador, DiscoveryToken, Faction, LeaderSkill, StrongholdAdvantage, TechToken, TerrorType, World } from "../../generated/enums";
import type { HeroRef } from "../../generated/HeroRef";
import type { PublicPlayer } from "../../generated/PublicPlayer";
import * as shared from "../../popups";
import { htmlEncode } from "../../text";
import type { Board } from "./geometry";
import { forceTokenHtml } from "./primitives";

const imageHover = (url: string) => `<img src='${url}' width=300 class='img-fluid' style='filter:drop-shadow(-3px 3px 2px black);'/>`;

export function treacheryCard(b: Board, id: number): string { return shared.treacheryCardPopup(b.skin, id); }
export function leaderSkill(b: Board, s: LeaderSkill): string { return shared.leaderSkillPopup(b.skin, s); }
export function techToken(b: Board, t: TechToken): string { return shared.techTokenPopup(b.skin, t); }
export function terror(b: Board, t: TerrorType): string { return shared.terrorPopup(b.skin, t); }
export function ambassador(b: Board, a: Ambassador): string { return shared.ambassadorPopup(b.skin, a); }
export function discovery(b: Board, d: DiscoveryToken): string { return shared.discoveryTokenPopup(b.skin, d); }
export function resourceCard(b: Board, skinId: number): string { return shared.resourceCardPopup(b.skin, skinId); }

// Skin.GetPopup(Leader, LeaderSkill).
export function skilledLeader(b: Board, leader: number, skill: LeaderSkill): string { return shared.leaderPopup(b.skin, leader, skill); }

// Skin.GetPopup(IHero, Game): a skilled leader shows its skill card.
export function hero(b: Board, h: HeroRef): string {
  if (h.t === "treacheryCard") return shared.treacheryCardPopup(b.skin, h.id);
  if (h.t === "messiah") return shared.messiahPopup(b.skin);
  const skill = b.view.players.flatMap((p) => p.skilledLeaders).find((s) => s.id === h.id)?.skill ?? null;
  return shared.leaderPopup(b.skin, h.id, skill);
}

// Skin.GetPopup(StrongholdAdvantage, Faction).
export function strongholdAdvantage(b: Board, adv: StrongholdAdvantage, owner: Faction): string {
  return `<div style='position:relative'><img style='position:relative;filter:drop-shadow(-3px 3px 2px black);' src='${b.skin.get("StrongholdCardImage_URL", adv)}' width=300/><img src='${b.skin.get("FactionImage_URL", owner)}' width=100 style='position:absolute;left:220px;top:40px;filter:drop-shadow(-3px 3px 2px black);'/></div>`;
}

// Skin.GetPopup(Homeworld): the homeworld's card.
export function homeworld(b: Board, w: World): string {
  return `<div class='pop-art'><img src='${b.skin.get("HomeWorldCardImage_URL", w)}'/></div>`;
}

// Support.FactionPopupHtml: the faction, who plays it, its leaders, the summary of its rules
// sheet, and the full sheet (shown while Shift is held). The sheet's texts come from the server's
// skin (`factionSheets`); a faction without a sheet has no popover.
export function faction(b: Board, p: PublicPlayer): string {
  const sheet = b.view.factionSheets.find((s) => s.faction === p.faction);
  if (!sheet) return "";
  const skin = b.skin;
  let html = "<div class=\"pop-faction\">";
  html += `<div class="pop-faction-head"><img src="${skin.get("FactionImage_URL", p.faction)}" alt=""/>`;
  html += `<div><h6>${skin.faction(p.faction)}</h6><div class="who">${p.name}</div></div></div>`;
  html += "<div class=\"pop-faction-brief\">";
  html += "<div class=\"pop-faction-leaders\">";
  for (const l of skin.statics.leaders.filter((l) => l.faction === p.faction)) html += `<img src="${skin.get("PersonImage_URL", l.id)}" alt=""/>`;
  html += "</div>";
  const atStart = shared.withSpiceIcons(skin, sheet.atStart.map((part) =>
    part.t === "text" ? part.v : part.t === "location" ? shared.placeChipHtml(skin, { location: part.id }) : shared.placeChipHtml(skin, { territory: part.id })).join(""), 18, false);
  html += `<dl><dt>At start</dt><dd>${atStart}</dd><dt>Free revival</dt><dd>${sheet.freeRevival}</dd></dl>`;
  html += `<div class="theme">${sheet.theme}</div>`;
  html += advantageNames(sheet.advantages);
  if (sheet.advancedAdvantages.length > 0) html += "<div class=\"section\">Advanced Game Advantages</div>" + advantageNames(sheet.advancedAdvantages);
  html += "<div class=\"foot\">Hold <kbd>Shift</kbd> for full rules</div></div>";
  html += `<div class="pop-faction-full">${sheet.html}</div></div>`;
  return html;
}

function advantageNames(names: string[]): string {
  return `<ul>${names.map((n) => `<li>${n}</li>`).join("")}</ul>`;
}

// CardPiles.PileHtml.
function pileHtml(heading: string, images: string[]): string {
  return `<div class="pop-pile"><h6>${heading}</h6><div class="pop-pile-cards">${images.join("")}</div></div>`;
}

// CardPiles.IntelOfResourceDiscardPile: with note-keeping, every card of the pile.
export function resourceDiscardPile(b: Board, pile: number[] | null, latest: number, letter: string): string {
  if (pile == null) return shared.resourceCardPopup(b.skin, latest);
  const name = b.applicable("IncreasedResourceFlow") ? `Discard pile ${letter}` : "Discard pile";
  return pileHtml(`${name} · ${pile.length}`, pile.map((c) => `<img src="${b.skin.get("ResourceCardImage_URL", c)}"/>`));
}

// CardPiles.IntelOfTreacheryDiscardPile: the top card, or with note-keeping every card, the ones
// this player hasn't seen face down.
export function treacheryDiscardPile(b: Board): string {
  const piles = b.view.piles;
  if (piles.treacheryDiscard == null) return shared.treacheryCardPopup(b.skin, piles.treacheryDiscardTop!);
  return pileHtml(`Discard pile · ${piles.treacheryDiscardCount}`, piles.treacheryDiscard.map((c) => c != null
    ? `<img src="${b.skin.get("TreacheryCardImage_URL", b.skin.cardSkinId(c))}"/>`
    : `<img class="face-down" src="${b.s("CardBack_TreacheryCard_URL")}"/>`));
}

// CardPiles.IntelOfRicheseCardPile: Richese sees its cache.
export function richeseCache(b: Board): string | null {
  const cache = b.view.piles.richeseCache;
  if (!cache || cache.length === 0) return null;
  const heading = `${shared.factionChipHtml(b.skin, "Richese", true)} cards · ${cache.length}`;
  return pileHtml(heading, cache.map((c) => `<img src="${b.skin.get("TreacheryCardImage_URL", b.skin.cardSkinId(c))}"/>`));
}

// Tanks.PanelHtml: one row per faction with anything in the tanks.
export function tanks(b: Board): string {
  const skin = b.skin;
  const v = b.view;
  let rows = "";
  for (const t of v.tanks) {
    if (t.forcesKilled === 0 && t.specialForcesKilled === 0 && t.leaders.length === 0 && !t.messiahDead) continue;
    const you = !v.viewer.isObserver && v.viewer.faction === t.faction;
    const onPlanet = [...new Set(v.board.forces.filter((x) => x.faction === t.faction && !b.isHomeworld(x.location)).map((x) => x.location))];
    const info = skin.statics.factions.find((x) => x.faction === t.faction);
    rows += "<tr>";
    rows += `<td>${shared.factionChipHtml(skin, t.faction, you, onPlanet)}</td>`;
    rows += "<td class='forces'>";
    if (t.forcesKilled > 0) {
      const title = htmlEncode(skin.get("ForceName_STR", info?.force ?? t.faction));
      rows += `<span class='count'>${forceTokenHtml(skin.get("FactionForceImage_URL", t.faction), t.faction, false, 28, title)}<b>${t.forcesKilled}</b></span>`;
    }
    rows += "</td><td>";
    if (t.specialForcesKilled > 0) {
      const title = htmlEncode(skin.get("SpecialForceName_STR", info?.specialForce ?? t.faction));
      const amount = t.faction === "Richese" ? "" : `<b>${t.specialForcesKilled}</b>`;
      rows += `<span class='count'>${forceTokenHtml(skin.get("FactionSpecialForceImage_URL", t.faction), t.faction, true, 28, title)}${amount}</span>`;
    }
    rows += "</td><td><div class='leaders'>";
    for (const l of t.leaders) {
      if (!l.faceDown) rows += `<img src='${skin.get("PersonImage_URL", l.id!)}' title='${htmlEncode(skin.person(l.id!))}' alt=''/>`;
      else if (l.id != null) rows += `<img class='face-down' src='${skin.get("PersonImage_URL", l.id)}' title='${htmlEncode(skin.person(l.id))} (face down)' alt=''/>`;
      else rows += `<img class='faction-back' src='${skin.get("FactionFacedownImage_URL", t.faction)}' title='Face down' alt=''/>`;
    }
    if (t.messiahDead) rows += `<img src='${b.s("Messiah_URL")}' title='${htmlEncode(skin.concept("Messiah"))}' alt=''/>`;
    rows += "</div></td></tr>";
  }
  return `<div class='tanks-pop'><h6>${htmlEncode(skin.concept("Graveyard"))}</h6><table><tr><th>Faction</th><th colspan='2'>Forces</th><th>Leaders</th></tr>${rows}</table></div>`;
}

// "dddd, dd MMMM yyyy H:mm:ss" of a DateTime as the game holds it: in UTC, printed without
// conversion to the browser's time zone, as C# formats it.
function longDate(iso: string | null): string {
  if (!iso) return "";
  const d = new Date(iso);
  const day = d.toLocaleDateString("en-US", { weekday: "long", timeZone: "UTC" });
  const month = d.toLocaleDateString("en-US", { month: "long", timeZone: "UTC" });
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${day}, ${pad(d.getUTCDate())} ${month} ${d.getUTCFullYear()} ${d.getUTCHours()}:${pad(d.getUTCMinutes())}:${pad(d.getUTCSeconds())}`;
}

// TimeSpan.ToString(@"h\:mm\:ss").
function hms(seconds: number): string {
  const s = Math.floor(seconds);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${Math.floor(s / 3600) % 24}:${pad(Math.floor(s / 60) % 60)}:${pad(s % 60)}`;
}

// Phases.TurnPopover: the ruleset, the rules by group, and the time each player took.
export function turn(b: Board): string {
  const v = b.view;
  // The ruleset, groups and rules as the server's skin describes them (Skin.Format of each).
  let ruleInfo = "";
  for (const g of v.ruleLabels.groups) {
    ruleInfo += `<h5 class='h5'>${g.group}</h5><p class='mt-0 mb-2'>${g.rules.join(", ")}.</p>`;
  }
  let result = `<div><h4>${v.ruleLabels.ruleset}</h4><p>Game version: ${v.version}</p>${ruleInfo}`;
  result += "<h4>Timers</h4>";
  result += `<p><string>Game started:</strong> ${longDate(v.timing.started)}`;
  result += `<p><string>Last activity:</strong> ${longDate(v.timing.lastActivity)}`;
  result += `<p><string>Game duration:</strong> ${Math.round(v.timing.duration / 360) / 10} hours`;
  result += "<table class='table table-sm'><thead><tr class='table-dark'><th>Player</th><th>Bidding</th><th>Move</th><th>Battle</th><th>Total</th></tr></thead><tbody>";
  for (const p of v.players) {
    const t = v.timing.timeSpent.find((x) => x.faction === p.faction);
    const bidding = t?.bidding ?? 0;
    const move = t?.shipmentAndMove ?? 0;
    const battle = t?.battle ?? 0;
    result += `<tr class='table-dark'><td>${p.name}</td><td>${hms(bidding)}</td><td>${hms(move)}</td><td>${hms(battle)}</td><td>${hms(bidding + move + battle)}</td></tr>`;
  }
  return result + "</tbody></table></div>";
}

export { imageHover };
