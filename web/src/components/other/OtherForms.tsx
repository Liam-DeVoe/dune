// The forms of package 7e by options form (OtherComponents/ActionPanel.razor's switch for these
// event types). `otherForm` returns null for forms of other packages. TradePopover and
// CardPlayPopover draw the same forms with `inPopover`.
import type { ActionView } from "../../generated/ActionView";
import {
  AllianceOfferedForm, BeneGesseritPredictionForm, CardTradedForm, ChoamDiscardedForm, ChoamEconomicsForm, ChoamKarmaPreventionForm,
  DiscardedForm, DiscardedSearchedForm, DiscardedTakenForm, DistransUsedForm, DivideResourcesForm, EmperorDiscardedForm, FactionSelectedForm,
  FactionTradeOfferedForm, IxianRemovedCardFromAuctionForm, IxianSelectedStartingCardForm, IxianSwappedCardOnBidForm, KarmaChoamDiscardForm,
  KarmaForm, KarmaHandSwapForm, KarmaHandSwapInitiatedForm, KarmaRicheseBuyForm, MulliganPerformedForm, NexusCardDrawnForm, PlayerReplacedForm,
  ReplacedCardWonForm, ResourcesAuditedForm, ResourcesTransferredForm, RicheseAnnouncesAuctionForm, RicheseAnnouncesBlackMarketForm,
  RicheseGaveCardForm, RicheseKeepsUnsoldCardForm, RicheseSpecifiesAuctionForm, SkillAssignedForm, StormDialledForm, StormSpellPlayedForm,
  TraitorDiscardedForm, TraitorsSelectedForm, type FormProps,
} from "./ChoiceForms";
import { KarmaMonsterForm, NexusPlayedForm } from "./MapForms";
import { SimpleForm, YesNoForm } from "./SimpleForms";
import { AllyPermissionForm, AutomationConfiguredForm, DealOfferedForm, DonatedForm, EmperorBidSupportForm } from "./TradeForms";

const forms: Record<string, (p: FormProps) => React.ReactNode> = {
  Simple: SimpleForm,
  YesNo: YesNoForm,
  AllianceOffered: AllianceOfferedForm,
  StormDialled: StormDialledForm,
  AllyPermission: AllyPermissionForm,
  AutomationConfigured: AutomationConfiguredForm,
  BeneGesseritPrediction: BeneGesseritPredictionForm,
  CardTraded: CardTradedForm,
  ChoamDiscarded: ChoamDiscardedForm,
  ChoamEconomics: ChoamEconomicsForm,
  ChoamKarmaPrevention: ChoamKarmaPreventionForm,
  DealOffered: DealOfferedForm,
  Discarded: DiscardedForm,
  DiscardedSearched: DiscardedSearchedForm,
  DiscardedTaken: DiscardedTakenForm,
  DistransUsed: DistransUsedForm,
  DivideResources: DivideResourcesForm,
  Donated: DonatedForm,
  EmperorBidSupport: EmperorBidSupportForm,
  EmperorDiscarded: EmperorDiscardedForm,
  FactionSelected: FactionSelectedForm,
  FactionTradeOffered: FactionTradeOfferedForm,
  IxianRemovedCardFromAuction: IxianRemovedCardFromAuctionForm,
  IxianSelectedStartingCard: IxianSelectedStartingCardForm,
  IxianSwappedCardOnBid: IxianSwappedCardOnBidForm,
  Karma: KarmaForm,
  KarmaChoamDiscard: KarmaChoamDiscardForm,
  KarmaHandSwap: KarmaHandSwapForm,
  KarmaHandSwapInitiated: KarmaHandSwapInitiatedForm,
  KarmaMonster: KarmaMonsterForm,
  KarmaRicheseBuy: KarmaRicheseBuyForm,
  MulliganPerformed: MulliganPerformedForm,
  NexusCardDrawn: NexusCardDrawnForm,
  NexusPlayed: NexusPlayedForm,
  PlayerReplaced: PlayerReplacedForm,
  ReplacedCardWon: ReplacedCardWonForm,
  ResourcesAudited: ResourcesAuditedForm,
  ResourcesTransferred: ResourcesTransferredForm,
  SkillAssigned: SkillAssignedForm,
  StormSpellPlayed: StormSpellPlayedForm,
  TraitorDiscarded: TraitorDiscardedForm,
  TraitorsSelected: TraitorsSelectedForm,
  RicheseAnnouncesAuction: RicheseAnnouncesAuctionForm,
  RicheseAnnouncesBlackMarket: RicheseAnnouncesBlackMarketForm,
  RicheseGaveCard: RicheseGaveCardForm,
  RicheseKeepsUnsoldCard: RicheseKeepsUnsoldCardForm,
  RicheseSpecifiesAuction: RicheseSpecifiesAuctionForm,
};

// CardPlays.cs `Trade.Actions`: these open from the menu bar (TradePopover), not the side column.
export const TRADE_ACTIONS = ["DealOffered", "Donated", "AllyPermission"];

// The form for one action of this package, or null. EndPhase's Simple options are the prompt's
// "Continue" button (GameStatusComponent), and AutomationConfigured opens from the menu bar
// (MenuPanels), so the side column's action panel leaves them out.
export function otherForm(action: ActionView, inPopover = false): React.ReactNode | null {
  const form = action.options?.form;
  if (form == null || action.eventType === "EndPhase") return null;
  const F = forms[form];
  return F ? <F key={action.eventType} action={action} inPopover={inPopover} /> : null;
}

// ActionPanel.Shows without a filter: not a Trade action, not AutomationConfigured (menu bar).
// Card plays from the hand (`me.hand[].playableWith`) are left out by the caller.
export function shownInActionPanel(action: ActionView): boolean {
  return !TRADE_ACTIONS.includes(action.eventType) && action.eventType !== "AutomationConfigured";
}
