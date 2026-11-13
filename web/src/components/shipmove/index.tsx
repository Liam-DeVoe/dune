// The ship-and-move forms, by event type, for ActionPanel's switch. ActionPanel leaves out the
// ones played from a card in the faction panel.
import type { ReactNode } from "react";
import type { ActionView } from "../../generated/ActionView";
import { AmbassadorActivatedForm, BeneGesseritFlipForm, DiscoveryRevealedForm, FlightUsedForm, KarmaShipmentPreventionForm, PerformFremenSetupForm, SetShipmentPermissionForm, TestingStationUsedForm } from "./OtherForms";
import { AmbassadorPlacedForm, BeneGesseritAccompaniesForm, ChoamMovePreventionForm, ChoamRemoveForceForm, DiscoveryEnteredForm, FremenSentMonsterForm, LocationPickForm, TerrorPlantedForm, TerrorRevealedForm } from "./PickForms";
import { PlacementForm } from "./PlacementForm";
import { ShipmentForm } from "./ShipmentForm";

// The form of a ship-and-move action, or undefined when the action is not one of them.
export function shipMoveForm(a: ActionView): ReactNode | undefined {
  const key = a.eventType;
  switch (a.options?.form) {
    case "Shipment": return <ShipmentForm key={key} action={a} />;
    case "Move": return <PlacementForm key={key} action={a} kind="Move" />;
    case "Caravan": return <PlacementForm key={key} action={a} kind="Caravan" />;
    case "FremenRidesMonster": return <PlacementForm key={key} action={a} kind="FremenRidesMonster" />;
    case "PerformFremenSetup": return <PerformFremenSetupForm key={key} action={a} />;
    case "DiscoveryEntered": return <DiscoveryEnteredForm key={key} action={a} />;
    case "AmbassadorActivated": return <AmbassadorActivatedForm key={key} action={a} />;
    case "AmbassadorPlaced": return <AmbassadorPlacedForm key={key} action={a} />;
    case "BeneGesseritAccompanies": return <BeneGesseritAccompaniesForm key={key} action={a} />;
    case "BeneGesseritFlip": return <BeneGesseritFlipForm key={key} action={a} />;
    case "ChoamMovePrevention": return <ChoamMovePreventionForm key={key} action={a} />;
    case "ChoamRemoveForce": return <ChoamRemoveForceForm key={key} action={a} />;
    case "DiscoveryRevealed": return <DiscoveryRevealedForm key={key} action={a} />;
    case "FlightUsed": return <FlightUsedForm key={key} action={a} />;
    case "TestingStationUsed": return <TestingStationUsedForm key={key} action={a} />;
    case "KarmaHmsMovement":
    case "PerformHmsMovement":
    case "PerformBeneGesseritPlacement":
    case "PerformMoritaniSetup":
    case "PerformHmsPlacement":
      return <LocationPickForm key={key} action={a} />;
    case "FremenSentMonster": return <FremenSentMonsterForm key={key} action={a} />;
    case "KarmaShipmentPrevention": return <KarmaShipmentPreventionForm key={key} action={a} />;
    case "SetShipmentPermission": return <SetShipmentPermissionForm key={key} action={a} />;
    case "TerrorPlanted": return <TerrorPlantedForm key={key} action={a} />;
    case "TerrorRevealed": return <TerrorRevealedForm key={key} action={a} />;
    default: return undefined;
  }
}

