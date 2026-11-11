// GenericComponents/ExpressionComponent.razor: draws a message the server rendered to parts
// (rust/view/src/expr.rs, which picks each part by the same switch). Labels come from the server
// (Skin.Describe with the game's skin); images, colours and popovers from the client's skin.
//
// size is the component's Size (14 by default, 20 in the log and prompts). logStyle draws a
// leader as its disc and a treachery card as a chip with its art, as the log does. textBorder
// is the TextBorder style, which C# applies to text, names and bold enums but not to parts of
// nested messages. The view flattens nested messages, so a caller that sets textBorder passes
// bordered (FlashView.bordered) to say which parts get it; without it every part does.
import type { CSSProperties } from "react";
import type { Ambassador, DiscoveryToken, Faction, LeaderSkill, TechToken, TerrorType } from "../generated/enums";
import type { Part } from "../generated/Part";
import {
  ambassadorPopup, discoveryTokenPopup, leaderPopup, leaderSkillPopup, messiahPopup, resourceCardPopup, techTokenPopup,
  terrorPopup, treacheryCardPopup, withSpiceIcons,
} from "../popups";
import { useGame } from "../state/context";
import { utilitiesRound } from "../text";
import { FactionChip } from "./FactionChip";
import { Image } from "./Image";
import { MonsterIcon } from "./MonsterIcon";
import { PlaceChip } from "./PlaceChip";
import { PopoverText } from "./PopoverText";
import { SimpleNumber } from "./SimpleNumber";
import { SpiceComponent } from "./SpiceComponent";

// The enum types ExpressionComponent draws bold, with spice amounts as icons.
const bold = new Set(["AuctionType", "ChoamEconomicsStatus", "TreacheryCardType", "StrongholdAdvantage", "FactionAdvantage", "Ruleset", "Rule"]);

// Support.HasStarredSpecialForce: special forces with a star badge.
const starred = new Set(["Fremen", "Emperor", "Ixian"]);

export interface RichTextProps {
  parts: Part[];
  size?: number;
  logStyle?: boolean;
  textBorder?: CSSProperties;
  bordered?: boolean[];
}

export function RichText({ parts, size = 14, logStyle = false, textBorder, bordered }: RichTextProps) {
  return <>{parts.map((p, i) => <PartView key={i} part={p} size={size} logStyle={logStyle} textBorder={bordered?.[i] === false ? undefined : textBorder} />)}</>;
}

function PartView({ part: p, size, logStyle, textBorder }: { part: Part; size: number; logStyle: boolean; textBorder?: CSSProperties }) {
  const { skin } = useGame();
  switch (p.t) {
    case "none": return <span className="fw-bold">none</span>;
    case "text": return <span style={textBorder}>{p.v}</span>;
    case "num": return <SimpleNumber size={size}>{utilitiesRound(p.v)}</SimpleNumber>;
    case "payment":
      return (
        <>
          <SpiceComponent amount={String(p.amount)} size={size} />
          {p.by && <span> by <FactionChip faction={p.by} /></span>}
        </>
      );
    case "faction": return <FactionChip faction={p.v} />;
    case "treacheryCard":
      return logStyle
        ? <PopoverText popover={treacheryCardPopup(skin, p.id)} className="card-chip"><img src={skin.cardImage(p.id)} alt="" />{p.label}</PopoverText>
        : <PopoverText popover={treacheryCardPopup(skin, p.id)} style={textBorder}>{p.label}</PopoverText>;
    case "leader": {
      const popover = leaderPopup(skin, p.id, p.skill);
      // Width = Size * 1.3f, a float.
      return logStyle
        ? <Image src={skin.personImage(p.id)} alt={p.label} width={Math.fround(size * Math.fround(1.3))} className="leader-disc" noShadow popover={popover} />
        : <PopoverText popover={popover} style={textBorder}>{p.label}</PopoverText>;
    }
    case "messiah": return <PopoverText popover={messiahPopup(skin)} style={textBorder}>{p.label}</PopoverText>;
    case "resourceCard": return <PopoverText popover={resourceCardPopup(skin, p.id)} style={textBorder}>{p.label}</PopoverText>;
    case "location": return <PlaceChip place={{ location: p.id }} />;
    case "territory": return <PlaceChip place={{ territory: p.id }} />;
    case "bool": return <span style={textBorder}>{p.label}</span>;
    case "enum": {
      switch (p.ty) {
        case "LeaderSkill": return <PopoverText popover={leaderSkillPopup(skin, p.v as LeaderSkill)} style={textBorder}>{p.label}</PopoverText>;
        case "TechToken": return <PopoverText popover={techTokenPopup(skin, p.v as TechToken)} style={textBorder}>{p.label}</PopoverText>;
        case "DiscoveryToken": return <PopoverText popover={discoveryTokenPopup(skin, p.v as DiscoveryToken)} style={textBorder}>{p.label}</PopoverText>;
        case "TerrorType": return <PopoverText popover={terrorPopup(skin, p.v as TerrorType)} style={textBorder}>{p.label}</PopoverText>;
        case "Concept":
          if (p.v === "Monster") return <MonsterIcon size={size} />;
          break;
        case "FactionForce":
          return <Image src={skin.url(skin.get("FactionForceImage_URL", p.v as Faction))} alt={p.label} width={size} tooltip={p.label} />;
        case "FactionSpecialForce":
          return <Image src={skin.url(skin.get("FactionSpecialForceImage_URL", p.v as Faction))} alt={p.label} width={size} tooltip={p.label} starred={starred.has(p.v)} />;
        case "Ambassador":
          return <Image src={skin.url(skin.get("AmbassadorImage_URL", p.v))} alt={p.label} width={size} popover={ambassadorPopup(skin, p.v as Ambassador)} />;
      }
      if (bold.has(p.ty)) {
        return <span className="fw-bold" style={textBorder} dangerouslySetInnerHTML={{ __html: withSpiceIcons(skin, p.label, size) }} />;
      }
      return <span style={textBorder}>{p.label}</span>;
    }
  }
}
