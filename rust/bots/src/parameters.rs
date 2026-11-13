use crate::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct BotParameters {
    pub bidding_resources_to_keep_when_card_is_perfect: i32,
    pub bidding_resources_to_keep_when_card_isnt_perfect: i32,
    pub bidding_passing_treshold: i32,
    pub karma_save_card_to_use_special_karma_ability: bool,
    pub shipment_minimum_other_players_i_trust_to_prevent_a_win: i32,
    pub shipment_dial_shortage_to_accept: f32,
    pub shipment_minimum_resources_to_keep_for_battle: i32,
    pub shipment_max_enemy_force_strength_fighting_for_spice: f32,
    pub shipment_expected_storm_moves_when_unknown: i32,
    pub shipment_dial_for_extra_forces_to_ship: i32,
    pub battle_maximum_unsupported_forces: i32,
    pub battle_mimimum_chance_to_assume_enemy_hero_survives: f32,
    pub battle_mimimum_chance_to_assume_my_leader_survives: f32,
    pub battle_dial_shortage_threshold_for_throwing: f32,
}

impl BotParameters {
    pub const PENALTY_FOR_ATTACKING_BOTS: f32 = 6.0;

    /// A fixed parameter set. Nothing mutates it, so building a fresh value on each call is exact (hazards.md 16.8).
    pub fn harkonnen_parameters() -> BotParameters {
        BotParameters {
            bidding_resources_to_keep_when_card_is_perfect: 1,
            bidding_resources_to_keep_when_card_isnt_perfect: 4,
            bidding_passing_treshold: 0,
            karma_save_card_to_use_special_karma_ability: true,
            shipment_minimum_other_players_i_trust_to_prevent_a_win: 4,
            shipment_dial_shortage_to_accept: 6.0,
            shipment_minimum_resources_to_keep_for_battle: 1,
            shipment_dial_for_extra_forces_to_ship: 1,
            shipment_max_enemy_force_strength_fighting_for_spice: 4.0,
            shipment_expected_storm_moves_when_unknown: 2,
            battle_maximum_unsupported_forces: 5,
            battle_mimimum_chance_to_assume_enemy_hero_survives: 0.6,
            battle_mimimum_chance_to_assume_my_leader_survives: 0.4,
            battle_dial_shortage_threshold_for_throwing: 3.0,
        }
    }

    /// A fixed parameter set. Nothing mutates it, so building a fresh value on each call is exact (hazards.md 16.8).
    pub fn atreides_parameters() -> BotParameters {
        BotParameters {
            bidding_resources_to_keep_when_card_is_perfect: 1,
            bidding_resources_to_keep_when_card_isnt_perfect: 4,
            bidding_passing_treshold: 2,
            karma_save_card_to_use_special_karma_ability: false,
            shipment_minimum_other_players_i_trust_to_prevent_a_win: 3,
            shipment_dial_shortage_to_accept: 2.0,
            shipment_minimum_resources_to_keep_for_battle: 1,
            shipment_dial_for_extra_forces_to_ship: 1,
            shipment_max_enemy_force_strength_fighting_for_spice: 2.0,
            shipment_expected_storm_moves_when_unknown: 3,
            battle_maximum_unsupported_forces: 3,
            battle_mimimum_chance_to_assume_enemy_hero_survives: 0.5,
            battle_mimimum_chance_to_assume_my_leader_survives: 0.5,
            battle_dial_shortage_threshold_for_throwing: 3.0,
        }
    }

    /// A fixed parameter set. Nothing mutates it, so building a fresh value on each call is exact (hazards.md 16.8).
    pub fn fremen_parameters() -> BotParameters {
        BotParameters {
            bidding_resources_to_keep_when_card_is_perfect: 0,
            bidding_resources_to_keep_when_card_isnt_perfect: 0,
            bidding_passing_treshold: 4,
            karma_save_card_to_use_special_karma_ability: false,
            shipment_minimum_other_players_i_trust_to_prevent_a_win: 2,
            shipment_dial_shortage_to_accept: 4.0,
            shipment_minimum_resources_to_keep_for_battle: 0,
            shipment_dial_for_extra_forces_to_ship: 6,
            shipment_max_enemy_force_strength_fighting_for_spice: 8.0,
            shipment_expected_storm_moves_when_unknown: 0,
            battle_maximum_unsupported_forces: 20,
            battle_mimimum_chance_to_assume_enemy_hero_survives: 0.4,
            battle_mimimum_chance_to_assume_my_leader_survives: 0.5,
            battle_dial_shortage_threshold_for_throwing: 4.0,
        }
    }

    /// A fixed parameter set. Nothing mutates it, so building a fresh value on each call is exact (hazards.md 16.8).
    pub fn emperor_parameters() -> BotParameters {
        BotParameters {
            bidding_resources_to_keep_when_card_is_perfect: 1,
            bidding_resources_to_keep_when_card_isnt_perfect: 6,
            bidding_passing_treshold: 3,
            karma_save_card_to_use_special_karma_ability: true,
            shipment_minimum_other_players_i_trust_to_prevent_a_win: 4,
            shipment_dial_shortage_to_accept: 3.0,
            shipment_minimum_resources_to_keep_for_battle: 2,
            shipment_dial_for_extra_forces_to_ship: 2,
            shipment_max_enemy_force_strength_fighting_for_spice: 0.0,
            shipment_expected_storm_moves_when_unknown: 6,
            battle_maximum_unsupported_forces: 3,
            battle_mimimum_chance_to_assume_enemy_hero_survives: 0.4,
            battle_mimimum_chance_to_assume_my_leader_survives: 0.6,
            battle_dial_shortage_threshold_for_throwing: 3.0,
        }
    }

    /// A fixed parameter set. Nothing mutates it, so building a fresh value on each call is exact (hazards.md 16.8).
    pub fn guild_parameters() -> BotParameters {
        BotParameters {
            bidding_resources_to_keep_when_card_is_perfect: 1,
            bidding_resources_to_keep_when_card_isnt_perfect: 8,
            bidding_passing_treshold: 0,
            karma_save_card_to_use_special_karma_ability: true,
            shipment_minimum_other_players_i_trust_to_prevent_a_win: 2,
            shipment_dial_shortage_to_accept: 0.0,
            shipment_minimum_resources_to_keep_for_battle: 4,
            shipment_dial_for_extra_forces_to_ship: 2,
            shipment_max_enemy_force_strength_fighting_for_spice: 0.0,
            shipment_expected_storm_moves_when_unknown: 6,
            battle_maximum_unsupported_forces: 0,
            battle_mimimum_chance_to_assume_enemy_hero_survives: 0.4,
            battle_mimimum_chance_to_assume_my_leader_survives: 0.6,
            battle_dial_shortage_threshold_for_throwing: 6.0,
        }
    }

    /// A fixed parameter set. Nothing mutates it, so building a fresh value on each call is exact (hazards.md 16.8).
    pub fn bene_gesserit_parameters() -> BotParameters {
        BotParameters {
            bidding_resources_to_keep_when_card_is_perfect: 1,
            bidding_resources_to_keep_when_card_isnt_perfect: 7,
            bidding_passing_treshold: 0,
            karma_save_card_to_use_special_karma_ability: false,
            shipment_minimum_other_players_i_trust_to_prevent_a_win: 2,
            shipment_dial_shortage_to_accept: 4.0,
            shipment_minimum_resources_to_keep_for_battle: 2,
            shipment_dial_for_extra_forces_to_ship: 2,
            shipment_max_enemy_force_strength_fighting_for_spice: 4.0,
            shipment_expected_storm_moves_when_unknown: 6,
            battle_maximum_unsupported_forces: 1,
            battle_mimimum_chance_to_assume_enemy_hero_survives: 0.4,
            battle_mimimum_chance_to_assume_my_leader_survives: 0.55,
            battle_dial_shortage_threshold_for_throwing: 6.0,
        }
    }

    /// A fixed parameter set. Nothing mutates it, so building a fresh value on each call is exact (hazards.md 16.8).
    pub fn ixian_parameters() -> BotParameters {
        BotParameters {
            bidding_resources_to_keep_when_card_is_perfect: 1,
            bidding_resources_to_keep_when_card_isnt_perfect: 5,
            bidding_passing_treshold: 4,
            karma_save_card_to_use_special_karma_ability: false,
            shipment_minimum_other_players_i_trust_to_prevent_a_win: 3,
            shipment_dial_shortage_to_accept: 4.0,
            shipment_minimum_resources_to_keep_for_battle: 0,
            shipment_dial_for_extra_forces_to_ship: 1,
            shipment_max_enemy_force_strength_fighting_for_spice: 4.0,
            shipment_expected_storm_moves_when_unknown: 5,
            battle_maximum_unsupported_forces: 2,
            battle_mimimum_chance_to_assume_enemy_hero_survives: 0.6,
            battle_mimimum_chance_to_assume_my_leader_survives: 0.4,
            battle_dial_shortage_threshold_for_throwing: 4.0,
        }
    }

    /// A fixed parameter set. Nothing mutates it, so building a fresh value on each call is exact (hazards.md 16.8).
    pub fn tleilaxu_parameters() -> BotParameters {
        BotParameters {
            bidding_resources_to_keep_when_card_is_perfect: 1,
            bidding_resources_to_keep_when_card_isnt_perfect: 5,
            bidding_passing_treshold: 3,
            karma_save_card_to_use_special_karma_ability: false,
            shipment_minimum_other_players_i_trust_to_prevent_a_win: 2,
            shipment_dial_shortage_to_accept: 8.0,
            shipment_minimum_resources_to_keep_for_battle: 0,
            shipment_dial_for_extra_forces_to_ship: 3,
            shipment_max_enemy_force_strength_fighting_for_spice: 6.0,
            shipment_expected_storm_moves_when_unknown: 0,
            battle_maximum_unsupported_forces: 8,
            battle_mimimum_chance_to_assume_enemy_hero_survives: 0.3,
            battle_mimimum_chance_to_assume_my_leader_survives: 0.6,
            battle_dial_shortage_threshold_for_throwing: 6.0,
        }
    }

    /// A fixed parameter set. Nothing mutates it, so building a fresh value on each call is exact (hazards.md 16.8).
    pub fn choam_parameters() -> BotParameters {
        BotParameters {
            bidding_resources_to_keep_when_card_is_perfect: 1,
            bidding_resources_to_keep_when_card_isnt_perfect: 8,
            bidding_passing_treshold: 3,
            karma_save_card_to_use_special_karma_ability: true,
            shipment_minimum_other_players_i_trust_to_prevent_a_win: 2,
            shipment_dial_shortage_to_accept: 4.0,
            shipment_minimum_resources_to_keep_for_battle: 0,
            shipment_dial_for_extra_forces_to_ship: 2,
            shipment_max_enemy_force_strength_fighting_for_spice: 0.0,
            shipment_expected_storm_moves_when_unknown: 4,
            battle_maximum_unsupported_forces: 1,
            battle_mimimum_chance_to_assume_enemy_hero_survives: 0.4,
            battle_mimimum_chance_to_assume_my_leader_survives: 0.55,
            battle_dial_shortage_threshold_for_throwing: 3.0,
        }
    }

    /// A fixed parameter set. Nothing mutates it, so building a fresh value on each call is exact (hazards.md 16.8).
    pub fn richese_parameters() -> BotParameters {
        BotParameters {
            bidding_resources_to_keep_when_card_is_perfect: 1,
            bidding_resources_to_keep_when_card_isnt_perfect: 5,
            bidding_passing_treshold: 4,
            karma_save_card_to_use_special_karma_ability: false,
            shipment_minimum_other_players_i_trust_to_prevent_a_win: 4,
            shipment_dial_shortage_to_accept: 4.0,
            shipment_minimum_resources_to_keep_for_battle: 0,
            shipment_dial_for_extra_forces_to_ship: 1,
            shipment_max_enemy_force_strength_fighting_for_spice: 4.0,
            shipment_expected_storm_moves_when_unknown: 4,
            battle_maximum_unsupported_forces: 1,
            battle_mimimum_chance_to_assume_enemy_hero_survives: 0.4,
            battle_mimimum_chance_to_assume_my_leader_survives: 0.5,
            battle_dial_shortage_threshold_for_throwing: 6.0,
        }
    }

    /// A fixed parameter set. Nothing mutates it, so building a fresh value on each call is exact (hazards.md 16.8).
    pub fn ecaz_parameters() -> BotParameters {
        BotParameters {
            bidding_resources_to_keep_when_card_is_perfect: 1,
            bidding_resources_to_keep_when_card_isnt_perfect: 5,
            bidding_passing_treshold: 4,
            karma_save_card_to_use_special_karma_ability: false,
            shipment_minimum_other_players_i_trust_to_prevent_a_win: 2,
            shipment_dial_shortage_to_accept: 2.0,
            shipment_minimum_resources_to_keep_for_battle: 0,
            shipment_dial_for_extra_forces_to_ship: 2,
            shipment_max_enemy_force_strength_fighting_for_spice: 0.0,
            shipment_expected_storm_moves_when_unknown: 4,
            battle_maximum_unsupported_forces: 2,
            battle_mimimum_chance_to_assume_enemy_hero_survives: 0.5,
            battle_mimimum_chance_to_assume_my_leader_survives: 0.5,
            battle_dial_shortage_threshold_for_throwing: 3.0,
        }
    }

    /// A fixed parameter set. Nothing mutates it, so building a fresh value on each call is exact (hazards.md 16.8).
    pub fn moritani_parameters() -> BotParameters {
        BotParameters {
            bidding_resources_to_keep_when_card_is_perfect: 1,
            bidding_resources_to_keep_when_card_isnt_perfect: 5,
            bidding_passing_treshold: 4,
            karma_save_card_to_use_special_karma_ability: false,
            shipment_minimum_other_players_i_trust_to_prevent_a_win: 4,
            shipment_dial_shortage_to_accept: 4.0,
            shipment_minimum_resources_to_keep_for_battle: 0,
            shipment_dial_for_extra_forces_to_ship: 2,
            shipment_max_enemy_force_strength_fighting_for_spice: 5.0,
            shipment_expected_storm_moves_when_unknown: 4,
            battle_maximum_unsupported_forces: 2,
            battle_mimimum_chance_to_assume_enemy_hero_survives: 0.4,
            battle_mimimum_chance_to_assume_my_leader_survives: 0.5,
            battle_dial_shortage_threshold_for_throwing: 4.0,
        }
    }

    pub fn get_default_parameters(f: Faction) -> BotParameters {
        // MemberwiseClone of a struct of value types is a plain copy.
        match f {
            Faction::Harkonnen => Self::harkonnen_parameters(),
            Faction::BeneGesserit => Self::bene_gesserit_parameters(),
            Faction::Atreides => Self::atreides_parameters(),
            Faction::Fremen => Self::fremen_parameters(),
            Faction::Emperor => Self::emperor_parameters(),
            Faction::Guild => Self::guild_parameters(),
            Faction::Ixian => Self::ixian_parameters(),
            Faction::Tleilaxu => Self::tleilaxu_parameters(),
            Faction::Choam => Self::choam_parameters(),
            Faction::Richese => Self::richese_parameters(),
            Faction::Ecaz => Self::ecaz_parameters(),
            Faction::Moritani => Self::moritani_parameters(),
            _ => Self::harkonnen_parameters(),
        }
    }

}
