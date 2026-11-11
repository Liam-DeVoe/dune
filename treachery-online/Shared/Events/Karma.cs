/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */


namespace Treachery.Shared;

public class Karma : GameEvent
{
    #region Construction

    public Karma(Game game, Faction initiator) : base(game, initiator)
    {
    }

    public Karma()
    {
    }

    #endregion Construction

    #region Properties

    public FactionAdvantage Prevented { get; set; }

    public int _cardId;

    [JsonIgnore]
    public TreacheryCard Card
    {
        get => TreacheryCardManager.Get(_cardId);
        set => _cardId = TreacheryCardManager.GetId(value);
    }

    #endregion Properties

    #region Validation

    public override Message? Validate()
    {
        if (!ValidFactionAdvantages(Game, Player).Contains(Prevented)) return Message.Express("You cannot prevent ", Prevented, " at this moment");

        return null;
    }

    public static IEnumerable<FactionAdvantage> ValidFactionAdvantages(Game g, Player p)
    {
        var result = new List<FactionAdvantage>
        {
            FactionAdvantage.None
        };

        if (p.Faction != Faction.Atreides && g.IsPlaying(Faction.Atreides))
        {

            if (In(g, MainPhase.Bidding)) result.Add(FactionAdvantage.AtreidesBiddingPrescience);
            if (In(g, MainPhase.Resurrection) || In(g, MainPhase.ShipmentAndMove)) result.Add(FactionAdvantage.AtreidesSpiceBlowPrescience);

            if (In(g, MainPhase.Battle))
            {
                if (g.Applicable(Rule.AtreidesMessiah)) result.Add(FactionAdvantage.AtreidesUseMessiah);
                result.Add(FactionAdvantage.AtreidesBattlePlanPrescience);
            }
        }

        if (p.Faction != Faction.Harkonnen && g.IsPlaying(Faction.Harkonnen))
        {
            if (In(g, MainPhase.Bidding)) result.Add(FactionAdvantage.HarkonnenFreeCard);

            if (In(g, MainPhase.Battle))
            {
                if (g.Applicable(Rule.HarkonnenCapturesOrKillsLeaders)) result.Add(FactionAdvantage.HarkonnenCaptureLeader);

                result.Add(FactionAdvantage.HarkonnenCallTraitorForAlly);
            }
        }

        if (p.Faction != Faction.Fremen && g.IsPlaying(Faction.Fremen))
        {
            if (In(g, MainPhase.Storm))
            {
                if (g.Applicable(Rule.FremenDeterminesStorm)) result.Add(FactionAdvantage.FremenStormPrescience);
                if (g.Applicable(Rule.FremenStormLosses)) result.Add(FactionAdvantage.FremenProtectedFromStorm);
            }

            if (In(g, MainPhase.Blow))
            {
                result.Add(FactionAdvantage.FremenControlsMonster);
                result.Add(FactionAdvantage.FremenProtectedFromMonster);
            }

            if (In(g, MainPhase.ShipmentAndMove)) result.Add(FactionAdvantage.FremenExtraMove);

            if (In(g, MainPhase.Battle))
            {
                if (g.Applicable(Rule.FremenSpecialForces)) result.Add(FactionAdvantage.FremenSpecialForceBonus);
                if (g.Applicable(Rule.AdvancedCombat)) result.Add(FactionAdvantage.FremenNotPayingForBattles);
            }
        }

        if (p.Faction != Faction.Emperor && g.IsPlaying(Faction.Emperor))
        {
            result.Add(FactionAdvantage.EmperorGiveSpiceToAlly);

            if (In(g, MainPhase.Bidding)) result.Add(FactionAdvantage.EmperorReceiveBid);

            if (In(g, MainPhase.Resurrection)) result.Add(FactionAdvantage.EmperorLetAllyReviveExtraForces);

            if (In(g, MainPhase.Battle) && g.Applicable(Rule.EmperorSpecialForces)) result.Add(FactionAdvantage.EmperorSpecialForceBonus);
        }

        if (p.Faction != Faction.Guild && g.IsPlaying(Faction.Guild))
            if (In(g, MainPhase.ShipmentAndMove))
            {
                var guildDetermineShipmentMayStillBePrevented = g.Version < 156 || (!g.HasActedOrPassed.Contains(Faction.Guild) && g.CurrentPhase != Phase.GuildShip);
                if (g.Applicable(Rule.GuildDetermineShipment) && guildDetermineShipmentMayStillBePrevented) result.Add(FactionAdvantage.GuildDetermineMoveMoment);
                result.Add(FactionAdvantage.GuildSpecialShipments);
                result.Add(FactionAdvantage.GuildShipmentsDiscount);
                result.Add(FactionAdvantage.GuildShipmentsDiscountAlly);
                result.Add(FactionAdvantage.GuildReceiveShipment);
            }

        if (p.Faction != Faction.BeneGesserit && g.IsPlaying(Faction.BeneGesserit))
        {
            if (In(g, MainPhase.Charity))
                if (g.Applicable(Rule.BeneGesseritAutoCharity)) result.Add(FactionAdvantage.BeneGesseritCharity);

            if (g.Applicable(Rule.BeneGesseritWorthlessAsKarma)) result.Add(FactionAdvantage.BeneGesseritWorthlessAsKarma);
            if (g.Applicable(Rule.BeneGesseritAdvisors)) result.Add(FactionAdvantage.BeneGesseritAnnouncesBattle);

            if (In(g, MainPhase.ShipmentAndMove))
            {
                if (g.Applicable(Rule.BeneGesseritAdvisors)) result.Add(FactionAdvantage.BeneGesseritIntrusion);
                result.Add(FactionAdvantage.BeneGesseritAccompanies);
            }

            if (In(g, MainPhase.Battle)) result.Add(FactionAdvantage.BeneGesseritUsingVoice);
        }

        if (p.Faction != Faction.Ixian && g.IsPlaying(Faction.Ixian))
        {
            if (In(g, MainPhase.Storm)) result.Add(FactionAdvantage.IxianMovingHms);

            if (In(g, MainPhase.Bidding))
            {
                result.Add(FactionAdvantage.IxianSelectingCardsOnAuction);
                result.Add(FactionAdvantage.IxianAllyDiscardingCard);
                if (g.Applicable(Rule.IxianSwappingCardOnBid)) result.Add(FactionAdvantage.IxianSwappingCard);
            }

            if (In(g, MainPhase.ShipmentAndMove)) result.Add(FactionAdvantage.IxianCyborgExtraMove);

            if (In(g, MainPhase.Battle))
            {
                result.Add(FactionAdvantage.IxianSpecialForceBonus);
                result.Add(FactionAdvantage.IxianReplacingSpecialForces);
            }
        }

        if (p.Faction != Faction.Tleilaxu && g.IsPlaying(Faction.Tleilaxu))
        {
            if (In(g, MainPhase.Bidding) || In(g, MainPhase.Resurrection)) result.Add(FactionAdvantage.TleilaxuIncreasingRevivalLimits);

            if (In(g, MainPhase.Resurrection))
            {
                result.Add(FactionAdvantage.TleilaxuRevivalDiscount);
                result.Add(FactionAdvantage.TleilaxuRevivalDiscountAlly);
                result.Add(FactionAdvantage.TleilaxuReceiveRevive);
                result.Add(FactionAdvantage.TleilaxuEarlyLeaderRevive);
                if (g.Applicable(Rule.TleilaxuGholas)) result.Add(FactionAdvantage.TleilaxuReviveGhola);
            }

            if (In(g, MainPhase.Contemplate)) result.Add(FactionAdvantage.TleilaxuReplacingFaceDancer);
        }

        if (p.Faction != Faction.Choam && g.IsPlaying(Faction.Choam))
        {
            if (In(g, MainPhase.Charity)) result.Add(FactionAdvantage.ChoamControllingCharity);

            result.Add(FactionAdvantage.ChoamDiscarding);

            if (In(g, MainPhase.Resurrection)) result.Add(FactionAdvantage.ChoamRevival);

            if (In(g, MainPhase.Battle))
            {
                result.Add(FactionAdvantage.ChoamReceiveForcePayment);
                if (g.Applicable(Rule.ChoamAuditor)) result.Add(FactionAdvantage.ChoamAudit);
            }

            if (In(g, MainPhase.Contemplate)) result.Add(FactionAdvantage.ChoamEconomics);
        }

        if (p.Faction != Faction.Richese && g.IsPlaying(Faction.Richese))
        {
            if (In(g, MainPhase.Bidding))
            {
                result.Add(FactionAdvantage.RicheseAuction);
                if (g.Applicable(Rule.RicheseBlackMarket)) result.Add(FactionAdvantage.RicheseBlackMarket);
            }

            if (In(g, MainPhase.ShipmentAndMove)) result.Add(FactionAdvantage.RicheseNofield);
        }

        if (p.Faction != Faction.Ecaz && g.IsPlaying(Faction.Ecaz))
        {
            if (In(g, MainPhase.Resurrection)) result.Add(FactionAdvantage.EcazAmbassadors);
            if (In(g, MainPhase.Battle)) result.Add(FactionAdvantage.EcazOccupation);
            if (In(g, MainPhase.Collection)) result.Add(FactionAdvantage.EcazCollection);
        }

        if (p.Faction != Faction.Moritani && g.IsPlaying(Faction.Moritani))
        {
            result.Add(FactionAdvantage.MoritaniEnemyOfEnemy);
            if (In(g, MainPhase.ShipmentAndMove)) result.Add(FactionAdvantage.MoritaniGainingVidal);
            if (In(g, MainPhase.Contemplate)) result.Add(FactionAdvantage.MoritaniPlantingTerror);
        }

        return result;
    }

    private static bool In(Game g, MainPhase phase)
    {
        return g.Version < 150 || g.CurrentMainPhase == phase;
    }

    public static IEnumerable<TreacheryCard> ValidKarmaCards(Game g, Player p)
    {
        return p.TreacheryCards.Where(c =>
            c.Type == TreacheryCardType.Karma ||
            (c.Type == TreacheryCardType.Useless && p.Is(Faction.BeneGesserit) && g.Applicable(Rule.BeneGesseritWorthlessAsKarma)) ||
            (c.Type == TreacheryCardType.Clairvoyance && p.Occupies(g.Map.Shrine)));
    }

    public static bool CanKarma(Game g, Player p)
    {
        return ValidKarmaCards(g, p).Any();
    }

    #endregion Validation

    #region Execution

    protected override void ExecuteConcreteEvent()
    {
        Game.Discard(Card, false);
        Game.Log(Game.KarmaPlayed(Initiator, Card, Prevented is FactionAdvantage.None ? [] : ["prevent ", Prevented]));
        Game.Stone(Milestone.Karma);

        if (Prevented is not FactionAdvantage.None) Game.Prevent(Initiator, Prevented, false);

        RevokeBattlePlansIfNeeded();

        if (Prevented is FactionAdvantage.BeneGesseritUsingVoice) Game.CurrentVoice = null;

        if (Prevented is FactionAdvantage.AtreidesBattlePlanPrescience) Game.CurrentPrescience = null;

        if (Player.Is(Faction.Ixian) &&
            Game.CurrentPhase == Phase.IxianSwappingCard &&
            Player.TreacheryCards.Count == 0)
            Game.ContinueAfterIxianCardSwapDecision();
        
        if (Prevented is FactionAdvantage.FremenRidesMonster && Game.CurrentPhase is Phase.FremenRidingMonsterA or Phase.FremenRidingMonsterB)
        {
            Game.DetermineNextShipmentAndMoveSubPhase();
        }
    }

    private void RevokeBattlePlansIfNeeded()
    {
        if (Game.CurrentMainPhase == MainPhase.Battle)
        {
            if (Prevented == FactionAdvantage.FremenNotPayingForBattles ||
                Prevented == FactionAdvantage.FremenSpecialForceBonus)
                Game.RevokePlanIfNeeded(Faction.Fremen);

            if (Prevented == FactionAdvantage.EmperorSpecialForceBonus) Game.RevokePlanIfNeeded(Faction.Emperor);

            if (Prevented == FactionAdvantage.IxianSpecialForceBonus) Game.RevokePlanIfNeeded(Faction.Ixian);

            if (Prevented == FactionAdvantage.AtreidesUseMessiah) Game.RevokePlanIfNeeded(Faction.Atreides);
        }
    }

    public override Message GetMessage()
    {
        return Message.Express(
            Initiator,
            " play ",
            MessagePart.ExpressIf(Card.Type != TreacheryCardType.Karma, Card, " as "),
            " a ",
            TreacheryCardType.Karma,
            " card");
    }

    #endregion Execution


}