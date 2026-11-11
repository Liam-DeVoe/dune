/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared;

public partial class Game
{
    #region State

    public GameSettings Settings { get; set; } = new();
    public List<FactionTradeOffered> CurrentTradeOffers { get; } = [];
    internal Phase PhaseBeforeSkillAssignment { get; private set; }
    public Faction NextFactionToPerformCustomSetup => Players.Select(p => p.Faction).FirstOrDefault(f => !HasActedOrPassed.Contains(f));
    public Deck<TreacheryCard>? StartingTreacheryCards { get; private set; }
    private TreacheryCard? ExtraStartingCardForHarkonnen { get; set; }

    #endregion State

    #region Setup

    private void EnterPhaseAwaitingPlayers()
    {
        Players = [];
        CurrentTurn = 0;
        Enter(Phase.AwaitingPlayers);
    }

    internal void AssignFactionsAndEnterFactionTrade()
    {
        var availableFactions = Version < 174 ? FactionsInPlay : FactionsInPlay.Where(f => !IsPlaying(f));
        var selectable = new Deck<Faction>(availableFactions, Random!);
        Stone(Milestone.Shuffled);
        selectable.Shuffle();

        foreach (var p in Players.Where(p => p.Faction == Faction.None))
        {
            p.Faction = selectable.Draw();
        }

        if (Version < 172)
            DeterminePositionsAtTable();            

        Enter(Applicable(Rule.CustomDecks) && Version < 134, Phase.CustomizingDecks, EnterPhaseTradingFactions);
    }

    internal void DeterminePositionsAtTable()
    {
        var seats = new Deck<int>(Random!);
        for (var i = 0; i < Settings.NumberOfPlayers; i++)
            seats.PutOnTop(i);
        
        seats.Shuffle();

        //Seat players
        foreach (var p in Players)
        {
            p.Seat = seats.Draw();
        }
    }

    internal void EnterPhaseTradingFactions()
    {
        CurrentTradeOffers.Clear();
        Enter(Phase.TradingFactions);
    }

    internal void EnterSetupPhase()
    {
        Enter(Applicable(Rule.CustomDecks) && Version >= 134, Phase.CustomizingDecks, EnterPhaseTradingFactions);

        CurrentTradeOffers.Clear();

        HasBattleWheel.Add(Players.OrderBy(p => p.Seat).First().Faction);
        HasBattleWheel.Add(Players.OrderBy(p => p.Seat).Last().Faction);

        foreach (var p in Players) p.AssignLeaders(this);

        var ecaz = GetPlayer(Faction.Ecaz);
        if (ecaz != null)
        {
            AssignInitialAmbassadors(ecaz);

            if (Applicable(Rule.EcazLoyalty))
            {
                EcazLoyalLeader = ecaz.Leaders.RandomOrDefault(Random!);
                Log(EcazLoyalLeader!, " is forever loyal to ", Faction.Ecaz);
            }
        }

        Enter(IsPlaying(Faction.BeneGesserit), Phase.BeneGesseritPredicting, TreacheryCardsBeforeTraitors, DealStartingTreacheryCards, DealTraitors);
    }

    internal void DealTraitors()
    {
        Stone(Milestone.Shuffled);
        TraitorDeck = CreateAndShuffleTraitorDeck(Random!);

        if (Applicable(Rule.HarkonnenMulligan) && IsPlaying(Faction.Harkonnen))
        {
            DealHarkonnenTraitorCards();
            Enter(Phase.HarkonnenMulligan);
        }
        else
        {
            for (var i = 1; i <= 4; i++)
                foreach (var p in Players.Where(p => p.Faction != Faction.Tleilaxu)) p.Traitors.Add(TraitorDeck.Draw());

            EnterSelectTraitors();
        }
    }

    private Deck<IHero> CreateAndShuffleTraitorDeck(LoggedRandom random)
    {
        var result = new Deck<IHero>(TraitorsInPlay, random);

        if (EcazLoyalLeader != null)
        {
            result.Items.Remove(EcazLoyalLeader);
            foreach (var p in Players) p.KnownNonTraitors.Add(EcazLoyalLeader);
        }

        if (Vidal != null)
            foreach (var p in Players) p.KnownNonTraitors.Add(Vidal);

        result.Shuffle();
        return result;
    }

    internal void DealHarkonnenTraitorCards()
    {
        var harkonnen = GetPlayer(Faction.Harkonnen);
        if (harkonnen is null) return;
        for (var i = 1; i <= 4; i++) harkonnen.Traitors.Add(TraitorDeck!.Draw());
    }

    internal void EnterSelectTraitors()
    {
        HasActedOrPassed.Clear();

        if (IsPlaying(Faction.Harkonnen)) HasActedOrPassed.Add(Faction.Harkonnen);

        if (IsPlaying(Faction.Tleilaxu)) HasActedOrPassed.Add(Faction.Tleilaxu);

        Enter(Players.Any(p => !(p.Is(Faction.Harkonnen) || p.Is(Faction.Tleilaxu))), Phase.SelectingTraitors, AssignFaceDancers);
    }

    internal void AssignFaceDancers()
    {
        var tleilaxu = GetPlayer(Faction.Tleilaxu);
        if (tleilaxu != null)
        {
            TraitorDeck!.Shuffle();
            for (var i = 0; i < 3; i++)
            {
                var leader = TraitorDeck.Draw();
                tleilaxu.FaceDancers.Add(leader);
                tleilaxu.KnownNonTraitors.Add(leader);
            }
        }

        Enter(TreacheryCardsBeforeTraitors, SetupSpiceAndForces, AssignLeaderSkills);
    }

    private void AssignLeaderSkills()
    {
        if (Applicable(Rule.LeaderSkills))
        {
            SkillDeck = new Deck<LeaderSkill>(Enumerations.GetValuesExceptDefault(LeaderSkill.None), Random!);
            SkillDeck.Shuffle();
            Stone(Milestone.Shuffled);

            var nrOfSkillsToAssign = Players.Count <= 7 ? 2 : 1;
            for (var i = 0; i < nrOfSkillsToAssign; i++)
                foreach (var p in Players) p.SkillsToChooseFrom.Add(SkillDeck.Draw());

            Enter(Phase.AssigningInitialSkills);
        }
        else
        {
            Enter(TreacheryCardsBeforeTraitors, DealTraitors, SetupSpiceAndForces);
        }
    }

    internal void SetupSpiceAndForces()
    {
        foreach (var p in Players) 
            SetupPlayerHomeworld(p);

        if (Applicable(Rule.CustomInitialForcesAndResources))
        {
            HasActedOrPassed.Clear();
            Enter(Phase.PerformCustomSetup);
        }
        else
        {
            foreach (var p in Players) SetupPlayerSpiceAndForcesOnPlanet(p);

            Action methodAfterSettingUp;
            if (TreacheryCardsBeforeTraitors)
                methodAfterSettingUp = EnterStormPhase;
            else
                methodAfterSettingUp = DealStartingTreacheryCards;

            Enter(
                IsPlaying(Faction.Fremen), Phase.FremenSettingUp,
                IsPlaying(Faction.BeneGesserit) && PerformBeneGesseritPlacement.BeneGesseritMayPlaceFirstForceInAnyTerritory(this), Phase.BeneGesseritSettingUp,
                IsPlaying(Faction.Moritani), Phase.MoritaniSettingUp,
                methodAfterSettingUp);
        }
    }

    private void SetupPlayerHomeworld(Player p)
    {
        var normalForceWorld = Map.Homeworlds.First(w => w.Faction == p.Faction && w.IsHomeOfNormalForces);

        switch (p.Faction)
        {
            case Faction.Fremen:
                p.InitializeHomeworld(normalForceWorld, Applicable(Rule.FremenSpecialForces) ? 17 : 20, Applicable(Rule.FremenSpecialForces) ? 3 : 0);
                break;

            case Faction.Emperor:
                p.InitializeHomeworld(normalForceWorld, Applicable(Rule.EmperorSpecialForces) ? 15 : 20, 0);
                if (Applicable(Rule.EmperorSpecialForces)) 
                    p.InitializeHomeworld(Map.Homeworlds.First(w => w.Faction == p.Faction && w.IsHomeOfSpecialForces), 0, 5);
                break;

            case Faction.Ixian:
                p.InitializeHomeworld(normalForceWorld, 13, 7);
                break;

            case Faction.Atreides:
            case Faction.Harkonnen:
            case Faction.Guild:
            case Faction.BeneGesserit:
            case Faction.Tleilaxu:
            case Faction.Choam:
            case Faction.Richese:
            case Faction.Ecaz:
            case Faction.Moritani:
                p.InitializeHomeworld(normalForceWorld, 20, 0);
                break;
        }
    }

    private void SetupPlayerSpiceAndForcesOnPlanet(Player p)
    {
        switch (p.Faction)
        {
            case Faction.Fremen:
                p.Resources = 3;
                break;

            case Faction.Atreides:
                p.Resources = 10;
                p.AddForces(Map.Arrakeen, 10, true);
                break;

            case Faction.Harkonnen:
                p.Resources = 10;
                p.AddForces(Map.Carthag, 10, true);
                break;

            case Faction.Emperor:
                p.Resources = 10;
                break;

            case Faction.Guild:
                p.Resources = 5;
                p.AddForces(Map.TueksSietch, 5, true);
                break;

            case Faction.BeneGesserit:
                p.Resources = 5;
                if (!PerformBeneGesseritPlacement.BeneGesseritMayPlaceFirstForceInAnyTerritory(this)) p.AddForces(Map.PolarSink, 1, true);
                break;

            case Faction.Ixian:
                p.Resources = 10;
                p.AddForces(Map.HiddenMobileStronghold, 3, true);
                p.AddSpecialForces(Map.HiddenMobileStronghold, 3, true);
                break;

            case Faction.Tleilaxu:
                p.Resources = 5;
                break;

            case Faction.Choam:
                p.Resources = 2;
                break;

            case Faction.Richese:
                p.Resources = 5;
                break;

            case Faction.Ecaz:
                p.Resources = 12;
                p.AddForces(Map.ImperialBasin.MiddleLocation, 6, true);
                break;

            case Faction.Moritani:
                p.Resources = 12;
                break;
        }
    }

    private void AssignInitialAmbassadors(Player p)
    {
        p.Ambassadors.Add(Ambassador.Ecaz);
        UnassignedAmbassadors!.Items.Remove(Ambassador.Ecaz);
        Log(p.Faction, " receive the ", Faction.Ecaz, " ambassador");
        AssignRandomAmbassadors(p);
    }

    internal void AssignRandomAmbassadors(Player p)
    {
        foreach (var item in AmbassadorsSetAside) UnassignedAmbassadors!.Items.Add(item);
        AmbassadorsSetAside.Clear();

        UnassignedAmbassadors!.Shuffle();
        Stone(Milestone.Shuffled);
        Log(p.Faction, " get 5 random Ambassadors");
        for (var i = 0; i < 5; i++) p.Ambassadors.Add(UnassignedAmbassadors.Draw());
    }

    internal void DealStartingTreacheryCards()
    {
        StartingTreacheryCards = new Deck<TreacheryCard>(Random!);
        foreach (var p in Players)
        {
            StartingTreacheryCards.Items.Add(TreacheryDeck!.Draw());

            if (p.Is(Faction.Harkonnen)) ExtraStartingCardForHarkonnen = TreacheryDeck.Draw();
        }

        Enter(IsPlaying(Faction.Ixian), Phase.IxianSelectingCard, DealRemainingStartingTreacheryCardsToNonIxian);
    }

    internal void DealRemainingStartingTreacheryCardsToNonIxian()
    {
        foreach (var p in Players.Where(p => p.Faction != Faction.Ixian))
        {
            var card = StartingTreacheryCards!.Draw();
            p.TreacheryCards.Add(card);
            LogTo(p.Faction, "Your starting treachery card is: ", card);

            if (p.Is(Faction.Harkonnen) && ExtraStartingCardForHarkonnen != null)
            {
                p.TreacheryCards.Add(ExtraStartingCardForHarkonnen);
                LogTo(Faction.Harkonnen, "Your extra card is: ", ExtraStartingCardForHarkonnen);
            }
        }

        Enter(TreacheryCardsBeforeTraitors, AssignLeaderSkills, EnterStormPhase);
    }

    internal bool TreacheryCardsBeforeTraitors => Version >= 121 && Applicable(Rule.LeaderSkills);

    #endregion Setup
}