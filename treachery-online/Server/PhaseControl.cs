using Treachery.Shared.Model;

namespace Treachery.Server;

/// <summary>
/// What a pause between phases waits for before the server continues it.
/// </summary>
/// <param name="WaitFor">Players who may act here, each of whom must act or pass.</param>
/// <param name="Countdown">Whether a secret card that anyone might hold could be played here. The pause then runs a
/// countdown that any player can hold.</param>
public record PauseRequirements(IReadOnlyList<Player> WaitFor, bool Countdown)
{
    public PauseRequirements(IEnumerable<Player> waitFor, bool countdown) : this(waitFor.ToList(), countdown) { }
}

/// <summary>
/// Decides, for each pause that the host used to end with Continue, who the pause waits for and whether it runs a
/// countdown. The pause continues on its own once both are done.
///
/// Waiting for a player reveals that they have an option, so a player is only waited for when everyone can see
/// they have one. Abilities that depend on a secret card anyone might hold get the countdown instead, which any
/// player can hold whether or not they have a card. Countdown conditions only use what is fixed when the game
/// starts: game settings, the factions playing and the turn number.
///
/// Only reads game state. It never calls GameStatus.DetermineStatus, which changes state. It calls
/// GetApplicableEvents for CardTraded and the revival actions only, as bots and the async-play mails already do on the live game.
/// </summary>
public static class PhaseControl
{
    /// <summary>Games older than this keep host-only control.</summary>
    public const int FirstVersion = 188;

    public const int CountdownMilliseconds = 5000;

    private static readonly PauseRequirements Nothing = new([], false);

    /// <summary>
    /// The requirements of the current pause, or null when the current phase is not a pause this class controls.
    /// The host keeps sole control of those.
    /// </summary>
    public static PauseRequirements? Determine(Game g)
    {
        if (g.Version < FirstVersion) return null;

        var amal = AmalInPlay(g);
        var nexus = g.Applicable(Rule.NexusCards);

        return g.CurrentPhase switch
        {
            Phase.SelectingFactions => Wait(g.Players.Where(p => p.Faction == Faction.None)),
            Phase.TradingFactions => Wait(g.Players),

            Phase.BeginningOfStorm => new(Players(g, p => DiscoveryEntered.ValidTargets(g, p).Any()), amal),
            Phase.MetheorAndStormSpell => new(Players(g, p => TestingStationUsed.CanBePlayed(g, p)), g.CurrentTurn > 1),
            Phase.StormReport => EndOfPhase(g),

            Phase.Thumper => new([], amal || ThumperInPlay(g) || g.IsPlaying(Faction.Fremen) || SecretAlly(g, nexus, Faction.Fremen)),
            Phase.HarvesterA or Phase.HarvesterB => new([], true),
            Phase.BlowReport => EndOfPhase(g, FremenSandworm(g)).WithCountdown(amal),

            Phase.BeginningOfCharity => new([], amal || g.IsPlaying(Faction.Choam) || (g.IsPlaying(Faction.BeneGesserit) && g.Applicable(Rule.BeneGesseritAutoCharity))),
            Phase.ClaimingCharity => new([], true),
            Phase.CharityReport => EndOfPhase(g),

            Phase.BeginningOfBidding => EndOfPhase(g).WithCountdown(amal || g.IsPlaying(Faction.Richese) || g.IsPlaying(Faction.Ixian)),
            Phase.ReplacingCardJustWon => Wait(Present(g.GetPlayer(g.FactionThatMayReplaceBoughtCard))),
            Phase.WaitingForNextBiddingRound => Wait(Present(Atreides(g))),
            Phase.BiddingReport => EndOfPhase(g, Present(TleilaxuLimits(g), EmperorDiscard(g))),

            Phase.BeginningOfResurrection => new(Present(TleilaxuLimits(g) ?? TleilaxuKarma(g)), amal || g.Applicable(Rule.Expansion3TreacheryCards) || SecretAlly(g, nexus, Faction.Fremen)),
            Phase.Resurrection => Wait(Revivers(g)),
            Phase.ResurrectionReport => EndOfPhase(g, Present(Ecaz(g))),

            Phase.BeginningOfShipAndMove => EndOfPhase(g).WithCountdown(amal || g.Applicable(Rule.RicheseTreacheryCards) || g.IsPlaying(Faction.Guild)),
            Phase.ShipmentAndMoveConcluded => EndOfPhase(g, Players(g, p => p.NoFieldIsActive)),

            Phase.BeginningOfBattle => new([], amal),
            Phase.CancellingTraitor => new([], true),
            Phase.BattleReport when g.CurrentMoment != MainPhaseMoment.End => Nothing,
            Phase.BattleReport => EndOfPhase(g),

            Phase.BeginningOfCollection => new(Present(MoritaniRemovingTokens(g)), amal || g.IsPlaying(Faction.Ecaz)),
            Phase.CollectionReport => EndOfPhase(g, Present(MoritaniRemovingTokens(g))).WithCountdown(SecretAlly(g, nexus, Faction.Choam)),

            Phase.Extortion => new([], true),
            Phase.Contemplate => new(Present(ChoamInMentat(g), MoritaniPlanting(g)), SecretAlly(g, nexus, Faction.Harkonnen)),
            Phase.TurnConcluded => EndOfPhase(g),

            _ => null
        };
    }

    /// <summary>
    /// Whether <paramref name="e"/> counts as its player being done with the current pause. A revival does: most
    /// players revive once, and anyone who wants to revive again can still do so while others are deciding.
    /// </summary>
    public static bool FinishesPause(Game g, GameEvent e) => g.CurrentPhase == Phase.Resurrection && e is Revival;

    private static PauseRequirements WithCountdown(this PauseRequirements r, bool countdown) =>
        r with { Countdown = r.Countdown || countdown };

    private static PauseRequirements Wait(IEnumerable<Player> players) => new(players.ToList(), false);

    private static IEnumerable<Player> Players(Game g, Func<Player, bool> canAct) => g.Players.Where(canAct);

    private static IEnumerable<Player> Present(params Player?[] players) => players.OfType<Player>();

    /// <summary>
    /// An end-of-phase pause: CHOAM may discard and CHOAM may trade a card with its ally, plus whoever else may
    /// act there.
    /// </summary>
    private static PauseRequirements EndOfPhase(Game g, IEnumerable<Player>? others = null)
    {
        var result = new List<Player>();

        var choam = g.GetPlayer(Faction.Choam);
        if (choam != null && choam.TreacheryCards.Count > 0 && !g.Prevented(FactionAdvantage.ChoamDiscarding))
            result.Add(choam);

        // The trade's conditions are all public: the alliance, both hand sizes and whether a trade was logged
        // this turn. Either side may start it.
        foreach (var p in g.Players.Where(p => (p.Faction == Faction.Choam && p.HasAlly) || p.Ally == Faction.Choam))
            if (!result.Contains(p) && g.GetApplicableEvents(p, false).Contains(typeof(CardTraded)))
                result.Add(p);

        foreach (var p in others ?? [])
            if (!result.Contains(p))
                result.Add(p);

        return new(result, false);
    }

    /// <summary>
    /// Players who may revive, ask Tleilaxu for a revival or answer such a request, or (for Tleilaxu) still set
    /// revival limits. All of these depend only on what is in the tanks, alliances and Karma played.
    /// </summary>
    private static IEnumerable<Player> Revivers(Game g)
    {
        Type[] revivalActions = [typeof(Revival), typeof(RequestTleilaxuRevival), typeof(AcceptOrCancelTleilaxuRevival)];
        var limits = TleilaxuLimits(g);

        return g.Players.Where(p => p == limits || g.GetApplicableEvents(p, false).Any(revivalActions.Contains));
    }

    private static bool AmalInPlay(Game g) =>
        g.Applicable(Rule.ExpansionTreacheryCardsAmal) ||
        (g.Version <= 104 && g.Applicable(Rule.ExpansionTreacheryCardsExceptPBandSSandAmal));

    private static bool ThumperInPlay(Game g) => g.Applicable(Rule.ExpansionTreacheryCardsExceptPBandSSandAmal);

    /// <summary>A Nexus card's "secret ally" mode, usable while that faction is not playing.</summary>
    private static bool SecretAlly(Game g, bool nexus, Faction f) => nexus && !g.IsPlaying(f);

    private static Player? SpecialKarma(Game g, Faction f)
    {
        var p = g.GetPlayer(f);
        return p != null && g.Applicable(Rule.AdvancedKarama) && !p.SpecialKarmaPowerUsed && !g.KarmaPrevented(f) ? p : null;
    }

    /// <summary>Fremen's once-per-game Karma sandworm. Only during Spice Blow, from turn 2.</summary>
    private static IEnumerable<Player> FremenSandworm(Game g) =>
        Present(g.CurrentTurn > 1 ? SpecialKarma(g, Faction.Fremen) : null);

    /// <summary>Tleilaxu's once-per-game Karma, which stops a faction reviving.</summary>
    private static Player? TleilaxuKarma(Game g) => SpecialKarma(g, Faction.Tleilaxu);

    /// <summary>
    /// Tleilaxu may raise revival limits at the end of Bidding or the start of Revival. The limits are reset
    /// when Ship &amp; Move starts, so any set limits were set this turn.
    /// </summary>
    private static Player? TleilaxuLimits(Game g)
    {
        var p = g.GetPlayer(Faction.Tleilaxu);
        return p != null && g.Players.Count > 1 && !g.Prevented(FactionAdvantage.TleilaxuIncreasingRevivalLimits) &&
               g.FactionsWithIncreasedRevivalLimits.Length == 0
            ? p
            : null;
    }

    /// <summary>The Emperor may pay 2 to discard a card at high threshold. Spice is secret, so it is not checked.</summary>
    private static Player? EmperorDiscard(Game g)
    {
        var p = g.GetPlayer(Faction.Emperor);
        return p != null && p.HasHighThreshold() ? p : null;
    }

    private static Player? Atreides(Game g) =>
        g.Prevented(FactionAdvantage.AtreidesBiddingPrescience) ? null : g.GetPlayer(Faction.Atreides);

    /// <summary>Ecaz places ambassadors at the end of Revival. Its spice is secret, so it is not checked.</summary>
    private static Player? Ecaz(Game g) =>
        g.Prevented(FactionAdvantage.EcazAmbassadors) ? null : g.GetPlayer(Faction.Ecaz);

    private static Player? MoritaniRemovingTokens(Game g)
    {
        var p = g.GetPlayer(Faction.Moritani);
        return p != null && TerrorPlanted.MayRemoveTokens(g, p) ? p : null;
    }

    /// <summary>Moritani plants a terror token once per turn. Its unplaced tokens are secret, so they are not checked.</summary>
    private static Player? MoritaniPlanting(Game g) =>
        !g.MoritaniHasPerformedTerror && !g.Prevented(FactionAdvantage.MoritaniPlantingTerror) ? g.GetPlayer(Faction.Moritani) : null;

    /// <summary>In the Mentat phase CHOAM may play Inflation or, with cards, Trip to Gamont.</summary>
    private static Player? ChoamInMentat(Game g)
    {
        var p = g.GetPlayer(Faction.Choam);
        if (p == null) return null;

        var inflation = g.EconomicsStatus == ChoamEconomicsStatus.None && !g.Prevented(FactionAdvantage.ChoamEconomics);
        return inflation || p.TreacheryCards.Count > 0 ? p : null;
    }
}
