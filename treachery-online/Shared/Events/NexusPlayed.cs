/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared;

public class NexusPlayed : GameEvent, ILocationEvent
{
    #region Construction

    public NexusPlayed(Game game, Faction initiator) : base(game, initiator)
    {
    }

    public NexusPlayed()
    {
    }

    #endregion Construction

    #region Properties

    public Faction Faction { get; set; }

    public PrescienceAspect AtreidesPrescienceAspect { get; set; }

    public int TleilaxuForces { get; set; }

    public int TleilaxuSpecialForces { get; set; }

    public int _tleilaxuHeroId = -1;

    [JsonIgnore]
    public IHero? TleilaxuHero
    {
        get => LeaderManager.HeroLookup.Find(_tleilaxuHeroId);
        set => _tleilaxuHeroId = LeaderManager.HeroLookup.GetId(value);
    }

    public bool TleilaxuAssignSkill { get; set; } = false;

    public int _choamCardId;

    [JsonIgnore]
    public TreacheryCard? ChoamCard
    {
        get => TreacheryCardManager.Get(_choamCardId);
        set => _choamCardId = TreacheryCardManager.GetId(value);
    }

    public int _ecazTerritoryId;

    [JsonIgnore]
    public Territory? EcazTerritory
    {
        get => Game.Map.TerritoryLookup.Find(_ecazTerritoryId);
        set => _ecazTerritoryId = Game.Map.TerritoryLookup.GetId(value);
    }

    public Faction EcazFaction { get; set; }

    public int _moritaniTerritoryId;

    [JsonIgnore]
    public Territory? MoritaniTerritory
    {
        get => Game.Map.TerritoryLookup.Find(_moritaniTerritoryId);
        set => _moritaniTerritoryId = Game.Map.TerritoryLookup.GetId(value);
    }

    public int TleilaxuNumberOfSpecialForcesInLocation { get; set; }

    public int _tleilaxuLocationId = -1;

    [JsonIgnore]
    public Location? TleilaxuLocation
    {
        get => Game.Map.LocationLookup.Find(_tleilaxuLocationId);
        set => _tleilaxuLocationId = Game.Map.LocationLookup.GetId(value);
    }

    [JsonIgnore]
    public Location To => TleilaxuLocation!;

    [JsonIgnore]
    public int TotalAmountOfForcesAddedToLocation => TleilaxuNumberOfSpecialForcesInLocation;

    [JsonIgnore]
    public int ForcesAddedToLocation => 0;

    [JsonIgnore]
    public int SpecialForcesAddedToLocation => TleilaxuNumberOfSpecialForcesInLocation;


    [JsonIgnore]
    public bool IsCunning => Initiator == Faction;

    [JsonIgnore]
    public bool IsSecretAlly => !Game.IsPlaying(Faction);

    [JsonIgnore]
    public bool IsBetrayal => !(IsCunning || IsSecretAlly);

    #endregion Properties

    #region Validation

    public override Message? Validate()
    {
        switch (Faction)
        {
            case Faction.None: return Message.Express("Invalid Nexus faction");

            case Faction.Atreides:
                if ((IsCunning || IsSecretAlly) && AtreidesPrescienceAspect == PrescienceAspect.None) return Message.Express("Invalid battle plan element");
                break;

            case Faction.Tleilaxu:
                if (IsSecretAlly)
                {
                    if (TleilaxuHero != null && !ValidTleilaxuHeroes(Game, Player).Contains(TleilaxuHero)) return Message.Express("Invalid leader");
                    if (TleilaxuForces > ValidTleilaxuMaxAmount(Game, Player, false)) return Message.Express("You can't revive that many ", Player.Force);
                    if (TleilaxuSpecialForces > ValidTleilaxuMaxAmount(Game, Player, true)) return Message.Express("You can't revive that many ", Player.SpecialForce);
                    if (DetermineTleilaxuCost() > Player.Resources) return Message.Express("You can't pay that much");
                    if (TleilaxuForces + TleilaxuSpecialForces > 5) return Message.Express("You can't revive that many forces");
                    if (TleilaxuAssignSkill && TleilaxuHero == null) return Message.Express("You must revive a leader to assign a skill to");
                    if (TleilaxuAssignSkill && (TleilaxuHero == null || !Revival.MayAssignSkill(Game, Player, TleilaxuHero))) return Message.Express("You can't assign a skill to this leader");

                    if (TleilaxuLocation != null)
                    {
                        if (!Revival.ValidRevivedForceLocations(Game, Player).Contains(TleilaxuLocation)) return Message.Express("You can't place revived forces there");

                        if (TleilaxuNumberOfSpecialForcesInLocation > Revival.NumberOfSpecialForcesThatMayBePlacedOnPlanet(Player, TleilaxuSpecialForces)) return Message.Express("You can't place that many forces directly on the planet");
                    }
                }
                break;

            case Faction.Choam:
                if (IsCunning)
                {
                    if (ChoamCard == null) return Message.Express("Select a ", TreacheryCardType.Useless, " card to discard");
                    if (ChoamCard != null && !ValidChoamCards(Player).Contains(ChoamCard)) return Message.Express("Invalid card");
                }
                break;

            case Faction.Ecaz:
                if (IsBetrayal)
                {
                    if (EcazTerritory == null) return Message.Express("Select a territory");
                    if (!ValidEcazTerritories(Game).Contains(EcazTerritory)) return Message.Express("Invalid territory");
                }
                else if (IsSecretAlly)
                {
                    if (!ValidEcazFactions(Game).Contains(EcazFaction)) return Message.Express("Invalid faction");
                }
                break;

            case Faction.Moritani:
                if (IsBetrayal && (MoritaniTerritory == null || !ValidMoritaniTerritories(Game).Contains(MoritaniTerritory))) return Message.Express("This territory contains no terror token");
                break;


        }

        return null;
    }

    public static bool CanUseCunning(Player p)
    {
        return p.Nexus != Faction.None && p.Nexus == p.Faction;
    }

    public static bool CanUseSecretAlly(Game g, Player p)
    {
        return p.Nexus != Faction.None && !g.IsPlaying(p.Nexus);
    }

    public static bool CanUseBetrayal(Game g, Player p)
    {
        return p.Nexus != Faction.None && p.Nexus != p.Faction && g.IsPlaying(p.Nexus);
    }

    public static bool IsApplicable(Game g, Player p)
    {
        if (g.CurrentPhase == Phase.NexusCards || g.CurrentPhaseCannotBeInterrupted) return false;

        var cunning = CanUseCunning(p);
        var secretAlly = CanUseSecretAlly(g, p);
        var betrayal = CanUseBetrayal(g, p);

        var gameIsInBattle = g.CurrentPhase == Phase.BattlePhase && g.CurrentBattle != null;
        var isCurrentlyFormulatingBattlePlan = gameIsInBattle && g.CurrentBattle!.IsAggressorOrDefender(p) && (g.DefenderPlan == null || g.AggressorPlan == null);

        return p.Nexus switch
        {
            Faction.Atreides when betrayal => gameIsInBattle && g.CurrentBattle!.IsAggressorOrDefender(Faction.Atreides),
            Faction.Atreides when cunning || secretAlly => isCurrentlyFormulatingBattlePlan,

            Faction.Harkonnen when betrayal => g.CurrentPhase == Phase.CancellingTraitor,
            Faction.Harkonnen when cunning => true,
            Faction.Harkonnen when secretAlly => g.CurrentMainPhase == MainPhase.Contemplate,

            Faction.Fremen when betrayal => g.CurrentMainPhase == MainPhase.Blow || g.CurrentMainPhase == MainPhase.ShipmentAndMove,
            Faction.Fremen when secretAlly => g.CurrentMainPhase == MainPhase.Blow || g.CurrentMainPhase == MainPhase.Resurrection,

            Faction.Emperor when betrayal => g.CurrentMainPhase == MainPhase.Bidding || (gameIsInBattle && g.Applicable(Rule.EmperorSpecialForces) && g.CurrentBattle!.IsAggressorOrDefender(Faction.Emperor)),
            Faction.Emperor when cunning => isCurrentlyFormulatingBattlePlan,

            Faction.Guild when betrayal => g.CurrentMainPhase == MainPhase.ShipmentAndMove && g.HasRecentPaymentFor(typeof(Shipment)),
            Faction.Guild when cunning => g.CurrentPhase == Phase.GuildMove && !g.InGuildCunningShipment,
            Faction.Guild when secretAlly => g.CurrentPhase == Phase.NonGuildShip,

            Faction.BeneGesserit when betrayal => gameIsInBattle && g.CurrentBattle!.IsInvolved(Faction.BeneGesserit),
            Faction.BeneGesserit when cunning => g.CurrentMainPhase == MainPhase.ShipmentAndMove,

            Faction.Ixian when betrayal => g.CurrentMainPhase == MainPhase.Bidding && g.CurrentPhase < Phase.BiddingReport,
            Faction.Ixian when cunning => isCurrentlyFormulatingBattlePlan,

            Faction.Tleilaxu when betrayal => g.CurrentPhase == Phase.Facedancing,
            Faction.Tleilaxu when cunning => true,
            Faction.Tleilaxu when secretAlly => g.CurrentPhase == Phase.Resurrection,

            Faction.Choam when betrayal => true,
            Faction.Choam when secretAlly => (g.CurrentMainPhase == MainPhase.Collection && ValidChoamCards(p).Any()) || (g.CurrentPhase == Phase.BattleConclusion && g.CurrentBattle != null && p.Faction == g.BattleWinner),

            Faction.Richese when betrayal && g.CurrentMainPhase is MainPhase.Bidding => 
                    g.Version >= 164 ? 
                
                    (g.RicheseBiddingJustFinished || g.CardSoldOnBlackMarket != null) 
                    && (g.HasQuiteRecentPaymentTo(Faction.Richese) || (g.CardJustWon != null && g.GetPlayer(Faction.Richese)?.Has(g.CardJustWon) == true)) :

                    g.RicheseBiddingJustFinished && g.CardJustWon != null,

            Faction.Ecaz when betrayal => g.CurrentMainPhase < MainPhase.ShipmentAndMove && ValidEcazTerritories(g).Any(),
            Faction.Ecaz when cunning || secretAlly => true,

            Faction.Moritani when betrayal => ValidMoritaniTerritories(g).Any(),

            _ => false
        };
    }

    public static int ValidTleilaxuMaxAmount(Game g, Player p, bool specialForces)
    {
        if (specialForces)
        {
            if (p.Faction == Faction.Emperor || p.Faction == Faction.Fremen)
            {
                if (g.FactionsThatRevivedSpecialForcesThisTurn.Contains(p.Faction))
                    return 0;
                return Math.Min(p.SpecialForcesKilled, 1);
            }

            return Math.Min(p.SpecialForcesKilled, 5);
        }

        return Math.Min(p.ForcesKilled, 5);
    }

    public static IEnumerable<IHero> ValidTleilaxuHeroes(Game game, Player player)
    {
        return RaiseDeadPlayed.ValidHeroes(game, player);
    }

    public static IEnumerable<TreacheryCard> ValidChoamCards(Player player)
    {
        return player.TreacheryCards.Where(c => c.Type == TreacheryCardType.Useless);
    }

    public static IEnumerable<Territory> ValidEcazTerritories(Game g)
    {
        var ecaz = g.GetPlayer(Faction.Ecaz);

        if (ecaz != null && ecaz.HasAlly) return g.Map.Territories(false).Where(t => ecaz.AnyForcesIn(t) > 0 && ecaz.AlliedPlayer.AnyForcesIn(t) > 0);

        return Array.Empty<Territory>();
    }

    public static IEnumerable<Faction> ValidEcazFactions(Game g)
    {
        return g.Players.Where(p => p.Faction != Faction.Ecaz && p.Faction != Faction.Tleilaxu).Select(p => p.Faction);
    }

    public static IEnumerable<Territory> ValidMoritaniTerritories(Game g)
    {
        return g.TerrorOnPlanet.Values.Distinct();
    }

    public static bool MaySelectLocationForRevivedForces(Game game, Player player, int specialForces)
    {
        return player.Is(Faction.Fremen) && specialForces >= 1 && player.HasHighThreshold() &&
               Revival.ValidRevivedForceLocations(game, player).Any();
    }

    public int DetermineTleilaxuCost()
    {
        return DetermineTleilaxuCost(TleilaxuForces, TleilaxuSpecialForces);
    }

    public static int DetermineTleilaxuCost(int Forces, int SpecialForces)
    {
        return Forces + SpecialForces;
    }

    #endregion Validation

    #region Execution

    protected override void ExecuteConcreteEvent()
    {
        if (IsCunning)
            HandleCunning();
        else if (IsSecretAlly)
            HandleSecretAlly();
        else
            HandleBetrayal();
    }

    private void HandleBetrayal()
    {
        var actor = Player;
        if (actor == null) return;

        switch (Faction)
        {
            case Faction.Atreides:
                Game.PlayNexusCard(Player);
                Game.Prevent(Initiator, FactionAdvantage.AtreidesBattlePlanPrescience);
                Game.CurrentPrescience = null;
                break;

            case Faction.Harkonnen:
                var currentBattle = Game.CurrentBattle;
                var traitorDeck = Game.TraitorDeck;
                if (currentBattle == null || traitorDeck == null) break;
                var harkonnen = GetPlayer(Faction.Harkonnen);
                var traitor = currentBattle.PlanOfOpponent(Faction.Harkonnen).Hero;
                if (harkonnen != null && traitor != null)
                {
                    harkonnen.Traitors.Remove(traitor);
                    traitorDeck.Items.Add(traitor);
                    traitorDeck.Shuffle();
                    Game.Stone(Milestone.Shuffled);
                    Game.HarkonnenTraitorWasCancelled = true;
                    Game.PlayNexusCard(Player, "cancel the ", Faction.Harkonnen, " traitor call");
                    Game.Enter(Phase.CallTraitorOrPass);
                    Game.HandleRevealedBattlePlans();
                }
                break;

            case Faction.Fremen:
                Game.PlayNexusCard(Player);
                if (Game.CurrentMainPhase == MainPhase.Blow)
                {
                    Game.Prevent(Initiator, FactionAdvantage.FremenRidesMonster);
                    if (Game.CurrentPhase is Phase.FremenRidingMonsterA or Phase.FremenRidingMonsterB)
                    {
                        Game.DetermineNextShipmentAndMoveSubPhase();
                    }
                }
                else if (Game.CurrentMainPhase == MainPhase.ShipmentAndMove)
                {
                    Game.Prevent(Initiator, FactionAdvantage.FremenExtraMove);
                }

                break;

            case Faction.Emperor:
                Game.PlayNexusCard(Player);
                if (Game.CurrentMainPhase == MainPhase.Bidding)
                    Game.Prevent(Initiator, FactionAdvantage.EmperorReceiveBid);
                else if (Game.CurrentMainPhase == MainPhase.Battle && Game.Applicable(Rule.EmperorSpecialForces)) Game.Prevent(Initiator, FactionAdvantage.EmperorSpecialForceBonus);
                break;

            case Faction.Guild:
                Game.PlayNexusCard(Player);
                foreach (var p in Game.StoredRecentlyPaid)
                {
                    object from = p.To == Faction.None ? "the Bank" : p.To;
                    Log(Player, "get ", p.Amount, " from ", from);
                    if (p.To != Faction.None)
                    {
                        var getFrom = GetPlayer(p.To);
                        if (getFrom != null) getFrom.Resources -= p.Amount;
                    }

                    Player.Resources += p.Amount;
                }

                if (Game.TargetOfBureaucracy == Faction.Guild) Game.TargetOfBureaucracy = Initiator;

                break;

            case Faction.BeneGesserit:
                Game.PlayNexusCard(Player);
                Game.Prevent(Initiator, FactionAdvantage.BeneGesseritUsingVoice);
                Game.CurrentVoice = null;
                break;

            case Faction.Ixian:
                Game.PlayNexusCard(Player);
                if (Game.CurrentPhase < Phase.IxianSelectingCard)
                    Game.Prevent(Initiator, FactionAdvantage.IxianSelectingCardsOnAuction);
                else if (Game.CurrentPhase > Phase.IxianSelectingCard && Game.CurrentPhase < Phase.BiddingReport) Game.Prevent(Initiator, FactionAdvantage.IxianSwappingCard);
                break;

            case Faction.Tleilaxu:
                Game.FacedancerWasCancelled = true;
                Game.PlayNexusCard(Player, "cancel the ", Faction.Tleilaxu, " face dancer reveal");
                Game.FinishBattle();
                break;

            case Faction.Choam:
                Game.PlayNexusCard(Player, "force ", Faction.Choam, " to discard one of their treachery cards at random");
                var victimPlayer = GetPlayer(Faction.Choam);
                if (victimPlayer?.TreacheryCards.Any() == true)
                {
                    var card = victimPlayer.TreacheryCards.RandomOrDefault(Game.Random!);
                    if (card != null) Game.Discard(card);
                }
                else
                    Log(Faction.Choam, " have no treachery cards to discard");
                break;

            case Faction.Richese:
                var paymentToRichese = Game.QuiteRecentPaymentTo(Faction.Richese);
                var richese = GetPlayer(Faction.Richese);
                if (paymentToRichese != null)
                {
                    var amountReceived = paymentToRichese.Amount - (Game.WasVictimOfBureaucracy == Faction.Richese ? 2 : 0);
                    Game.PlayNexusCard(Player, "let ", Faction.Richese, " return the payment of ", Payment.Of(amountReceived), " they just received");
                    if (richese != null) richese.Resources -= amountReceived;
                }
                else if (richese != null && Game.CardJustWon != null && richese.Has(Game.CardJustWon))
                {
                    Game.PlayNexusCard(Player, "force ", Faction.Richese, " to discard the card they just won, refunding payment");
                    Game.Discard(richese, Game.CardJustWon);
                    if (Game is { Version: >= 174, WinningBid: { KarmaBid: false, Initiator: Faction.Richese } })
                    {
                        richese.Resources += Game.WinningBid.Amount;
                        var richeseAlly = richese.AlliedPlayer;
                        if (richeseAlly != null && Game.WinningBid.AllyContributionAmount > 0)
                        {
                            richeseAlly.Resources += Game.WinningBid.AllyContributionAmount;
                        }
                        if (richeseAlly != null && Game.WinningBid.EmperorContributionAmount > 0)
                        {
                            richeseAlly.Resources += Game.WinningBid.EmperorContributionAmount;
                        }
                    }
                }
                break;

            case Faction.Ecaz:
                var ecazsAlly = Game.GetPlayer(Faction.Ecaz)?.AlliedPlayer;
                if (ecazsAlly != null && EcazTerritory != null)
                {
                    ecazsAlly.ForcesToReserves(EcazTerritory);
                    Game.PlayNexusCard(Player, "return all ", ecazsAlly.Faction, " forces in ", EcazTerritory, " to reserves");
                    Game.FlipBeneGesseritAdvisorsWhenAlone();
                }
                break;

            case Faction.Moritani:
                if (MoritaniTerritory != null)
                {
                    var terrorToRemove = Game.TerrorIn(MoritaniTerritory).RandomOrDefault(Game.Random!);
                    Game.TerrorOnPlanet.Remove(terrorToRemove);
                    if (Game.Version >= 169) Game.UnplacedTerrorTokens.Add(terrorToRemove);
                    Game.PlayNexusCard(Player, "return a terror token from ", MoritaniTerritory, " to reserves");
                }
                break;

        }
    }

    private void HandleCunning()
    {
        var actor = Player;
        if (actor == null) return;

        switch (Faction)
        {
            case Faction.Atreides:
                Game.CurrentNexusPrescience = this;
                Game.PlayNexusCard(Player, "see their opponent's ", AtreidesPrescienceAspect);
                break;

            case Faction.Harkonnen:
                if (Game.TraitorDeck == null) break;
                Game.PhaseBeforeDiscardingTraitor = Game.CurrentPhase;
                Game.FactionThatMustDiscardTraitor = Initiator;
                Game.NumberOfTraitorsToDiscard = 1;
                Game.PlayNexusCard(Player, "draw a new traitor");
                actor.Traitors.Add(Game.TraitorDeck.Draw());
                Game.Enter(Phase.DiscardingTraitor);
                break;

            case Faction.Emperor:
                Game.CurrentEmperorCunning = this;
                Game.PlayNexusCard(Player, "let 5 ", FactionForce.Emperor, " count as ", FactionSpecialForce.Emperor, " during this battle");
                break;

            case Faction.Guild:
                Game.CurrentGuildCunning = this;
                Game.PlayNexusCard(Player, "perform an extra shipment after their move");
                break;

            case Faction.BeneGesserit:
                Game.CurrentBeneGesseritCunning = this;
                Game.PlayNexusCard(Player, "be able to flip advisor to fighters during ", MainPhase.ShipmentAndMove);
                break;

            case Faction.Ixian:
                Game.CurrentIxianCunning = this;
                Game.PlayNexusCard(Player, "let ", FactionForce.Ixian, " be full strength during this battle");
                break;

            case Faction.Tleilaxu:
                if (Game.TraitorDeck == null) break;
                if (GetPlayer(Faction.Tleilaxu) is not { } tleilaxu) break;

                Game.PlayNexusCard(actor, "replace their ", tleilaxu.RevealedFaceDancers.Count, " revealed face dancers");
                if (tleilaxu.RevealedFaceDancers.Count > 0)
                {
                    for (var i = 0; i < tleilaxu.RevealedFaceDancers.Count; i++) tleilaxu.FaceDancers.Add(Game.TraitorDeck.Draw());

                    Game.TraitorDeck.Items.AddRange(tleilaxu.RevealedFaceDancers);
                    Game.TraitorDeck.Shuffle();

                    foreach (var dancer in tleilaxu.RevealedFaceDancers) tleilaxu.FaceDancers.Remove(dancer);
                    tleilaxu.RevealedFaceDancers.Clear();

                    Game.Stone(Milestone.Shuffled);
                }
                break;

            case Faction.Ecaz:
                if (Game.Vidal != null && !Game.IsAlive(Game.Vidal))
                {
                    Game.Revive(Player, Game.Vidal);

                    if (TleilaxuAssignSkill) Game.PrepareSkillAssignmentToRevivedLeader(Player, Game.Vidal);
                }
                Game.TakeVidal(Player, VidalMoment.EndOfTurn);
                Game.PlayNexusCard(Player, "take ", Game.Vidal, " this turn");
                break;
        }
    }

    private void HandleSecretAlly()
    {
        var actor = Player;
        if (actor == null) return;

        switch (Faction)
        {
            case Faction.Atreides:
                Game.CurrentNexusPrescience = this;
                Game.PlayNexusCard(Player, "see their opponent's ", AtreidesPrescienceAspect);
                break;

            case Faction.Harkonnen:
                if (Game.TraitorDeck == null) break;
                Game.PhaseBeforeDiscardingTraitor = Game.CurrentPhase;
                Game.FactionThatMustDiscardTraitor = Initiator;
                Game.NumberOfTraitorsToDiscard = 2;
                Game.PlayNexusCard(Player, "draw two new traitors");
                actor.Traitors.Add(Game.TraitorDeck.Draw());
                actor.Traitors.Add(Game.TraitorDeck.Draw());
                Game.Enter(Phase.DiscardingTraitor);
                break;

            case Faction.Fremen:
                Game.CurrentFremenSecretAlly = this;
                if (Game.CurrentMainPhase == MainPhase.Blow)
                    Game.PlayNexusCard(Player, "prevent their forces from being devoured by ", Concept.Monster);
                else if (Game.CurrentMainPhase == MainPhase.Resurrection) Game.PlayNexusCard(Player, "increase their free revival to 3");
                break;

            case Faction.Guild:
                Game.CurrentGuildSecretAlly = this;
                Game.PlayNexusCard(Player, "be able to ship as ", Faction.Guild);
                break;

            case Faction.Tleilaxu:
                Game.Stone(Milestone.RaiseDead);
                var player = GetPlayer(Initiator);
                if (player == null) break;

                player.ReviveForces(TleilaxuForces);
                player.ReviveSpecialForces(TleilaxuSpecialForces);

                if (TleilaxuSpecialForces > 0) Game.FactionsThatRevivedSpecialForcesThisTurn.Add(Initiator);

                if (TleilaxuHero != null)
                {
                    Game.Revive(Player, TleilaxuHero);

                    if (TleilaxuAssignSkill && TleilaxuHero is Leader revivedLeader) Game.PrepareSkillAssignmentToRevivedLeader(player, revivedLeader);
                }

                Game.PlayNexusCard(Player, "revive ",
                    MessagePart.ExpressIf(TleilaxuHero != null, TleilaxuHero),
                    MessagePart.ExpressIf(TleilaxuHero != null && TleilaxuForces + TleilaxuSpecialForces > 0, " and "),
                    MessagePart.ExpressIf(TleilaxuForces > 0, TleilaxuForces, " ", player.Force),
                    MessagePart.ExpressIf(TleilaxuForces > 0 && TleilaxuSpecialForces > 0, " and "),
                    MessagePart.ExpressIf(TleilaxuSpecialForces > 0, TleilaxuSpecialForces, " ", player.SpecialForce));

                if (TleilaxuLocation != null && Initiator == Faction.Fremen)
                {
                    player.ShipSpecialForces(TleilaxuLocation, 1);
                    Log(Initiator, " place ", FactionSpecialForce.Fremen, " in ", TleilaxuLocation);
                }

                break;

            case Faction.Choam:
                if (Game.CurrentMainPhase == MainPhase.Collection)
                {
                    Game.PlayNexusCard(Player, "discard a ", TreacheryCardType.Useless, " card to get ", Payment.Of(2));
                    if (ChoamCard == null) break;
                    Game.Discard(Player, ChoamCard);
                    Player.Resources += 2;
                }
                else if (Game.CurrentPhase == Phase.BattleConclusion)
                {
                    var currentBattle = Game.CurrentBattle;
                    if (currentBattle == null) break;

                    var auditee = currentBattle.OpponentOf(Initiator);
                    if (auditee == null) break;
                    var recentBattlePlan = currentBattle.PlanOf(auditee);
                    if (recentBattlePlan == null) break;
                    var auditableCards = auditee.TreacheryCards.Where(c => c != recentBattlePlan.Weapon && c != recentBattlePlan.Defense && c != recentBattlePlan.Hero).ToList();

                    Game.PlayNexusCard(Player, "see a random treachery card in the ", auditee.Faction, " hand");

                    if (auditableCards.Any())
                    {
                        var auditedCard = auditableCards.RandomOrDefault(Game.Random!);
                        if (auditedCard != null)
                        {
                            Game.RegisterKnown(Player, auditedCard);
                            LogTo(Initiator, "You see: ", auditedCard);
                            LogTo(auditee.Faction, "You showed them: ", auditedCard);
                        }
                    }
                    else
                    {
                        Log(auditee.Faction, " don't have cards to audit");
                    }
                }
                break;

            case Faction.Ecaz:
                Game.PlayNexusCard(Player, "force ", EcazFaction, " to reveal if they have an ", Faction.Ecaz, " traitor");
                Log(EcazFaction, " reveal to ", Initiator, " if they have a ", Initiator, " traitor");
                var hasTraitor = GetPlayer(EcazFaction)?.Traitors.Any(t => t.Faction == Initiator) == true;
                LogTo(Initiator, EcazFaction, " reveal that they ", hasTraitor ? "DO" : "DON'T", " have a ", Initiator, " traitor ");
                LogTo(EcazFaction, " you revealed to them that you ", hasTraitor ? "DO" : "DON'T", " have a ", Initiator, " traitor ");
                break;

        }
    }

    public override Message GetMessage()
    {
        return Message.Express(Initiator, " play a Nexus card");
    }

    #endregion Execution
}