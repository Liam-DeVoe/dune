//! Port of the `#region FactionManual` of Model/Skin.cs: each faction's rules sheet, as HTML
//! (`GetFactionInfo_HTML`) or as its parts (`GetFactionSheet`).
//!
//! C# notes the parts in two private fields (`_sheetBeingRead`, `_sheetInAdvanced`) while a
//! template is built. Here they are a [`SheetReader`] passed through the template, which gives
//! the same results without making `Skin` mutable.

use treachery_engine::enumerations::*;
use treachery_engine::messages::expression::ExpressionElement;
use treachery_engine::game::Game;
use treachery_engine::identity::IFetcher;
use treachery_engine::cards::treachery_card_manager::TreacheryCardManager;

use crate::skin::{to_upper_invariant, Skin};

/// `public record FactionSheet(string AtStart, int Resources, int FreeRevival, string Theme,
/// List<string> Advantages, List<string> AdvancedAdvantages, object[] Parameters)`: the parts of
/// a faction's rules sheet. `at_start` still has the sheet's `{n}` placeholders, for `parameters`.
#[derive(Clone, Debug, PartialEq)]
pub struct FactionSheet {
    pub at_start: String,
    pub resources: i32,
    pub free_revival: i32,
    pub theme: String,
    pub advantages: Vec<String>,
    pub advanced_advantages: Vec<String>,
    pub parameters: Vec<ExpressionElement>,
}

/// `_sheetBeingRead` and `_sheetInAdvanced`.
pub struct SheetReader {
    sheet: Option<FactionSheet>,
    in_advanced: bool,
}

/// The `title` of the `Advantage(object title, ...)` overloads: a string, or anything
/// `Describe` takes.
pub enum Title {
    Str(&'static str),
    Object(ExpressionElement),
}

impl From<&'static str> for Title {
    fn from(s: &'static str) -> Self {
        Title::Str(s)
    }
}

macro_rules! title_from {
    ($($t:ty),*) => {$(
        impl From<$t> for Title {
            fn from(v: $t) -> Self {
                Title::Object(ExpressionElement::from(v))
            }
        }
    )*};
}

title_from!(Concept, MainPhase, TreacheryCardType, FactionSpecialForce, FactionForce, Faction);

impl Skin {
    /// `GetFactionInfo_HTML(Game g, Faction f, string? sheetStyle = null)`: the faction's rules
    /// sheet. `sheet_style` replaces the sheet's own inline style (`FACTION_INFORMATIONCARDSTYLE`).
    pub fn get_faction_info_html(&self, g: &Game, f: Faction, sheet_style: Option<&str>) -> String {
        let parameters = self.faction_sheet_parameters(g, sheet_style.unwrap_or(&self.faction_informationcardstyle));
        let template = self.faction_sheet_template(g, f, &mut SheetReader { sheet: None, in_advanced: false });
        if template.is_empty() {
            String::new()
        } else {
            self.with_spice_icons(&self.format_in(&template, &parameters, Some(g)), 20, false)
        }
    }

    /// `GetFactionSheet(Game g, Faction f)`: the parts of the sheet a summary needs, or `None`
    /// for a faction without one.
    pub fn get_faction_sheet(&self, g: &Game, f: Faction) -> Option<FactionSheet> {
        let mut r = SheetReader {
            sheet: Some(FactionSheet {
                at_start: String::new(),
                resources: 0,
                free_revival: 0,
                theme: String::new(),
                advantages: Vec::new(),
                advanced_advantages: Vec::new(),
                parameters: self.faction_sheet_parameters(g, ""),
            }),
            in_advanced: false,
        };
        if self.faction_sheet_template(g, f, &mut r).is_empty() { None } else { r.sheet }
    }

    /// `private string FactionSheetTemplate(Game g, Faction f)`
    fn faction_sheet_template(&self, g: &Game, f: Faction, r: &mut SheetReader) -> String {
        r.in_advanced = false;
        match f {
            Faction::Atreides => self.get_atreides_template(g, r),
            Faction::Harkonnen => self.get_harkonnen_template(g, r),
            Faction::Fremen => self.get_fremen_template(g, r),
            Faction::Emperor => self.get_emperor_template(g, r),
            Faction::Guild => self.get_guild_template(g, r),
            Faction::BeneGesserit => self.get_bene_gesserit_template(g, r),
            Faction::Tleilaxu => self.get_tleilaxu_template(g, r),
            Faction::Ixian => self.get_ixian_template(g, r),
            Faction::Choam => self.get_choam_template(g, r),
            Faction::Richese => self.get_richese_template(g, r),
            Faction::Ecaz => self.get_ecaz_template(g, r),
            Faction::Moritani => self.get_moritani_template(g, r),
            _ => String::new(),
        }
    }

    /// `private object[] FactionSheetParameters(Game g, string sheetStyle)`
    fn faction_sheet_parameters(&self, g: &Game, sheet_style: &str) -> Vec<ExpressionElement> {
        let card_name = |id: i32| {
            let skin_id = TreacheryCardManager::lookup().find(id).map(|c| c.skin_id()).unwrap_or(-1);
            self.treachery_card_name_str.get_value_or_default(skin_id, "?")
        };
        let m = &g.map;
        crate::args![
            Faction::Atreides,                                         //0
            Faction::Harkonnen,                                         //1
            Faction::Emperor,                                           //2
            Faction::Fremen,                                        //3
            Faction::Guild,                                        //4
            Faction::BeneGesserit,                                          //5
            m.arrakeen(),                                           //6
            m.carthag(),                                            //7
            m.tueks_sietch(),                                       //8
            m.sietch_tabr(),                                        //9
            m.false_wall_south(),                                   //10
            m.false_wall_west(),                                    //11
            m.the_great_flat(),                                     //12
            TreacheryCardType::Mercenary,                           //13
            TreacheryCardType::Useless,                             //14
            self.concept_str.index(Concept::Monster).clone(),       //15
            self.concept_str.index(Concept::Resource).clone(),      //16
            TreacheryCardType::Shield,                              //17
            TreacheryCardType::Laser,                               //18
            TreacheryCardType::Karma,                               //19
            self.concept_str.index(Concept::Messiah).clone(),       //20
            self.main_phase_str.index(MainPhase::ShipmentAndMove).clone(), //21
            m.habbanya_sietch(),                                    //22
            self.special_force_name_str.index(Faction::Fremen).clone(), //23
            self.special_force_name_str.index(Faction::Emperor).clone(), //24
            sheet_style,                                            //25
            m.polar_sink(),                                         //26
            Faction::Tleilaxu,                                        //27
            Faction::Ixian,                                          //28
            self.force_name_str.index(Faction::Ixian).clone(),       //29
            self.special_force_name_str.index(Faction::Ixian).clone(), //30
            Concept::Graveyard,                                     //31
            FactionForce::Ixian,                                     //32
            FactionSpecialForce::Ixian,                              //33
            m.hidden_mobile_stronghold(),                           //34
            Faction::Richese,                                         //35
            Faction::Choam,                                         //36
            card_name(TreacheryCardManager::CARD_BALISET),          //37
            card_name(TreacheryCardManager::CARD_JUBBA_CLOAK),      //38
            card_name(TreacheryCardManager::CARD_KULL_WAHAD),       //39
            card_name(TreacheryCardManager::CARD_KULON),            //40
            card_name(TreacheryCardManager::CARD_LALALA),           //41
            card_name(TreacheryCardManager::CARD_TRIP_TO_GAMONT),   //42
            self.main_phase_str.index(MainPhase::Contemplate).clone(), //43
            self.person_name_str.index(1036).clone(),               //44
            self.main_phase_str.index(MainPhase::Bidding).clone(),  //45
            self.main_phase_str.index(MainPhase::Battle).clone(),   //46
            self.main_phase_str.index(MainPhase::Charity).clone(),  //47
            Faction::Ecaz,                                          //48
            Faction::Moritani,                                          //49
            m.imperial_basin(),                                     //50
        ]
    }

    /// `private string SheetHeader(string atStart, int resources, int freeRevival, string theme)`
    fn sheet_header(&self, r: &mut SheetReader, at_start: &str, resources: i32, free_revival: i32, theme: &str) -> String {
        if let Some(sheet) = r.sheet.as_mut() {
            sheet.at_start = at_start.to_string();
            sheet.resources = resources;
            sheet.free_revival = free_revival;
            sheet.theme = theme.to_string();
        }

        format!(
            "<div style='{{25}}'><p><strong>AT START:</strong> {at_start}. Start with {resources} {{16}}.</p><p><strong>FREE REVIVAL:</strong> {free_revival}</p><h5>{theme}</h5>"
        )
    }

    /// `Advantage(string advantageName, ...)` and `Advantage(object title, ...)` with a `condition`.
    fn advantage(&self, r: &mut SheetReader, title: Title, advantage_description: &str, condition: bool) -> String {
        let advantage_name = self.title_text(title);
        if condition {
            note_advantage(r, &advantage_name);
        }
        if condition {
            format!("<p><strong>{}</strong> - {advantage_description}", to_upper_invariant(&advantage_name))
        } else {
            String::new()
        }
    }

    /// `Advantage(Game g, Rule rule, string advantageName, ...)` and `Advantage(Game g, Rule rule, object title, ...)`
    fn advantage_rule(&self, r: &mut SheetReader, g: &Game, rule: Rule, title: Title, advantage_description: &str) -> String {
        let advantage_name = self.title_text(title);
        if g.applicable(rule) {
            note_advantage(r, &advantage_name);
        }
        if g.applicable(rule) {
            format!("<p><strong>{}</strong> - {advantage_description}", to_upper_invariant(&advantage_name))
        } else {
            String::new()
        }
    }

    fn title_text(&self, title: Title) -> String {
        match title {
            Title::Str(s) => s.to_string(),
            Title::Object(o) => self.describe(&o),
        }
    }

    /// `private string AdvancedHeader(Game g, params Rule[] rules)`
    fn advanced_header(&self, r: &mut SheetReader, g: &Game, rules: &[Rule]) -> String {
        if rules.iter().any(|x| g.applicable(*x)) {
            r.in_advanced = true;
        }
        if rules.iter().any(|x| g.applicable(*x)) { "<h5>Advanced Game Advantages</h5>".to_string() } else { String::new() }
    }

    /// `private string GetAtreidesTemplate(Game g)`
    fn get_atreides_template(&self, g: &Game, r: &mut SheetReader) -> String {
        [
            self.sheet_header(r, "10 tokens in {6} and 10 in reserve (off-planet)", 10, 2, "You have limited prescience"),
            self.advantage(r, Title::from("Bidding Prescience"), "you see the treachery card on bid.", true),
            self.advantage(r, Title::from("Spice Blow Prescience"), "you see the top card of the {16} deck.", true),
            self.advantage(r, Title::from("Battle Prescience"), "you may force your opponent to tell you your choice of one of the four elements he will use in his battle plan against you; the leader, the weapon, the defense or the number dialed. If your opponent tells you that he is not playing a weapon or defense, you may not ask something else.", true),
            self.advanced_header(r, g, &[Rule::AtreidesMessiah, Rule::AdvancedKarama]),
            self.advantage_rule(r, g, Rule::AtreidesMessiah, Title::from(Concept::Messiah), "after losing a total of at least 7 forces in battle, you may use the {20}. It cannot be used alone in battle but may add its +2 strength to any one leader or {13} per turn. If the leader or {13} is killed, the {20} has no effect in the battle. {20} can only be killed if blown up by a {18}/{17} explosion. A leader accompanied by {20} cannot turn traitor. If killed, the {20} must be revived like any other leader. The {20} has no effect on {0} leader revival."),
            self.advantage_rule(r, g, Rule::AdvancedKarama, Title::from(TreacheryCardType::Karma), "you may use {19} to see one player's entire battle plan."),
            r#"<h5>Alliance</h5>
                <p>You may assist your allies by forcing their opponents to tell them one element of their battle plan.</p>
                <h5>Strategy</h5>
                <p>You are handicapped by the fact that you must both purchase cards and ship onto the planet, and you have no source of income other than {16}. This will keep you in constant battles. Since you start from {6} you have the movement advantage of 3 from the outset, and it is wise to protect this. Your prescience allows you to avoid being devoured by {15} and helps you to get some slight head start on the {16} blow. In addition, you can gain some slight advantage over those who would do battle with you by your foreknowledge of one element of their battle plan.</p>
                </div>"#.to_string(),
        ]
        .concat()
    }

    /// `private string GetHarkonnenTemplate(Game g)`
    fn get_harkonnen_template(&self, g: &Game, r: &mut SheetReader) -> String {
        [
            self.sheet_header(r, "10 tokens in {7} and 10 tokens in reserve (off-planet)", 10, 2, "You excel in treachery"),
            self.advantage(r, Title::from("Traitors"), "you keep all traitors you draw at the start of the game.", true),
            self.advantage(r, Title::from("Treachery"), "you may hold up to 8 treachery cards. At start of game, you are dealt 2 cards instead of 1, and every time you buy a card you get an extra card free from the deck (if you have less than 8 total).", true),
            self.advantage_rule(r, g, Rule::HarkonnenMulligan, Title::from("Mulligan"), "at start, when you draw 2 or more of your own leaders as traitors, you may shuffle them back and redraw four traitors."),
            self.advanced_header(r, g, &[Rule::HarkonnenCapturesOrKillsLeaders, Rule::AdvancedKarama]),
            self.advantage_rule(r, g, Rule::HarkonnenCapturesOrKillsLeaders, Title::from("Captured leaders"), "every time you win a battle you can select randomly one leader from the loser (including the leader used in battle, if not killed, but excluding all leaders already used elsewhere that turn). You can kill that leader for 2 {16}; or use the leader once in a battle after which you must return it to the original owner. If all your own leaders have been killed, all captured leaders are immediately returned to their original owners. Killed captured leaders are put in the 'tanks' from which the original owners can revive them (subject to the revival rules)."),
            self.advantage_rule(r, g, Rule::AdvancedKarama, Title::from(TreacheryCardType::Karma), "during the Bidding phase, you may use {19} to take at random up to all treachery cards of any one player of your choice, as long has your maximum hand size is not exceeded. Then, for each card you took you must give him one of your cards in return."),
            r#"<h5>Alliance</h5>
                <p>Your traitors may be used against your allies opponents.</p>
                <h5>Strategy</h5>
                <p>Your major handicap is your difficulty in obtaining {16}. You are at your greatest relative strength at the beginning of the game and should capitalize on this fact by quickly buying as many treachery cards as you can, and then surging into battle. Since you get 2 cards for every one you bid for, you can afford to bid a little higher than most, but if you spend too lavishly at first you will not have enough {16} to ship in forces or buy more cards later. The large number of cards you may hold will increase your chances of holding worthless cards. To counteract this you should pick your battles, both to unload cards and to flush out the traitors in your pay.</p>
                </div>"#.to_string(),
        ]
        .concat()
    }

    /// `private string GetFremenTemplate(Game g)`
    fn get_fremen_template(&self, g: &Game, r: &mut SheetReader) -> String {
        [
            self.sheet_header(r, "10 tokens distributed as you like on {9}, {10}, and {11}; and 10 tokens in on-planet reserves", 3, 3, "You are native to this planet and know its ways"),
            self.advantage(r, Title::from("Shipment"), "instead of shipping like other factions, you may bring any number of forces from your reserves onto any territory within two territories of and including {12} (subject to storm and occupancy rules).", true),
            self.advantage(r, Title::from("Movement"), "you may move two territories instead of one.", true),
            self.advantage(r, Title::from(Concept::Monster), "if {15} appears in a territory where you have forces, they are not devoured but, immediately upon conclusion of the following nexus phase, may move from that territory to any one territory (subject to storm and occupancy rules).", true),
            self.advantage(r, Title::from("Special victory condition"), "if no player has won by the end of the last turn and if you (or no one) occupies {9} and {22} and neither {1}, {0}, {2} nor {35} occupy {8}, you and your ally have prevented interference with your plans to alter the planet and win the game.", true),
            self.advantage(r, Title::from("Special victory condition"), "if no player has been able to win the game by the end of play, you prevented control over the planet and you and your ally automatically win the game.", !g.is_playing(Faction::Guild) && !g.applicable(Rule::DisableGuildSpecialVictory)),
            self.advanced_header(r, g, &[Rule::FremenDeterminesStorm, Rule::FremenSendingMonster, Rule::FremenStormLosses, Rule::FremenSpecialForces, Rule::AdvancedCombat, Rule::AdvancedKarama]),
            self.advantage_rule(r, g, Rule::FremenDeterminesStorm, Title::from("Storm rule"), "you can see the number of sectors the next storm will move."),
            self.advantage_rule(r, g, Rule::FremenSendingMonster, Title::from("Sandworms"), "during a {16} blow, each time {15} appears after the first time, you choose in which unprotected territory it appears."),
            self.advantage_rule(r, g, Rule::FremenStormLosses, Title::from(MainPhase::Storm), "If caught in a storm, only half your forces are killed. You may rally forces into a storm at half loss."),
            self.advantage_rule(r, g, Rule::FremenSpecialForces, Title::from(FactionSpecialForce::Fremen), "Your 3 {23} are worth two normal forces in battle and in taking losses. They are treated as one token in revival. Only one {23} can be revived per turn."),
            self.advantage_rule(r, g, Rule::AdvancedCombat, Title::from(MainPhase::Battle), "Your forces do not require {16} to count at full strength."),
            self.advantage_rule(r, g, Rule::AdvancedKarama, Title::from(TreacheryCardType::Karma), "during {16} blow, you may use {19} to cause {15} to appear in any unprotected territory that you wish."),
            r#"<h5>Alliance</h5>
                <p>You may protect your allies from being devoured by {15} and may let them revive 3 forces for free.</p>
                <h5>Strategy</h5>
                <p>Your major handicap is poverty. You won't be available to buy cards early game. You must be patient and move your forces into any vacant strongholds, avoiding battles until you are prepared. In battles you can afford to dial high and sacrifice your troops since they have a high revival rate and you can bring them back into play at no cost. You have better mobility than those without a city, and good fighting leaders. Bide your time and wait for an accessible {16} blow that no one else wants in order to build up your resources.</p>
                </div>"#.to_string(),
        ]
        .concat()
    }

    /// `private string GetEmperorTemplate(Game g)`
    fn get_emperor_template(&self, g: &Game, r: &mut SheetReader) -> String {
        [
            self.sheet_header(r, "20 tokens in reserve (off-planet)", 10, 1, "You have access to great wealth"),
            self.advantage(r, Title::from(MainPhase::Bidding), "Payments for treachery cards go to you.", true),
            self.advantage_rule(r, g, Rule::EmperorSupportingNonAllyBids, Title::from("Support"), "you may support bids of non-allied players. Any {16} paid this way flows back to you at the end of the bidding phase."),
            self.advanced_header(r, g, &[Rule::EmperorSpecialForces]),
            self.advantage_rule(r, g, Rule::EmperorSpecialForces, Title::from(FactionSpecialForce::Emperor), "your 5 {24} have a special fighting capability. They are worth two normal tokens in battle and in taking losses against all opponents except {3}. They are treated as one token in revival. Only one {24} can be revived per turn."),
            self.advantage_rule(r, g, Rule::AdvancedKarama, Title::from(TreacheryCardType::Karma), "you may use {19} to revive up to three tokens or one leader for free."),
            r#"<h5>Alliance</h5>
                <p>Unlike other factions, you may give {16} to your allies which is received immediately. Their payment for any treachery card even with your own {16} comes right back to you. In addition, you may pay (directly to the bank) for the revival of up to 3 extra of their forces (for a possible total of 6).</p>
                <h5>Strategy</h5>
                <p>Your major handicap is that you must ship in all of your tokens at the start of the game and often this requires a battle before you are prepared. Even though you do not need to forage for {16} on the surface of the planet often, you still are quite subject to attack since you are likely to concentrate on the cities for the mobility they give you. On the plus side you will never need {16} badly, since the bidding will keep you supplied.</p>
                </div>"#.to_string(),
        ]
        .concat()
    }

    /// `private string GetGuildTemplate(Game g)`
    fn get_guild_template(&self, g: &Game, r: &mut SheetReader) -> String {
        [
            self.sheet_header(r, "5 tokens in {8} and 15 tokens in reserve (off-planet)", 5, 1, "You control all shipments onto the planet"),
            self.advantage(r, Title::from("Payment for shipment"), "Payments by other players for shipments onto the planet from off-planet reserves go to you.", true),
            self.advantage(r, Title::from("Types of shipment"), "You can make one of three possible types of shipments; you may ship normally from reserves onto the planet; or you may site-to-site ship any number of tokens from any one territory to any other territory on the board; or you may ship any number of tokens from any one territory back to your reserves.", true),
            self.advantage(r, Title::from("Half Price"), "You pay only half the fee when shipping. The cost for shipping to your reserves is one {16} for every two tokens shipped.", true),
            self.advantage(r, Title::from("Special victory condition"), "If no player has been able to win the game by the end of play, you prevented control over the planet and you and your ally automatically win the game.", !g.applicable(Rule::DisableGuildSpecialVictory)),
            self.advanced_header(r, g, &[Rule::GuildDetermineShipment]),
            self.advantage_rule(r, g, Rule::GuildDetermineShipment, Title::from("Ship when you wish"), "During Shipment & Move, you decide when you take your turn. You do not have to reveal when you intend to take your turn until the moment you wish to take it."),
            self.advantage_rule(r, g, Rule::AdvancedKarama, Title::from(TreacheryCardType::Karma), "You may use {19} to stop one off-planet shipment of any one player. You may do this directly before or after the shipment occurs."),
            r#"<h5>Alliance</h5>
                <p>Allies may also perform site-to-site shipments and, like you, pay half the fee.</p>
                <h5>Strategy</h5>
                <p>Your major handicap is your weak array of leaders and your inability to revive quickly. In addition, you usually cannot buy treachery cards at the beginning of the game. You are vulnerable at this point and should make your stronger moves after building up your resources. If players do not ship on at a steady rate you will have to fight for {16} on the planet or collect only the isolated blows. Your major advantage is that you can ship onto the planet inexpensively and can ship from any one territory to any other. This mobility allows you to make surprise moves and is particularly useful when you are the last player in the movement round. If the game is out of reach and well along, try suicide battles against the strongest players to weaken them and prevent a win until the last turn: the victory is then yours.</p>
                </div>"#.to_string(),
        ]
        .concat()
    }

    /// `private string GetBeneGesseritTemplate(Game g)`
    fn get_bene_gesserit_template(&self, g: &Game, r: &mut SheetReader) -> String {
        [
            self.sheet_header(r, "1 token in {26} and 19 tokens in reserve (off-planet)", 5, 1, "You are adept in the ways of mind control"),
            self.advantage(r, Title::from("Prediction"), "At start of game you secretly predict which faction will win in which turn (you can't predict the special victory condition of {4} or {3} at the end of play). If that factions wins (alone or as an ally, even your own) when you have predicted, you alone win instead. You can also win normally.", true),
            self.advantage(r, Title::from("Shipment"), "Whenever any other player ships, you may ship for free one force to {26}.", true),
            self.advantage(r, Title::from("Voice"), "You may Voice your opponent in battle to play or not to play a projectile/poison weapon or defense, a worthless card or a <i>specific</i> special card. If he can't comply with your command, he may do as he wishes.", true),
            self.advanced_header(r, g, &[Rule::BeneGesseritFirstForceInAnyTerritory, Rule::BeneGesseritAutoCharity, Rule::BeneGesseritAdvisors, Rule::BeneGesseritWorthlessAsKarma]),
            self.advantage_rule(r, g, Rule::BeneGesseritAutoCharity, Title::from(MainPhase::Charity), "You automatically receive 2 charity during the {47} Phase."),
            self.advantage_rule(r, g, Rule::BeneGesseritAdvisors, Title::from("Advisors"), "You can peacefully coexist as <i>spiritual advisors</i> with opponent forces in the same territory. Advisors cannot collect {16}, cannot be involved in combat, don't prevent opponent control of a stronghold and don't yield three territory movement bonus. They are still susceptible to storms, {15} and {18}/{17} explosions. You can never have both advisors and fighters in the same territory."),
            self.advantage_rule(r, g, Rule::BeneGesseritAdvisors, Title::from("At Start"), "Instead of starting with 1 force in {26} you may start with an advisor in a territory of choice."),
            self.advantage_rule(r, g, Rule::BeneGesseritAdvisors, Title::from("Advisor Shipment"), "You can ship advisors to a territory where you have advisors and whenever any other player ships, instead of shipping a force to {26}, you may ship an advisor to the same territory."),
            self.advantage_rule(r, g, Rule::BeneGesseritAdvisors, Title::from("End peaceful coexistence"), "You may announce before the {21} phase all territories in which you no longer wish to peacefully coexist. Advisors there are flipped to fighters."),
            self.advantage_rule(r, g, Rule::BeneGesseritAdvisors, Title::from("Flipping"), "When you move forces into an occupied territory where you don't have forces or when another player moves tokens into a territory where you have advisors, you decide to stay there as either advisors or fighters."),
            self.advantage(r, Title::from("Not with ally"), "Allied forces may not willingly end up in a territory where you have advisors (and vice versa).", g.applicable(Rule::BeneGesseritAdvisors) && !g.applicable(Rule::AdvisorsDontConflictWithAlly)),
            self.advantage_rule(r, g, Rule::BeneGesseritWorthlessAsKarma, Title::from(TreacheryCardType::Karma), "You may use a worthless card as a {19} card."),
            r#"<h5>Alliance Power</h5>
                <p>You may Voice your ally's opponent.</p>
                <h5>Strategy</h5>
                <p>Your major handicap is your low revival rate. Don't allow large numbers of your tokens to be killed or you may find yourself without sufficient reserves to bring onto the planet. Your strengths are that you have the ability to win by correctly predicting another winner and the secretly working for that player. In addition, you can be quite effective in battles by voicing your opponent and leaving him weaponless or defenseless. You can afford to bide your time while casting subtle innuendoes about which player you have picked to win.</p>
                </div>"#.to_string(),
        ]
        .concat()
    }

    /// `private string GetTleilaxuTemplate(Game g)`
    fn get_tleilaxu_template(&self, g: &Game, r: &mut SheetReader) -> String {
        [
            self.sheet_header(r, "20 tokens in reserve (off-planet)", 5, 2, "You have superior genetic engineering technology"),
            self.advantage(r, Title::from("Face dancers"), r#"At the start of the game you are not dealt Traitor Cards. After traitors are selected, unused traitor cards are shuffled and you get the top 3 cards. These are your Face Dancers. After a faction has won a battle you may reveal the leader they used to be a Face Dancer, and the following occurs:
                  <ol>
                  <li>The battle is fully resolved a normal.</li>
                  <li>The Face Dancer leader is sent to the {31} if it was not already killed. You don't collect {16} for it.</li>
                  <li>The remaining amount of forces in the territory go back to their reserves and are replaced by up to the same amount of your forces from your reserves and/or from anywhere on the planet.</li>
                  </ol>
                  <p>Once revealed you do not replace a Face Dancer (Traitor Card) until you have revealed all 3. When that happens, they are shuffled into the Traitor deck and you get 3 new Face Dancers.
                  During {43}, you may replace an unrevealed Face Dancer with one from the traitor deck (after shuffling it back).</p>"#, true),
            self.advantage(r, Title::from("Unlimited Revival"), "You have no revival limits and your revive at half price (rounded up). Payments by other players for revivals go to you. Each time a player uses free revival or a Tleilaxu Ghola card, you get 1 {16} from the bank.", true),
            self.advantage(r, Title::from("Increased Revival"), "You may increase the 3 force revival limit for any other faction to 5.", true),
            self.advantage(r, Title::from("Leader Revival"), "Upon request by a faction for a specific killed leader, you can set a price for its early revival. This can only be done when this leader cannot be revived according to normal revival rules.", true),
            self.advantage(r, Title::from("Zoal"), "{44}’s value in battle matches the value of the opponent’s leader (0 for a Cheap Hero), and for collecting {16} when killed. {44} costs 3 to revive.", true),
            self.advanced_header(r, g, &[Rule::TleilaxuGholas, Rule::AdvancedKarama]),
            self.advantage(r, Title::from("Gholas"), "When you have fewer than five leaders available, you may revive dead leaders of other factions at your discounted rate and add them to your leader pool.", true),
            self.advantage_rule(r, g, Rule::AdvancedKarama, Title::from(TreacheryCardType::Karma), "you may prevent a player from performing a revival (forces and/or leader)."),
            r#"<h5>Alliance Power</h5>
                <p>Your ally may revive at half price (rounded up).</p>
                <h5>Strategy</h5>
                <p>You are handicapped by having no forces on the planet to start and only a small amount of {16} until you begin receiving {16} for revivals. You will have to bide your time as other factions battle, waiting until you start gaining {16} and giving your Face Dancers a chance to suddenly strike, or get into minor battles early to drive forces to the tanks, and possibly get a Face Dancer reveal. Use your ability to cycle through Face Dancers during the Mentat Pause to position yourself with a potentially more useful Face Dancer.</p>
                </div>"#.to_string(),
        ]
        .concat()
    }

    /// `private string GetIxianTemplate(Game g)`
    fn get_ixian_template(&self, g: &Game, r: &mut SheetReader) -> String {
        [
            self.sheet_header(r, "3 {32} and 3 {33} in the {34}. 10 {32} and 4 {33} in reserve (off-planet)", 10, 1, "You are skilled in technology and production"),
            self.advantage(r, Title::from("Start of game"), "During Setup you see all initially dealt Treachery Cards and choose your starting card from them.", true),
            self.advantage(r, Title::from("Bidding"), "Before Bidding, one extra card is drawn and you see them all and put one of those cards on top or on the bottom of the Treachery Card deck. The remaining cards are shuffled for the bidding round.", true),
            self.advantage(r, Title::from(FactionSpecialForce::Ixian), "Your 7 {33} are each worth 2 normal forces in battle, are able to move 2 territories instead of 1 and can collect 3 {16}. Your {33} forces ship normally, but each costs 3 to revive.", true),
            self.advantage(r, Title::from(FactionForce::Ixian), "Your 13 {32} ship normally but are worth ½ in battle. {32} can be used to absorb losses after a battle. After battle losses are calculated, any of your surviving {32} in that territory can be exchanged for {33} you lost in that battle. {32} can control strongholds and collect {16}. {32} move 2 if accompanied by a {33}, or 1 if they are not.", true),
            self.advantage(r, Title::from("Hidden Mobile Stronghold"), "After the first storm movement, place your {34} in any non-stronghold territory. This stronghold counts towards the game win and is protected from worms and storms. Subsequently, before storms are revealed and as long as your forces occupy it, you may move your {34} up to 3 territories to any non-stronghold territory. You can't move it into or out of a storm. When you move into, from, or through a sector containing {16}, you may immediately collect 2 {16} per force in your stronghold. No other faction may ship forces directly into your {34}, or move it if they take control. Other factions must move or ship forces into the territory it is pointing at (including {26}), and then use one movement to enter.", true),
            self.advanced_header(r, g, &[Rule::IxianSwappingCardOnBid, Rule::AdvancedCombat, Rule::AdvancedKarama]),
            self.advantage_rule(r, g, Rule::IxianSwappingCardOnBid, Title::from("Tech"), "Once, during the Bidding phase, before {0} gets to look and bidding begins, you may take the card about to be bid on and replace it with a card from your hand."),
            self.advantage_rule(r, g, Rule::AdvancedCombat, Title::from("{32} strength"), "{32} are worth ½ in battle. You can’t increase the effectiveness of {32} in battle by spending {16}."),
            self.advantage_rule(r, g, Rule::AdvancedKarama, Title::from(TreacheryCardType::Karma), "During Shipment and Movement, you may move the {34} 2 territories on your turn as well as make your normal movement."),
            r#"<h5>Alliance Power</h5>
                <p>After your ally purchases a Treachery Card during bidding, they may immediately discard it and draw the top card from the deck.</p>
                <h5>Strategy</h5>
                <p>You are handicapped by having weaker forces in the half strength {32}, which make up the bulk of your forces. You have no regular source of {16} income. However, tactical placement of your {34} can position you to acquire {16} left behind on the planet. You also have an advantage over other factions because you know what Treachery cards are in play and you can mix in or suppress certain cards during the bidding phase.</p>
                </div>"#.to_string(),
        ]
        .concat()
    }

    /// `private string GetRicheseTemplate(Game g)`
    fn get_richese_template(&self, g: &Game, r: &mut SheetReader) -> String {
        [
            self.sheet_header(r, "20 tokens in reserve (off-planet). Start with 5 {16}. You have a separate deck of 10 {35} cards that are not part of your hand", 5, 2, "You have alternative technology"),
            self.advantage(r, Title::from(MainPhase::Bidding), r#"When drawing Treachery cards up for bid, one less card is drawn and instead you choose to auction a card from your {35} cards first or last. When it is time to auction that card, you choose and reveal the card and the type of auction (Once Around or Silent). Payments for your cards by other players go to you. If you buy any, the {16} goes to the {2} or the {16} Bank normally. Whenever discarded, these cards go to the normal discard pile. They can’t be bought with {19}.
              <ul>
              <li><i>Once Around:</i> pick clockwise or counter-clockwise. Starting with the faction on one side of you, each player able to bid may pass or bid higher. You bid last and the highest bidder gets the card. If everyone else passes, you may either get the card for free or remove it from the game.</li>
              <li><i>Silent:</i> all factions able to bid secretly choose an amount to bid. Choices are revealed simultaneously. The faction that bid to most wins the card (ties break according to Storm order). If all factions bid zero {16}, you may either get the card for free or remove it from the game.</li>
              </ul>"#, true),
            self.advantage(r, Title::from(MainPhase::ShipmentAndMove), r#"Instead of normal shipping, you may ship one of your No-Field tokens (0, 3 or 5) face-down, paying for one force. Other factions do not know how many of your forces are located there and proceed as if at least one is there. You may reveal a No-Field token at any time before the Battle phase, placing the indicated number of forces from your reserves (or up to that amount if you have fewer forces left). You may move a No-Field token like a force.<br>
              You may not have two No-Field tokens on the planet at the same time. The last revealed token stays face-up in front of your shield until another one is revealed.<br>
              When you are in a battle, you must reveal the value of a No-Field token in that territory and place the indicated number of forces from your reserves (or up to that amount if you have fewer forces left). When you are in a Battle with a No-Field token, {0} may not see your force dial."#, true),
            self.advanced_header(r, g, &[Rule::RicheseBlackMarket, Rule::AdvancedCombat]),
            self.advantage_rule(r, g, Rule::RicheseBlackMarket, Title::from("Black Market"), "At the start of the Bidding Round, you may auction one card from your hand. You may tell what you are selling (you may lie) and keep the card face-down ({0} may still look). You may use any type of auction. If no player bids, you keep the card. If it is sold, one fewer card is put up for auction as part of the normal Bidding Round. Payments by other players go to you. Black market cards cannot be bought with {19}. If the normal bidding type was used, the regular bidding round resumes wherever it left off."),
            self.advantage_rule(r, g, Rule::AdvancedKarama, Title::from(TreacheryCardType::Karma), "You may buy a card from your separate cache for 3 {16} at any time."),
            r#"<h5>Alliance</h5>
                <p>Your ally may ship using one of your available No-Field tokens, revealing it immediately upon shipping. Place the used No-Field token face-up in front of your shield until you reveal another No-Field token.</p>
                <p>You may give your ally a {35} card from your hand at any time.</p>
                <h5>Strategy</h5>
                <p>You are at a disadvantage by having no forces on the planet, and not much spice to operate. Try to be aware if factions would be inclined to buy one of your special Cards either for their use or to keep it out of the hands of another faction. Selling your cards will be your one regular form of income until you have gained enough spice. Use your No-Field tokens to get forces on the planet cheaply and confuse your opponents.</p>
                </div>"#.to_string(),
        ]
        .concat()
    }

    /// `private string GetChoamTemplate(Game g)`
    fn get_choam_template(&self, g: &Game, r: &mut SheetReader) -> String {
        [
            self.sheet_header(r, "20 tokens in reserve (off-planet)", 2, 0, "You control economic affairs across the Imperium"),
            self.advantage(r, Title::from(MainPhase::Charity), "Each turn, you collect 2 {16} for each faction in the game during Charity before any factions collect. If another faction collects Charity, it is paid to them from your {16}.", true),
            self.advantage(r, Title::from(MainPhase::Resurrection), "You have no limit to the number of forces you may pay to revive and it only costs you 1 for each force.", true),
            self.advantage(r, Title::from("Treachery"), r#"You may hold up to 5 Treachery Cards. At the end of any phase, you may reveal duplicates of the same card from your hand for 3 {16} each. You may also discard {14} cards for 2 {16} each. Alternatively, you may use {14} cards as follows:
                <ul>
                <li><i>{37}</i>: Prevent a player from moving forces into a territory you occupy during {21}. They may ship in normally.</li>
                <li><i>{38}</i>: Prevent a loss of your forces in one territory to the Storm when it moves.</li>
                <li><i>{39}</i>: Prevent a player from playing a {19} card this phase as they attempt to do so.</li>
                <li><i>{40}</i>: Move your forces one extra territory on your turn during {21}.</li>
                <li><i>{41}</i>: Prevent a player from taking Free Revival.</li>
                <li><i>{42}</i>: Force a player to send 1 force back to reserves during {43}.</li>
                </ul>"#, true),
            self.advantage(r, Title::from("Inflation"), "You may play your Inflation token during {43} with either the Double or Cancel side face-up. In the following game turn, Charity is either doubled or canceled for that turn (including {5} in the advanced game). While Charity is doubled, no bribes can be made. In the next Mentat the Inflation token is flipped to the other side. If the token has already been flipped it is removed from the game instead.", true),
            self.advanced_header(r, g, &[Rule::ChoamAuditor, Rule::AdvancedCombat, Rule::AdvancedKarama]),
            self.advantage_rule(r, g, Rule::AdvancedCombat, Title::from("Forces"), "When other players pay for their forces in battle, half of the {16} (rounded down) goes to you."),
            self.advantage_rule(r, g, Rule::ChoamAuditor, Title::from("Auditor"), "You gain the Auditor Leader and it is added to the Traitor deck at the start of the game. Whenever you use the Auditor as a leader in a battle, you may audit your opponent. You may look at two cards in your opponent's hand at random (not counting cards used in battle) if the Auditor survived, or one card if killed. That faction may pay you 1 {16} per card you would get to see to cancel the audit. The Auditor may be revived as if all of your leaders were in the Tanks. The Auditor can't be revived as a ghola, nor be captured by {1}. The Auditor can't have a Leader Skill."),
            self.advantage_rule(r, g, Rule::AdvancedKarama, Title::from(TreacheryCardType::Karma), "You may discard any Treachery cards from your hand and gain 3 {16} each."),
            r#"<h5>Alliance</h5>
                <p>Once per turn at the end of a phase, you may trade a Treachery Card with your ally. This trade is simultaneous and two-way.</p>
                <p>You may pay for your ally’s forces in battle.</p>
                <h5>Strategy</h5>
                <p>Your leaders are weak, but you have a steady income. Stockpile Treachery Cards. You start with no forces on the planet and must ship them all in. For this reason, you may want to wait until you can attack with a large force. Use your Inflation token at a key moment, especially at a time when others aren’t collecting Charity.</p>
                </div>"#.to_string(),
        ]
        .concat()
    }

    /// `private string GetEcazTemplate(Game g)`
    fn get_ecaz_template(&self, g: &Game, r: &mut SheetReader) -> String {
        [
            self.sheet_header(r, "6 tokens in {50} and 14 in reserve (off-planet)", 12, 2, "You forge strong alliances"),
            self.advantage(r, Title::from("Ambassadors"), "At start you get the {48} Ambassador token and 5 random Ambassadors for other factions. At the end of Revival you may place Ambassadors in any stronghold not in storm that does not have one for 1 spice (cost increases by 1 spice for each subsequent Ambassador that turn). When another faction (other than your ally or the faction matching the marker) enters a stronghold with an Ambassador, you may trigger its effect, setting it aside. After triggering all non-{48} Ambassadors, you get 5 random new ones. Ambassadors are vulnerable to game effects like storm or explosions, returning to your supply.", true),
            self.advantage(r, Title::from("Occupy"), "When you are in an alliance, you and your ally’s forces are considered the same faction, and may enter and occupy the same territory. If you both collect spice from a desert territory, you split the it however you both agree (as evenly as possible if you can’t agree). If both in a battle, you decide which of you is considered the faction that fights. Regardless of who fights, your ally's forces are the ones dialed, and half of your forces in the territory (rounded up) are added to the number dialed. After the battle, half of your forces (rounded down) remain in the territory if your side wins, and the rest go to the Tanks. If you and your ally both occupy a stronghold at the end of a turn, it only counts as one stronghold for both of you, and you control it. It only takes three strongholds for you and your ally to win, if you both are co-occupied in all three.", true),
            self.advantage(r, Title::from(MainPhase::Resurrection), "You may always revive Duke Vidal for 5 spice, no matter how many of your leaders are in the Tanks.  You may revive leaders normally when at least 5 are in the Tanks (counting Duke Vidal).", true),
            self.advanced_header(r, g, &[Rule::EcazLoyalty, Rule::IncreasedResourceFlow, Rule::AdvancedKarama]),
            self.advantage_rule(r, g, Rule::EcazLoyalty, Title::from("Loyalty"), "At the start of the game, one of your leader (selected randomly) is removed from the Traitor deck."),
            self.advantage_rule(r, g, Rule::IncreasedResourceFlow, Title::from(MainPhase::Collection), "You and your ally both gain income from strongholds that yield {16} during collection."),
            self.advantage_rule(r, g, Rule::AdvancedKarama, Title::from(TreacheryCardType::Karma), "If you played neither a weapon nor a defense, you may add the difference between your leader disc and your opponent’s leader disc to your number dialed."),
            r#"<h5>Alliance</h5>
                <p>You may have your ally benefit from a triggered Ambassador's effect.</p>
                <h5>Strategy</h5>
                <p>Much of your strength comes from being in an alliance as soon as possible.  Your Occupy advantage can provide a significant boost when you send several forces into a stronghold your ally occupies.  Your Ambassadors can provide you with useful perks, and placing them requires careful consideration, not only with the order you select them, but in how their presence on the map can influence the other factions.</p>
                </div>"#.to_string(),
        ]
        .concat()
    }

    /// `private string GetMoritaniTemplate(Game g)`
    fn get_moritani_template(&self, g: &Game, r: &mut SheetReader) -> String {
        [
            self.sheet_header(r, "6 tokens in any unoccupied territory when all other factions set up and 14 in reserve (off-planet)", 12, 2, "You resort to terrorism"),
            self.advantage(r, Title::from("Terrorize"), "During the Mentat Pause, you may place a Terror token face down in any stronghold that doesn't have one (other than the Ixian Hidden Mobile Stronghold), even one under storm, or else move one to a stronghold that doesn't have one.  You may reveal and activate a Terror token when another faction (other than your ally) enters a stronghold containing one (either moving or shipping in, including advisors), applying effects to that faction. Revealed tokens are removed from the game.", true),
            self.advantage(r, Title::from("Duke Vidal"), "You gain Duke Vidal at the end of Shipping and Movement if you are in at least two battles in strongholds (not counting battles involving Ecaz) if he is not captured, a ghola, or in the Tanks, taking him from any faction currently controlling him.  Set him aside at the end of the turn if he’s not in the Tanks or captured.", true),
            self.advantage(r, Title::from("Enemy of my Enemy"), "When a faction (other than Ecaz) would trigger a Terror token, you may offer to enter into an alliance with that faction before the token is revealed.  If that faction accepts, you both are now allied (breaking existing alliances either or both of you were in).  Your Terror token is not revealed, and returns to your supply.  If that faction does not accept, the Terror token must be revealed.", true),
            self.advanced_header(r, g, &[Rule::MoritaniAssassinate, Rule::AdvancedKarama]),
            self.advantage_rule(r, g, Rule::MoritaniAssassinate, Title::from("Assassinate Leaders"), "When you lose a battle in which the opposing player had a leader disc that was not killed (and no Traitor was called), you may reveal a Traitor Card for the same faction (other than the leader you opposed).  If they are not in the Tanks, kill that leader and collect spice for them.  During the Mentat Pause, set the revealed card aside face up as a marker, then draw a new Traitor Card.  You may only use this advantage once against each other faction in the game. You may reveal a Traitor Card normally, but then this advantage is lost."),
            self.advantage_rule(r, g, Rule::AdvancedKarama, Title::from(TreacheryCardType::Karma), "If you lose a battle, force your opponent to discard or keep any or all Treachery Cards they played."),
            r#"<h5>Alliance</h5>
                <p>When your ally loses a battle that had a winner, they may keep one Treachery Card they played in the battle that they would have been able to keep had they won.</p>
                <h5>Strategy</h5>
                <p>You are handicapped by having to wait a few turns before you can get multiple Terror tokens onto the board, and you can only gain Duke Vidal by getting into battles in strongholds.  It’s to your advantage to prolong the game until you can maneuver into a situation where you can either utilize Enemy of My Enemy to gain a useful ally at a critical moment, or the other factions have been weakened enough by your Terror tokens that you can go for the win alone.</p>
                </div>"#.to_string(),
        ]
        .concat()
    }

}

/// `private void NoteAdvantage(string name)`
fn note_advantage(r: &mut SheetReader, name: &str) {
    let in_advanced = r.in_advanced;
    let Some(sheet) = r.sheet.as_mut() else { return };
    if in_advanced { &mut sheet.advanced_advantages } else { &mut sheet.advantages }.push(name.to_string());
}
