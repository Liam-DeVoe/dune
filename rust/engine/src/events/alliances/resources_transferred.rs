use crate::prelude::*;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ResourcesTransferred {
    pub resources: u16,
}

impl ResourcesTransferred {
    pub fn can_be_played(g: &Game, p: PlayerId) -> bool {
        g.player(p).has_ally() && ResourcesTransferred::max_amount(g, p) > 0 && ResourcesTransferred::may_transfer(g, p)
    }

    pub fn may_transfer(g: &Game, p: PlayerId) -> bool {
        !(g.has_bid_to_pay(p) || g.current_phase_cannot_be_interrupted())
    }

    pub fn max_amount(game: &Game, p: PlayerId) -> i32 {
        let pl = game.player(p);
        i32::from(pl.transferable_resources).min(i32::from(pl.resources))
    }
}

impl EventRules for ResourcesTransferred {
    fn validate(&self, game: &Game, by: By) -> Result<(), Rejection> {
        if i32::from(self.resources) > ResourcesTransferred::max_amount(game, by.player()) {
            return Err(Rejection(express!("You can't transfer more than ", Payment::of(ResourcesTransferred::max_amount(game, by.player())))));
        }
        if !ResourcesTransferred::may_transfer(game, by.player()) {
            return Err(Rejection(express!("You currently can't transfer ", Concept::Resource)));
        }
        // Spice still owed for a bid can't be given away, or paying the bid would take the
        // player below zero (escapes.md, "Forces below zero").
        if !game.may_give_away_spice(by.player(), i32::from(self.resources)) {
            return Err(Rejection(express!("You need that spice to pay your bid")));
        }

        Ok(())
    }

    fn apply(&self, game: &mut Game, by: By) -> GameResult {
        let m = self.describe(game, by);
        game.log_event(m);
        let me = by.player();
        {
            let p = game.player_mut(me);
            crate::game::amounts::spend(&mut p.resources, i32::from(self.resources), "spice")?;
            crate::game::amounts::spend(&mut p.transferable_resources, i32::from(self.resources), "transferable spice")?;
        }
        let ally = game.player(me).allied_player(game).required("allied player")?;
        let a = game.player_mut(ally);
        crate::game::amounts::gain(&mut a.resources, i32::from(self.resources), "spice")?;
        Ok(())
    }

    fn describe(&self, _game: &Game, by: By) -> Message {
        express!(by.faction, " transfer ", Payment::of(i32::from(self.resources)), " to their ally")
    }
}
