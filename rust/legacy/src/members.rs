//! The JSON members of every event and of `GameSettings`, in the order they are written.
//!
//! Generated once from the serializer's type information; maintained by hand since the generator
//! was retired. The machinery is in json/mod.rs.

use crate::{FromJson, JsonError, JsonObject, JsonValue, JsonWriter};
use treachery_engine::prelude::*;

impl JsonObject for AcceptOrCancelTleilaxuRevival {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Cancel", &self.cancel);
        w.member("Price", &self.price);
        w.member("_heroId", &self.hero);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Cancel" => self.cancel = FromJson::from_json(value)?,
            "Price" => self.price = FromJson::from_json(value)?,
            "_heroId" => self.hero = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for AllianceBroken {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for AllianceByAmbassador {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for AllianceByTerror {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for AllianceOffered {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Target", &self.target);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Target" => self.target = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for AlliancePassed {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for AllyPermission {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("AllyMayShipAsGuild", &self.ally_may_ship_as_guild);
        w.member("GuildAllowsShippingDiscount", &self.ally_may_ship_as_guild);
        w.member("EmperorWillPayForExtraRevival", &self.emperor_will_pay_for_extra_revival);
        w.member("FremenWillProtectFromMonster", &self.fremen_will_protect_from_monster);
        w.member("FremenAllowsThreeFreeRevivals", &self.fremen_allows_three_free_revivals);
        w.member("FremenSharesPrescience", &self.fremen_shares_prescience);
        w.member("FremenRefundsBattleDial", &self.fremen_refunds_battle_dial);
        w.member("TleilaxuAllowsRevivalDiscount", &self.ally_may_revive_as_tleilaxu);
        w.member("AllyMayReviveAsTleilaxu", &self.ally_may_revive_as_tleilaxu);
        w.member("IxianAllowsReplacingCards", &self.ally_may_replace_cards);
        w.member("AllyMayReplaceCards", &self.ally_may_replace_cards);
        w.member("AtreidesSharesPrescience", &self.atreides_shares_prescience);
        w.member("BeneGesseritAllowsUseOfVoice", &self.bene_gesserit_allows_use_of_voice);
        w.member("RicheseAllowsUseOfNoField", &self.richese_allows_use_of_no_field);
        w.member("MoritaniAllowsKeepingCards", &self.moritani_allows_keeping_cards);
        w.member("EcazSharesAmbassadors", &self.ecaz_shares_ambassadors);
        w.member("PermittedResources", &self.permitted_resources);
        w.member("_permittedKarmaCardId", &self.permitted_karma_card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "AllyMayShipAsGuild" => self.ally_may_ship_as_guild = FromJson::from_json(value)?,
            "GuildAllowsShippingDiscount" => self.ally_may_ship_as_guild = FromJson::from_json(value)?,
            "EmperorWillPayForExtraRevival" => self.emperor_will_pay_for_extra_revival = FromJson::from_json(value)?,
            "FremenWillProtectFromMonster" => self.fremen_will_protect_from_monster = FromJson::from_json(value)?,
            "FremenAllowsThreeFreeRevivals" => self.fremen_allows_three_free_revivals = FromJson::from_json(value)?,
            "FremenSharesPrescience" => self.fremen_shares_prescience = FromJson::from_json(value)?,
            "FremenRefundsBattleDial" => self.fremen_refunds_battle_dial = FromJson::from_json(value)?,
            "TleilaxuAllowsRevivalDiscount" => self.ally_may_revive_as_tleilaxu = FromJson::from_json(value)?,
            "AllyMayReviveAsTleilaxu" => self.ally_may_revive_as_tleilaxu = FromJson::from_json(value)?,
            "IxianAllowsReplacingCards" => self.ally_may_replace_cards = FromJson::from_json(value)?,
            "AllyMayReplaceCards" => self.ally_may_replace_cards = FromJson::from_json(value)?,
            "AtreidesSharesPrescience" => self.atreides_shares_prescience = FromJson::from_json(value)?,
            "BeneGesseritAllowsUseOfVoice" => self.bene_gesserit_allows_use_of_voice = FromJson::from_json(value)?,
            "RicheseAllowsUseOfNoField" => self.richese_allows_use_of_no_field = FromJson::from_json(value)?,
            "MoritaniAllowsKeepingCards" => self.moritani_allows_keeping_cards = FromJson::from_json(value)?,
            "EcazSharesAmbassadors" => self.ecaz_shares_ambassadors = FromJson::from_json(value)?,
            "PermittedResources" => self.permitted_resources = FromJson::from_json(value)?,
            "_permittedKarmaCardId" => self.permitted_karma_card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for AmalPlayed {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for AmbassadorActivated {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("BeneGesseritSelectedAmbassador", &self.bene_gesserit_selected_ambassador);
        w.member("EcazOfferAlliance", &self.ecaz_offer_alliance);
        w.member("EcazGiveVidalToAlly", &self.ecaz_give_vidal_to_ally);
        w.member("EcazTakeVidal", &self.ecaz_take_vidal);
        w.member("GuildForceAmount", &self.guild_force_amount);
        w.member("TleilaxuAmountOfForces", &self.tleilaxu_amount_of_forces);
        w.member("TleilaxuAssignSkill", &self.tleilaxu_assign_skill);
        w.member("_choamCardIds", &self.choam_card_ids);
        w.member("_fremenOrGuildToId", &self.fremen_or_guild_to);
        w.member("_fremenForceLocations", &self.fremen_force_locations);
        w.member("_ixianCardId", &self.ixian_card);
        w.member("_tleilaxuHeroId", &self.tleilaxu_hero);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "BeneGesseritSelectedAmbassador" => self.bene_gesserit_selected_ambassador = FromJson::from_json(value)?,
            "EcazOfferAlliance" => self.ecaz_offer_alliance = FromJson::from_json(value)?,
            "EcazGiveVidalToAlly" => self.ecaz_give_vidal_to_ally = FromJson::from_json(value)?,
            "EcazTakeVidal" => self.ecaz_take_vidal = FromJson::from_json(value)?,
            "GuildForceAmount" => self.guild_force_amount = FromJson::from_json(value)?,
            "TleilaxuAmountOfForces" => self.tleilaxu_amount_of_forces = FromJson::from_json(value)?,
            "TleilaxuAssignSkill" => self.tleilaxu_assign_skill = FromJson::from_json(value)?,
            "_choamCardIds" => self.choam_card_ids = FromJson::from_json(value)?,
            "_fremenOrGuildToId" => self.fremen_or_guild_to = FromJson::from_json(value)?,
            "_fremenForceLocations" => self.fremen_force_locations = crate::read_force_locations(value)?,
            "_ixianCardId" => self.ixian_card = FromJson::from_json(value)?,
            "_tleilaxuHeroId" => self.tleilaxu_hero = FromJson::from_json(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for AmbassadorPlaced {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Ambassador", &self.ambassador);
        w.member("_strongholdId", &self.stronghold);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Ambassador" => self.ambassador = FromJson::from_json(value)?,
            "_strongholdId" => self.stronghold = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for AuditCancelled {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Cancelled", &self.cancelled);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Cancelled" => self.cancelled = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for Audited {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for AutomationConfigured {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("AutomationRuleId", &self.automation_rule_id);
        w.member("Action", &self.action);
        w.member("RuleType", &self.rule_type);
        w.member("BiddingAboveAmount", &self.bidding_above_amount);
        w.member("BiddingWinningFaction", &self.bidding_winning_faction);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "AutomationRuleId" => self.automation_rule_id = FromJson::from_json(value)?,
            "Action" => self.action = FromJson::from_json(value)?,
            "RuleType" => self.rule_type = FromJson::from_json(value)?,
            "BiddingAboveAmount" => self.bidding_above_amount = FromJson::from_json(value)?,
            "BiddingWinningFaction" => self.bidding_winning_faction = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for Battle {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Messiah", &self.messiah);
        w.member("Forces", &self.forces);
        w.member("ForcesAtHalfStrength", &self.forces_at_half_strength);
        w.member("SpecialForces", &self.special_forces);
        w.member("SpecialForcesAtHalfStrength", &self.special_forces_at_half_strength);
        w.member("AllyContributionAmount", &self.ally_contribution_amount);
        w.member("BankerBonus", &self.banker_bonus);
        w.member("_heroId", &self.hero);
        w.member("_weaponCardId", &self.weapon);
        w.member("_defenseCardId", &self.defense);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Messiah" => self.messiah = FromJson::from_json(value)?,
            "Forces" => self.forces = FromJson::from_json(value)?,
            "ForcesAtHalfStrength" => self.forces_at_half_strength = FromJson::from_json(value)?,
            "SpecialForces" => self.special_forces = FromJson::from_json(value)?,
            "SpecialForcesAtHalfStrength" => self.special_forces_at_half_strength = FromJson::from_json(value)?,
            "AllyContributionAmount" => self.ally_contribution_amount = FromJson::from_json(value)?,
            "BankerBonus" => self.banker_bonus = FromJson::from_json(value)?,
            "_heroId" => self.hero = FromJson::from_json(value)?,
            "_weaponCardId" => self.weapon = FromJson::from_json(value)?,
            "_defenseCardId" => self.defense = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for BattleClaimed {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for BattleConcluded {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Kill", &self.kill);
        w.member("Capture", &self.capture);
        w.member("SpecialForceLossesReplaced", &self.special_force_losses_replaced);
        w.member("StolenToken", &self.stolen_token);
        w.member("AddExtraForce", &self.add_extra_force);
        w.member("_traitorToReplaceId", &self.traitor_to_replace);
        w.member("_newTraitorId", &self.new_traitor);
        w.member("_cardIds", &self.card_ids);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Kill" => self.kill = FromJson::from_json(value)?,
            "Capture" => self.capture = FromJson::from_json(value)?,
            "SpecialForceLossesReplaced" => self.special_force_losses_replaced = FromJson::from_json(value)?,
            "StolenToken" => self.stolen_token = FromJson::from_json(value)?,
            "AddExtraForce" => self.add_extra_force = FromJson::from_json(value)?,
            "_traitorToReplaceId" => self.traitor_to_replace = FromJson::from_json(value)?,
            "_newTraitorId" => self.new_traitor = FromJson::from_json(value)?,
            "_cardIds" => self.card_ids = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for BattleInitiated {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Target", &self.target);
        w.member("_territoryId", &self.territory);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Target" => self.target = FromJson::from_json(value)?,
            "_territoryId" => self.territory = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for BattleRevision {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for BeneGesseritAccompanies {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Accompanies", &self.accompanies);
        w.member("ExtraAdvisor", &self.extra_advisor);
        w.member("_targetId", &self.location);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Accompanies" => self.accompanies = FromJson::from_json(value)?,
            "ExtraAdvisor" => self.extra_advisor = FromJson::from_json(value)?,
            "_targetId" => self.location = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for BeneGesseritBattleAnnouncement {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_territoryId", &self.territory);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_territoryId" => self.territory = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for BeneGesseritFlip {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("AsAdvisors", &self.as_advisors);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "AsAdvisors" => self.as_advisors = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for BeneGesseritPrediction {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("ToWin", &self.to_win);
        w.member("Turn", &self.turn);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "ToWin" => self.to_win = FromJson::from_json(value)?,
            "Turn" => self.turn = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for Bid {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Amount", &self.amount);
        w.member("AllyContributionAmount", &self.ally_contribution_amount);
        w.member("EmperorContributionAmount", &self.emperor_contribution_amount);
        w.member("UsesEmperorSecretAlly", &self.uses_emperor_secret_ally);
        w.member("KarmaBid", &self.karma_bid);
        w.member("_karmaCardId", &self.karma_card);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Amount" => self.amount = FromJson::from_json(value)?,
            "AllyContributionAmount" => self.ally_contribution_amount = FromJson::from_json(value)?,
            "EmperorContributionAmount" => self.emperor_contribution_amount = FromJson::from_json(value)?,
            "UsesEmperorSecretAlly" => self.uses_emperor_secret_ally = FromJson::from_json(value)?,
            "KarmaBid" => self.karma_bid = FromJson::from_json(value)?,
            "_karmaCardId" => self.karma_card = FromJson::from_json(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for BlackMarketBid {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Amount", &self.amount);
        w.member("AllyContributionAmount", &self.ally_contribution_amount);
        w.member("EmperorContributionAmount", &self.emperor_contribution_amount);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Amount" => self.amount = FromJson::from_json(value)?,
            "AllyContributionAmount" => self.ally_contribution_amount = FromJson::from_json(value)?,
            "EmperorContributionAmount" => self.emperor_contribution_amount = FromJson::from_json(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for Bureaucracy {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for Captured {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for Caravan {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("AsAdvisors", &self.as_advisors);
        w.member("_toId", &self.to);
        w.member("_forceLocations", &self.force_locations);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "AsAdvisors" => self.as_advisors = FromJson::from_json(value)?,
            "_toId" => self.to = FromJson::from_json(value)?,
            "_forceLocations" => self.force_locations = crate::read_force_locations(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for CardGiven {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for CardTraded {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Target", &self.target);
        w.member("_cardId", &self.card);
        w.member("_requestedCardId", &self.requested_card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Target" => self.target = FromJson::from_json(value)?,
            "_cardId" => self.card = FromJson::from_json(value)?,
            "_requestedCardId" => self.requested_card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for CardsDetermined {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_treacheryCardIds", &self.treachery_card_ids);
        w.member("_richeseCardIds", &self.richese_card_ids);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_treacheryCardIds" => self.treachery_card_ids = FromJson::from_json(value)?,
            "_richeseCardIds" => self.richese_card_ids = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for CharityClaimed {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for ChoamDiscarded {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_cardId", &self.card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_cardId" => self.card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for ChoamEconomics {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Status", &self.status);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Status" => self.status = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for ChoamExtraMove {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for ChoamFreeRevivalPrevention {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Target", &self.target);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Target" => self.target = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for ChoamKarmaPrevention {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Target", &self.target);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Target" => self.target = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for ChoamMovePrevention {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_territoryId", &self.territory);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_territoryId" => self.territory = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for ChoamRemoveForce {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Target", &self.target);
        w.member("SpecialForce", &self.special_force);
        w.member("_locationId", &self.location);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Target" => self.target = FromJson::from_json(value)?,
            "SpecialForce" => self.special_force = FromJson::from_json(value)?,
            "_locationId" => self.location = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for ClairVoyanceAnswered {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Answer", &self.answer);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Answer" => self.answer = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for ClairVoyancePlayed {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Question", &self.question);
        w.member("Target", &self.target);
        w.member("QuestionParameter1", &self.question_parameter1);
        w.member("QuestionParameter2", &self.question_parameter2);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Question" => self.question = FromJson::from_json(value)?,
            "Target" => self.target = FromJson::from_json(value)?,
            "QuestionParameter1" => self.question_parameter1 = FromJson::from_json(value)?,
            "QuestionParameter2" => self.question_parameter2 = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for DealAccepted {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("BoundFaction", &self.bound_faction);
        w.member("Type", &self.type_);
        w.member("DealParameter1", &self.deal_parameter1);
        w.member("DealParameter2", &self.deal_parameter2);
        w.member("Text", &self.text);
        w.member("End", &self.end);
        w.member("Price", &self.price);
        w.member("Benefit", &self.benefit);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "BoundFaction" => self.bound_faction = FromJson::from_json(value)?,
            "Type" => self.type_ = FromJson::from_json(value)?,
            "DealParameter1" => self.deal_parameter1 = FromJson::from_json(value)?,
            "DealParameter2" => self.deal_parameter2 = FromJson::from_json(value)?,
            "Text" => self.text = FromJson::from_json(value)?,
            "End" => self.end = FromJson::from_json(value)?,
            "Price" => self.price = FromJson::from_json(value)?,
            "Benefit" => self.benefit = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for DealOffered {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("To", &self.to);
        w.member("Type", &self.type_);
        w.member("DealParameter1", &self.deal_parameter1);
        w.member("DealParameter2", &self.deal_parameter2);
        w.member("Text", &self.text);
        w.member("EndPhase", &self.end_phase);
        w.member("Price", &self.price);
        w.member("Benefit", &self.benefit);
        w.member("Cancel", &self.cancel);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "To" => self.to = FromJson::from_json(value)?,
            "Type" => self.type_ = FromJson::from_json(value)?,
            "DealParameter1" => self.deal_parameter1 = FromJson::from_json(value)?,
            "DealParameter2" => self.deal_parameter2 = FromJson::from_json(value)?,
            "Text" => self.text = FromJson::from_json(value)?,
            "EndPhase" => self.end_phase = FromJson::from_json(value)?,
            "Price" => self.price = FromJson::from_json(value)?,
            "Benefit" => self.benefit = FromJson::from_json(value)?,
            "Cancel" => self.cancel = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for DealRejected {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("BoundFaction", &self.bound_faction);
        w.member("Type", &self.type_);
        w.member("DealParameter1", &self.deal_parameter1);
        w.member("DealParameter2", &self.deal_parameter2);
        w.member("Text", &self.text);
        w.member("End", &self.end);
        w.member("Price", &self.price);
        w.member("Benefit", &self.benefit);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "BoundFaction" => self.bound_faction = FromJson::from_json(value)?,
            "Type" => self.type_ = FromJson::from_json(value)?,
            "DealParameter1" => self.deal_parameter1 = FromJson::from_json(value)?,
            "DealParameter2" => self.deal_parameter2 = FromJson::from_json(value)?,
            "Text" => self.text = FromJson::from_json(value)?,
            "End" => self.end = FromJson::from_json(value)?,
            "Price" => self.price = FromJson::from_json(value)?,
            "Benefit" => self.benefit = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for Diplomacy {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_cardId", &self.card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_cardId" => self.card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for Discarded {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_cardId", &self.card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_cardId" => self.card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for DiscardedSearched {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_cardId", &self.card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_cardId" => self.card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for DiscardedSearchedAnnounced {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for DiscardedTaken {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_cardId", &self.card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_cardId" => self.card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for DiscoveryEntered {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_toId", &self.to);
        w.member("_forceLocations", &self.force_locations);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_toId" => self.to = FromJson::from_json(value)?,
            "_forceLocations" => self.force_locations = crate::read_force_locations(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for DiscoveryRevealed {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_locationId", &self.location);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_locationId" => self.location = FromJson::from_json(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for DistransUsed {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Target", &self.target);
        w.member("_cardId", &self.card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Target" => self.target = FromJson::from_json(value)?,
            "_cardId" => self.card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for DivideResources {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("PortionToFirstPlayer", &self.portion_to_first_player);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "PortionToFirstPlayer" => self.portion_to_first_player = FromJson::from_json(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for DivideResourcesAccepted {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for Donated {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Target", &self.target);
        w.member("Resources", &self.resources);
        w.member("FromBank", &self.from_bank);
        w.member("_cardId", &self.card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Target" => self.target = FromJson::from_json(value)?,
            "Resources" => self.resources = FromJson::from_json(value)?,
            "FromBank" => self.from_bank = FromJson::from_json(value)?,
            "_cardId" => self.card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for EmperorBidSupport {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Amounts", &self.amounts);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Amounts" => self.amounts = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for EmperorDiscarded {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_cardId", &self.card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_cardId" => self.card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for EndPhase {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for EstablishPlayers {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Settings", &self.settings);
        w.member("Seed", &self.seed);
        w.member("MaximumNumberOfPlayers", &self.maximum_number_of_players);
        w.member("MaximumTurns", &self.maximum_turns);
        w.member("ApplicableRules", &self.applicable_rules);
        w.member("_players", &self.players);
        w.member("_factionsInPlay", &self.factions_in_play);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Settings" => self.settings = FromJson::from_json(value)?,
            "Seed" => self.seed = FromJson::from_json(value)?,
            "MaximumNumberOfPlayers" => self.maximum_number_of_players = FromJson::from_json(value)?,
            "MaximumTurns" => self.maximum_turns = FromJson::from_json(value)?,
            "ApplicableRules" => self.applicable_rules = FromJson::from_json(value)?,
            "_players" => self.players = FromJson::from_json(value)?,
            "_factionsInPlay" => self.factions_in_play = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for ExtortionPrevented {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for FaceDanced {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("ForcesFromReserve", &self.forces_from_reserve);
        w.member("FaceDancerCalled", &self.face_dancer_called);
        w.member("_targetForceLocations", &self.target_force_locations);
        w.member("_toId", &self.to);
        w.member("_forceLocations", &self.force_locations);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "ForcesFromReserve" => self.forces_from_reserve = FromJson::from_json(value)?,
            "FaceDancerCalled" => self.face_dancer_called = FromJson::from_json(value)?,
            "_targetForceLocations" => self.target_force_locations = crate::read_force_locations(value)?,
            "_toId" => self.to = FromJson::from_json(value)?,
            "_forceLocations" => self.force_locations = crate::read_force_locations(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for FaceDancerReplaced {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("dancerId", &self.selected_dancer);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "dancerId" => self.selected_dancer = FromJson::from_json(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for FaceDancerRevealed {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for FactionSelected {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("InitiatorPlayerName", &self.initiator_player_name);
        w.member("Seat", &self.seat);
        w.member("Faction", &self.faction);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "InitiatorPlayerName" => self.initiator_player_name = FromJson::from_json(value)?,
            "Seat" => self.seat = FromJson::from_json(value)?,
            "Faction" => self.faction = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for FactionTradeOffered {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Target", &self.target);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Target" => self.target = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for FlightDiscoveryUsed {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for FlightUsed {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("MoveThreeTerritories", &self.move_three_territories);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "MoveThreeTerritories" => self.move_three_territories = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for FremenRidesMonster {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("ForcesFromReserves", &self.forces_from_reserves);
        w.member("SpecialForcesFromReserves", &self.special_forces_from_reserves);
        w.member("_toId", &self.to);
        w.member("_forceLocations", &self.force_locations);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "ForcesFromReserves" => self.forces_from_reserves = FromJson::from_json(value)?,
            "SpecialForcesFromReserves" => self.special_forces_from_reserves = FromJson::from_json(value)?,
            "_toId" => self.to = FromJson::from_json(value)?,
            "_forceLocations" => self.force_locations = crate::read_force_locations(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for FremenSentMonster {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_territoryId", &self.territory);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_territoryId" => self.territory = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for GuildDelay {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for HMSAdvantageChosen {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Advantage", &self.advantage);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Advantage" => self.advantage = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for HarvesterPlayed {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for HideSecrets {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for IxianRemovedCardFromAuction {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("PutOnTop", &self.put_on_top);
        w.member("_cardId", &self.card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "PutOnTop" => self.put_on_top = FromJson::from_json(value)?,
            "_cardId" => self.card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for IxianSelectedStartingCard {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_cardId", &self.card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_cardId" => self.card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for IxianSwappedCardOnBid {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_cardId", &self.card);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_cardId" => self.card = FromJson::from_json(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for JuicePlayed {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Type", &self.type_);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Type" => self.type_ = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for Karma {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Prevented", &self.prevented);
        w.member("_cardId", &self.card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Prevented" => self.prevented = FromJson::from_json(value)?,
            "_cardId" => self.card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for KarmaChoamDiscard {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_cardToUse", &self.card_to_use);
        w.member("_cardIds", &self.card_ids);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_cardToUse" => self.card_to_use = FromJson::from_json(value)?,
            "_cardIds" => self.card_ids = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for KarmaEcazDial {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for KarmaFreeRevival {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("AmountOfForces", &self.amount_of_forces);
        w.member("AmountOfSpecialForces", &self.amount_of_special_forces);
        w.member("AssignSkill", &self.assign_skill);
        w.member("_heroId", &self.hero);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "AmountOfForces" => self.amount_of_forces = FromJson::from_json(value)?,
            "AmountOfSpecialForces" => self.amount_of_special_forces = FromJson::from_json(value)?,
            "AssignSkill" => self.assign_skill = FromJson::from_json(value)?,
            "_heroId" => self.hero = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for KarmaHandSwap {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_cardIds", &self.card_ids);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_cardIds" => self.card_ids = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for KarmaHandSwapInitiated {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Target", &self.target);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Target" => self.target = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for KarmaHmsMovement {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Passed", &self.passed);
        w.member("_targetId", &self.target);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Passed" => self.passed = FromJson::from_json(value)?,
            "_targetId" => self.target = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for KarmaMonster {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_territoryId", &self.territory);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_territoryId" => self.territory = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for KarmaPrescience {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for KarmaRevivalPrevention {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Target", &self.target);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Target" => self.target = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for KarmaRicheseBuy {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_cardId", &self.card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_cardId" => self.card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for KarmaShipmentPrevention {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Target", &self.target);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Target" => self.target = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for LoserConcluded {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("KarmaForcedKeptCardDecision", &self.karma_forced_kept_card_decision);
        w.member("Assassinate", &self.assassinate);
        w.member("_forcedKeptOrDiscardedCardIds", &self.forced_kept_or_discarded_card_ids);
        w.member("_keptCardId", &self.kept_card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "KarmaForcedKeptCardDecision" => self.karma_forced_kept_card_decision = FromJson::from_json(value)?,
            "Assassinate" => self.assassinate = FromJson::from_json(value)?,
            "_forcedKeptOrDiscardedCardIds" => self.forced_kept_or_discarded_card_ids = FromJson::from_json(value)?,
            "_keptCardId" => self.kept_card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for MetheorPlayed {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for Move {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("AsAdvisors", &self.as_advisors);
        w.member("_toId", &self.to);
        w.member("_forceLocations", &self.force_locations);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "AsAdvisors" => self.as_advisors = FromJson::from_json(value)?,
            "_toId" => self.to = FromJson::from_json(value)?,
            "_forceLocations" => self.force_locations = crate::read_force_locations(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for MulliganPerformed {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for NexusCardDrawn {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for NexusPlayed {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Faction", &self.faction);
        w.member("AtreidesPrescienceAspect", &self.atreides_prescience_aspect);
        w.member("TleilaxuForces", &self.tleilaxu_forces);
        w.member("TleilaxuSpecialForces", &self.tleilaxu_special_forces);
        w.member("TleilaxuAssignSkill", &self.tleilaxu_assign_skill);
        w.member("EcazFaction", &self.ecaz_faction);
        w.member("TleilaxuNumberOfSpecialForcesInLocation", &self.tleilaxu_number_of_special_forces_in_location);
        w.member("_tleilaxuHeroId", &self.tleilaxu_hero);
        w.member("_choamCardId", &self.choam_card);
        w.member("_ecazTerritoryId", &self.ecaz_territory);
        w.member("_moritaniTerritoryId", &self.moritani_territory);
        w.member("_tleilaxuLocationId", &self.tleilaxu_location);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Faction" => self.faction = FromJson::from_json(value)?,
            "AtreidesPrescienceAspect" => self.atreides_prescience_aspect = FromJson::from_json(value)?,
            "TleilaxuForces" => self.tleilaxu_forces = FromJson::from_json(value)?,
            "TleilaxuSpecialForces" => self.tleilaxu_special_forces = FromJson::from_json(value)?,
            "TleilaxuAssignSkill" => self.tleilaxu_assign_skill = FromJson::from_json(value)?,
            "EcazFaction" => self.ecaz_faction = FromJson::from_json(value)?,
            "TleilaxuNumberOfSpecialForcesInLocation" => self.tleilaxu_number_of_special_forces_in_location = FromJson::from_json(value)?,
            "_tleilaxuHeroId" => self.tleilaxu_hero = FromJson::from_json(value)?,
            "_choamCardId" => self.choam_card = FromJson::from_json(value)?,
            "_ecazTerritoryId" => self.ecaz_territory = FromJson::from_json(value)?,
            "_moritaniTerritoryId" => self.moritani_territory = FromJson::from_json(value)?,
            "_tleilaxuLocationId" => self.tleilaxu_location = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for NexusVoted {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for PerformBeneGesseritPlacement {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_targetId", &self.target);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_targetId" => self.target = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for PerformFremenSetup {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_toId", &self.to);
        w.member("_forceLocations", &self.force_locations);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_toId" => self.to = FromJson::from_json(value)?,
            "_forceLocations" => self.force_locations = crate::read_force_locations(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for PerformHmsMovement {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_targetId", &self.target);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_targetId" => self.target = FromJson::from_json(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for PerformHmsPlacement {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_targetId", &self.target);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_targetId" => self.target = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for PerformMoritaniSetup {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_targetId", &self.target);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_targetId" => self.target = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for PerformSetup {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Resources", &self.resources);
        w.member("_toId", &self.to);
        w.member("_forceLocations", &self.force_locations);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Resources" => self.resources = FromJson::from_json(value)?,
            "_toId" => self.to = FromJson::from_json(value)?,
            "_forceLocations" => self.force_locations = crate::read_force_locations(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for Planetology {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("AddOneToMovement", &self.add_one_to_movement);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "AddOneToMovement" => self.add_one_to_movement = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for PlayerReplaced {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("ToReplace", &self.to_replace);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "ToReplace" => self.to_replace = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for PoisonToothCancelled {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for PortableAntidoteUsed {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for Prescience {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Aspect", &self.aspect);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Aspect" => self.aspect = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for RaiseDeadPlayed {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("AmountOfForces", &self.amount_of_forces);
        w.member("AmountOfSpecialForces", &self.amount_of_special_forces);
        w.member("AssignSkill", &self.assign_skill);
        w.member("NumberOfSpecialForcesInLocation", &self.number_of_special_forces_in_location);
        w.member("_heroId", &self.hero);
        w.member("_locationId", &self.location);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "AmountOfForces" => self.amount_of_forces = FromJson::from_json(value)?,
            "AmountOfSpecialForces" => self.amount_of_special_forces = FromJson::from_json(value)?,
            "AssignSkill" => self.assign_skill = FromJson::from_json(value)?,
            "NumberOfSpecialForcesInLocation" => self.number_of_special_forces_in_location = FromJson::from_json(value)?,
            "_heroId" => self.hero = FromJson::from_json(value)?,
            "_locationId" => self.location = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for RecruitsPlayed {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for ReplacedCardWon {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for RequestTleilaxuRevival {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_heroId", &self.hero);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_heroId" => self.hero = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for ResidualPlayed {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for ResourcesAudited {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Target", &self.target);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Target" => self.target = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for ResourcesTransferred {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Resources", &self.resources);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Resources" => self.resources = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for Retreat {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Forces", &self.forces);
        w.member("SpecialForces", &self.special_forces);
        w.member("_targetId", &self.location);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Forces" => self.forces = FromJson::from_json(value)?,
            "SpecialForces" => self.special_forces = FromJson::from_json(value)?,
            "_targetId" => self.location = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for Revival {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("AmountOfForces", &self.amount_of_forces);
        w.member("AmountOfSpecialForces", &self.amount_of_special_forces);
        w.member("ExtraForcesPaidByEmperor", &self.extra_forces_paid_by_emperor);
        w.member("ExtraSpecialForcesPaidByEmperor", &self.extra_special_forces_paid_by_emperor);
        w.member("NumberOfForcesInLocation", &self.number_of_forces_in_location);
        w.member("NumberOfSpecialForcesInLocation", &self.number_of_special_forces_in_location);
        w.member("AssignSkill", &self.assign_skill);
        w.member("UsesEmperorSecretAlly", &self.uses_emperor_secret_ally);
        w.member("_locationId", &self.location);
        w.member("_heroId", &self.hero);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "AmountOfForces" => self.amount_of_forces = FromJson::from_json(value)?,
            "AmountOfSpecialForces" => self.amount_of_special_forces = FromJson::from_json(value)?,
            "ExtraForcesPaidByEmperor" => self.extra_forces_paid_by_emperor = FromJson::from_json(value)?,
            "ExtraSpecialForcesPaidByEmperor" => self.extra_special_forces_paid_by_emperor = FromJson::from_json(value)?,
            "NumberOfForcesInLocation" => self.number_of_forces_in_location = FromJson::from_json(value)?,
            "NumberOfSpecialForcesInLocation" => self.number_of_special_forces_in_location = FromJson::from_json(value)?,
            "AssignSkill" => self.assign_skill = FromJson::from_json(value)?,
            "UsesEmperorSecretAlly" => self.uses_emperor_secret_ally = FromJson::from_json(value)?,
            "_locationId" => self.location = FromJson::from_json(value)?,
            "_heroId" => self.hero = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for RicheseAnnouncesAuction {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("First", &self.first);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "First" => self.first = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for RicheseAnnouncesBlackMarket {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("AuctionType", &self.auction_type);
        w.member("Direction", &self.direction);
        w.member("_cardId", &self.card);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "AuctionType" => self.auction_type = FromJson::from_json(value)?,
            "Direction" => self.direction = FromJson::from_json(value)?,
            "_cardId" => self.card = FromJson::from_json(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for RicheseGaveCard {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_cardId", &self.card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_cardId" => self.card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for RicheseKeepsUnsoldCard {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for RicheseRevealedNoField {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for RicheseSpecifiesAuction {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("AuctionType", &self.auction_type);
        w.member("Direction", &self.direction);
        w.member("_cardId", &self.card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "AuctionType" => self.auction_type = FromJson::from_json(value)?,
            "Direction" => self.direction = FromJson::from_json(value)?,
            "_cardId" => self.card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for RockWasMelted {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Kill", &self.kill);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Kill" => self.kill = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for SetIncreasedRevivalLimits {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Factions", &self.factions);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Factions" => self.factions = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for SetShipmentPermission {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Factions", &self.factions);
        w.member("Permission", &self.permission);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Factions" => self.factions = FromJson::from_json(value)?,
            "Permission" => self.permission = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for Shipment {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("ShipmentType", &self.shipment_type);
        w.member("ForceAmount", &self.force_amount);
        w.member("SpecialForceAmount", &self.special_force_amount);
        w.member("SmuggledAmount", &self.smuggled_amount);
        w.member("SmuggledSpecialAmount", &self.smuggled_special_amount);
        w.member("AllyContributionAmount", &self.ally_contribution_amount);
        w.member("UseRicheseSecretAlly", &self.use_richese_secret_ally);
        w.member("_toId", &self.to);
        w.member("_fromId", &self.from);
        w.member("_noFieldValue", &self.no_field_value);
        w.member("_cunningNoFieldValue", &self.cunning_no_field_value);
        w.member("_karmaCardId", &self.karma_card);
        w.member("_forceLocations", &self.force_locations);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "ShipmentType" => self.shipment_type = FromJson::from_json(value)?,
            "ForceAmount" => self.force_amount = FromJson::from_json(value)?,
            "SpecialForceAmount" => self.special_force_amount = FromJson::from_json(value)?,
            "SmuggledAmount" => self.smuggled_amount = FromJson::from_json(value)?,
            "SmuggledSpecialAmount" => self.smuggled_special_amount = FromJson::from_json(value)?,
            "AllyContributionAmount" => self.ally_contribution_amount = FromJson::from_json(value)?,
            "UseRicheseSecretAlly" => self.use_richese_secret_ally = FromJson::from_json(value)?,
            "_toId" => self.to = FromJson::from_json(value)?,
            "_fromId" => self.from = FromJson::from_json(value)?,
            "_noFieldValue" => self.no_field_value = FromJson::from_json(value)?,
            "_cunningNoFieldValue" => self.cunning_no_field_value = FromJson::from_json(value)?,
            "_karmaCardId" => self.karma_card = FromJson::from_json(value)?,
            "_forceLocations" => self.force_locations = crate::read_force_locations(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for SkillAssigned {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Skill", &self.skill);
        w.member("_leaderId", &self.leader);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Skill" => self.skill = FromJson::from_json(value)?,
            "_leaderId" => self.leader = FromJson::from_json(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for StormDialled {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Amount", &self.amount);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Amount" => self.amount = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for StormSpellPlayed {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("MoveAmount", &self.move_amount);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "MoveAmount" => self.move_amount = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for SwitchedSkilledLeader {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for TakeLosses {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("ForceAmount", &self.force_amount);
        w.member("SpecialForceAmount", &self.special_force_amount);
        w.member("UseUselessCard", &self.use_useless_card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "ForceAmount" => self.force_amount = FromJson::from_json(value)?,
            "SpecialForceAmount" => self.special_force_amount = FromJson::from_json(value)?,
            "UseUselessCard" => self.use_useless_card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for TerrorPlanted {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Type", &self.type_);
        w.member("_territoryId", &self.stronghold);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Type" => self.type_ = FromJson::from_json(value)?,
            "_territoryId" => self.stronghold = FromJson::from_json(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for TerrorRevealed {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("AllianceOffered", &self.alliance_offered);
        w.member("Type", &self.type_);
        w.member("ForcesInSneakAttack", &self.forces_in_sneak_attack);
        w.member("RobberyTakesCard", &self.robbery_takes_card);
        w.member("_sneakAttackTo", &self.sneak_attack_to);
        w.member("_cardToGiveInSabotageId", &self.card_to_give_in_sabotage);
        w.member("Passed", &self.passed);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "AllianceOffered" => self.alliance_offered = FromJson::from_json(value)?,
            "Type" => self.type_ = FromJson::from_json(value)?,
            "ForcesInSneakAttack" => self.forces_in_sneak_attack = FromJson::from_json(value)?,
            "RobberyTakesCard" => self.robbery_takes_card = FromJson::from_json(value)?,
            "_sneakAttackTo" => self.sneak_attack_to = FromJson::from_json(value)?,
            "_cardToGiveInSabotageId" => self.card_to_give_in_sabotage = FromJson::from_json(value)?,
            "Passed" => self.passed = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for TestingStationUsed {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("ValueAdded", &self.value_added);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "ValueAdded" => self.value_added = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for Thought {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_cardId", &self.card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_cardId" => self.card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for ThoughtAnswered {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_cardId", &self.card);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_cardId" => self.card = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for ThumperPlayed {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for TraitorDiscarded {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_traitorId", &self.traitor);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_traitorId" => self.traitor = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for TraitorsSelected {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("_traitorId", &self.selected_traitor);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "_traitorId" => self.selected_traitor = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for TreacheryCalled {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("TraitorCalled", &self.traitor_called);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "TraitorCalled" => self.traitor_called = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for VoiceDeclined {
    fn write_members(&self, _w: &mut JsonWriter) {
    }

    fn read_member(&mut self, _key: &str, _value: &JsonValue) -> Result<bool, JsonError> {
        Ok(false)
    }
}

impl JsonObject for Voice {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("Must", &self.must);
        w.member("Type", &self.type_);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "Must" => self.must = FromJson::from_json(value)?,
            "Type" => self.type_ = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl JsonObject for GameSettings {
    fn write_members(&self, w: &mut JsonWriter) {
        w.member("NumberOfPlayers", &self.number_of_players);
        w.member("MaximumTurns", &self.maximum_turns);
        w.member("InitialRules", &self.initial_rules);
        w.member("AllowedFactionsInPlay", &self.allowed_factions_in_play);
        w.member("AutoOpenEmptySeats", &self.auto_open_empty_seats);
        w.member("AsyncPlay", &self.async_play);
        w.member("AsyncPlayMessageIntervalMinutes", &self.async_play_message_interval_minutes);
    }

    fn read_member(&mut self, key: &str, value: &JsonValue) -> Result<bool, JsonError> {
        match key {
            "NumberOfPlayers" => self.number_of_players = FromJson::from_json(value)?,
            "MaximumTurns" => self.maximum_turns = FromJson::from_json(value)?,
            "InitialRules" => self.initial_rules = FromJson::from_json(value)?,
            "AllowedFactionsInPlay" => self.allowed_factions_in_play = FromJson::from_json(value)?,
            "AutoOpenEmptySeats" => self.auto_open_empty_seats = FromJson::from_json(value)?,
            "AsyncPlay" => self.async_play = FromJson::from_json(value)?,
            "AsyncPlayMessageIntervalMinutes" => self.async_play_message_interval_minutes = FromJson::from_json(value)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}
