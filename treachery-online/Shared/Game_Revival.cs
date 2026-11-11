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
    #region BeginningOfRevival

    internal bool RevivalTechTokenIncome { get; set; }
    public Dictionary<Faction, int> TotalRevivalsThisTurn { get; set; } = [];
    public Dictionary<Faction, int> FreeRevivalsThisTurn { get; } = [];
    public List<Faction> FactionsThatRevivedSpecialForcesThisTurn { get; } = [];
    public Dictionary<Faction, int> LeaderRevivalsThisTurn { get; } = [];
    internal bool TleilaxuStartedRevivalWithLowThreshold { get; set; }
    public RecruitsPlayed? CurrentRecruitsPlayed { get; set; }
    public Faction[] FactionsWithIncreasedRevivalLimits { get; internal set; } = [];
    public List<RequestTleilaxuRevival> CurrentRevivalRequests { get; } = [];
    public Dictionary<IHero, int> EarlyRevivalsOffers { get; } = new();
    public ChoamFreeRevivalPrevention? CurrentFreeRevivalPrevention { get; internal set; }
    internal KarmaRevivalPrevention? CurrentKarmaRevivalPrevention { get; set; }
    public int AmbassadorsPlacedThisTurn { get; internal set; }
    

    #endregion

    #region Revival

    internal void PrepareSkillAssignmentToRevivedLeader(Player player, Leader? leader)
    {
        if (CurrentPhase != Phase.AssigningSkill && leader != null) 
            PhaseBeforeSkillAssignment = CurrentPhase;

        player.MostRecentlyRevivedLeader = leader;
        SkillDeck!.Shuffle();
        Stone(Milestone.Shuffled);
        player.SkillsToChooseFrom.Add(SkillDeck.Draw());
        player.SkillsToChooseFrom.Add(SkillDeck.Draw());
        Enter(Phase.AssigningSkill);
    }

    #endregion

    #region Information

    public int FreeRevivals(Player player, bool usesEmperorSecretAlly)
    {
        if (FreeRevivalPrevented(player.Faction)) return 0;

        if (Version < 181 && FreeRevivalsThisTurn.ContainsKey(player.Faction)) return 0;

        var nrOfFreeRevivals = 0;

        switch (player.Faction)
        {
            case Faction.Fremen: nrOfFreeRevivals = 3; break;
            case Faction.Atreides: nrOfFreeRevivals = 2; break;
            case Faction.Harkonnen: nrOfFreeRevivals = 2; break;
            case Faction.Emperor: nrOfFreeRevivals = 1; break;
            case Faction.Guild: nrOfFreeRevivals = 1; break;
            case Faction.BeneGesserit: nrOfFreeRevivals = 1; break;
            case Faction.Ixian: nrOfFreeRevivals = 1; break;
            case Faction.Tleilaxu: nrOfFreeRevivals = 2; break;

            case Faction.Choam: nrOfFreeRevivals = 0; break;
            case Faction.Richese: nrOfFreeRevivals = 2; break;
            case Faction.Ecaz: nrOfFreeRevivals = 2; break;
            case Faction.Moritani: nrOfFreeRevivals = 2; break;
        }
        
        if (Version < 176 && CurrentRecruitsPlayed != null) nrOfFreeRevivals *= 2;

        if (player.Ally == Faction.Fremen && FremenAllowsThreeFreeRevivals ||
            CurrentFremenSecretAlly != null && CurrentFremenSecretAlly.Player == player) nrOfFreeRevivals = 3;
        
        if (Version is >= 176 and <= 179 && CurrentRecruitsPlayed != null) nrOfFreeRevivals *= 2;

        if (usesEmperorSecretAlly) nrOfFreeRevivals += 3;

        if (GetsExtraCharityAndFreeRevivalDueToLowThreshold(player)) nrOfFreeRevivals += 1;
        
        if (Version >= 179 && CurrentRecruitsPlayed != null) nrOfFreeRevivals *= 2;

        return Math.Max(nrOfFreeRevivals - FreeRevivalsThisTurn.GetValueOrDefault(player.Faction, 0), 0);
    }

    private bool GetsExtraCharityAndFreeRevivalDueToLowThreshold(Player player)
    {
        return (!(player.Is(Faction.Emperor) || player.Is(Faction.Choam)) && player.HasLowThreshold()) ||
               (player.Is(Faction.Emperor) && player.HasLowThreshold(World.Emperor)) ||
               (player.Is(Faction.Choam) && player.HasLowThreshold() && OccupierOf(World.Choam) == null);
    }

    public int GetMaxRevivals(Game g, Player p, bool emperorSecretAllyUsed)
    {
        //int nrOfFreeRevivals = g.FreeRevivals(p, emperorSecretAllyUsed);

        if (p.Is(Faction.Tleilaxu) || (p.Is(Faction.Choam) && !g.Prevented(FactionAdvantage.ChoamRevival)))
            return int.MaxValue;

        var emperorSecretAlly = emperorSecretAllyUsed ? 3 : 0;
        var lowThreshold = g.HasLowThreshold(p.Faction) ? 1 : 0;
        var alreadyRevived = g.TotalRevivalsThisTurn.GetValueOrDefault(p.Faction, 0);
        
        if (CurrentRecruitsPlayed != null)
            return Math.Max(7 + emperorSecretAlly + lowThreshold - alreadyRevived, 0);
        
        if (FactionsWithIncreasedRevivalLimits.Contains(p.Faction))
            return Math.Max(5 + lowThreshold + emperorSecretAlly - alreadyRevived, 0);
        
        return Math.Max(3 + lowThreshold + emperorSecretAlly - alreadyRevived, 0);
    }

    private bool FreeRevivalPrevented(Faction f)
    {
        return CurrentFreeRevivalPrevention?.Target == f;
    }

    private bool PreventedFromReviving(Faction f)
    {
        return CurrentKarmaRevivalPrevention != null && CurrentKarmaRevivalPrevention.Target == f;
    }

    public Ambassador AmbassadorIn(Territory t)
    {
        return AmbassadorsOnPlanet.GetValueOrDefault(t, Ambassador.None);
    }

    #endregion Information

    public void ClaimFreeRevival(Player player)
    {
        var maxFreeRevivals = FreeRevivals(player, false);
        
        if (maxFreeRevivals <= 0) return;
        
        var freeRevivedSpecialForces = 0;
        var freeRevivedNormalForces = 0;
        
        var maxRevivableSpecialForces = Revival.ValidMaxRevivals(this, player, true, false);
        var maxRevivableNormalForces = Revival.ValidMaxRevivals(this, player, false, false);
        
        while (freeRevivedSpecialForces < maxFreeRevivals &&
               freeRevivedSpecialForces < maxRevivableSpecialForces)
        {
            freeRevivedSpecialForces++;
        }
        
        while (freeRevivedSpecialForces + freeRevivedNormalForces < maxFreeRevivals &&
               freeRevivedNormalForces < maxRevivableNormalForces)
        {
            freeRevivedNormalForces++;
        }
        
        if (freeRevivedNormalForces + freeRevivedSpecialForces <= 0) return;
        
        if (freeRevivedNormalForces > 0)
            player.ReviveForces(freeRevivedNormalForces);

        if (freeRevivedSpecialForces > 0)
            player.ReviveSpecialForces(freeRevivedSpecialForces);
       
        if (player.Faction != Faction.Tleilaxu) RevivalTechTokenIncome = true;

        var tleilaxu = GetPlayer(Faction.Tleilaxu);
        var tleilaxuReceivesIncome = false;
        
        if (tleilaxu != null)
        {
            tleilaxuReceivesIncome = !TleilaxuStartedRevivalWithLowThreshold && !Prevented(FactionAdvantage.TleilaxuReceiveRevive);

            if (tleilaxuReceivesIncome)
            {
                tleilaxu.Resources += 1;
            }
        }
        
        FreeRevivalsThisTurn[player.Faction] = FreeRevivalsThisTurn.GetValueOrDefault(player.Faction, 0) + freeRevivedNormalForces + freeRevivedSpecialForces;
        TotalRevivalsThisTurn[player.Faction] = TotalRevivalsThisTurn.GetValueOrDefault(player.Faction, 0) + freeRevivedNormalForces + freeRevivedSpecialForces;
        
        Log(Message.Express(player.Faction, " revive ",
            MessagePart.ExpressIf(freeRevivedSpecialForces > 0, freeRevivedSpecialForces, player.SpecialForce),
            MessagePart.ExpressIf(freeRevivedNormalForces > 0, freeRevivedNormalForces, player.Force),
            " for free",
            MessagePart.ExpressIf(tleilaxuReceivesIncome, Faction.Tleilaxu, " receive ", Payment.Of(1)))
            .WithLog(new RevivalRow(player.Faction, freeRevivedNormalForces, freeRevivedSpecialForces, 0)
            {
                TleilaxuIncome = tleilaxuReceivesIncome ? 1 : 0
            }));
        
        Stone(Milestone.Revival);
    }
}