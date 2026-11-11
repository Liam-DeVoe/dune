using System;
using System.Collections.Generic;
using System.Linq;
using Microsoft.VisualStudio.TestTools.UnitTesting;
using Treachery.Bots;
using Treachery.Server;
using Treachery.Shared;
using Treachery.Shared.Model;

namespace Treachery.Test;

[TestClass]
public class PhaseControlTests
{
    private static readonly int[] Seeds = [1, 2, 3, 4, 5, 6, 7, 8];

    /// <summary>
    /// Plays bot games with every expansion and checks every pause the host could end: it has a rule, the rule
    /// behaves as the plan says, and what it waits for does not depend on hidden information.
    /// </summary>
    [TestMethod]
    public void EveryPauseHasAPublicRule()
    {
        Message.DefaultDescriber = DefaultSkin.Default;
        var seen = new Dictionary<Phase, int>();
        var waited = new Dictionary<Phase, int>();

        foreach (var seed in Seeds)
        {
            var game = CreateGame(seed);
            var bots = game.Players.ToDictionary(p => p.Faction,
                p => (IBot)new ClassicBot(game, p, BotParameters.GetDefaultParameters(p.Faction)));
            var random = new Random(seed);

            for (var eventNumber = 0; game.CurrentPhase != Phase.GameEnded && eventNumber < 3000; eventNumber++)
            {
                if (game.GetApplicableEvents(null, true).Contains(typeof(EndPhase)))
                {
                    var requirements = PhaseControl.Determine(game);
                    Assert.IsNotNull(requirements, $"No pause rule for {game.CurrentPhase} (seed {seed})");

                    CheckRow(game, requirements);
                    CheckHiddenInformationIsIgnored(game, requirements, random);

                    seen[game.CurrentPhase] = seen.GetValueOrDefault(game.CurrentPhase) + 1;
                    if (requirements.WaitFor.Count > 0)
                        waited[game.CurrentPhase] = waited.GetValueOrDefault(game.CurrentPhase) + 1;
                }

                var e = NextBotEvent(game, bots);
                Assert.IsNotNull(e, $"No bot could act in {game.CurrentPhase} (seed {seed})");
                e.Time = DateTimeOffset.UnixEpoch.AddSeconds(eventNumber);
                Assert.IsNull(e.Execute(true, true), $"Invalid {e.GetType().Name} in {game.CurrentPhase} (seed {seed})");
            }
        }

        Console.WriteLine(string.Join(Environment.NewLine,
            seen.OrderBy(kv => kv.Key).Select(kv => $"{kv.Key}: {kv.Value} pauses, {waited.GetValueOrDefault(kv.Key)} waited")));

        Phase[] expected =
        [
            Phase.MetheorAndStormSpell, Phase.StormReport, Phase.Thumper, Phase.BlowReport,
            Phase.BeginningOfCharity, Phase.ClaimingCharity, Phase.CharityReport,
            Phase.BeginningOfBidding, Phase.WaitingForNextBiddingRound, Phase.BiddingReport,
            Phase.BeginningOfResurrection, Phase.ResurrectionReport,
            Phase.BeginningOfShipAndMove, Phase.ShipmentAndMoveConcluded,
            Phase.BeginningOfBattle, Phase.BattleReport,
            Phase.BeginningOfCollection, Phase.CollectionReport,
            Phase.Contemplate, Phase.TurnConcluded
        ];

        foreach (var phase in expected)
            Assert.IsTrue(seen.ContainsKey(phase), $"The bot games never reached {phase}");
    }

    /// <summary>Old games keep host-only control.</summary>
    [TestMethod]
    public void OldVersionsAreNotControlled()
    {
        var game = new Game(PhaseControl.FirstVersion - 1, new Participation());
        Assert.IsNull(PhaseControl.Determine(game));
    }

    /// <summary>Spot checks of rows from the plan, phrased independently of the implementation.</summary>
    private static void CheckRow(Game g, PauseRequirements r)
    {
        var waitFor = r.WaitFor.Select(p => p.Faction).ToList();
        Assert.AreEqual(waitFor.Count, waitFor.Distinct().Count(), $"{g.CurrentPhase} waits for a player twice");

        var amal = g.Applicable(Rule.ExpansionTreacheryCardsAmal);
        var choam = g.GetPlayer(Faction.Choam);
        var choamMayDiscard = choam != null && choam.TreacheryCards.Count > 0 && !g.Prevented(FactionAdvantage.ChoamDiscarding);

        switch (g.CurrentPhase)
        {
            case Phase.StormReport or Phase.CharityReport or Phase.BiddingReport or Phase.ResurrectionReport or
                Phase.ShipmentAndMoveConcluded or Phase.CollectionReport or Phase.TurnConcluded or Phase.BeginningOfBidding or
                Phase.BeginningOfShipAndMove or Phase.BlowReport:
                Assert.AreEqual(choamMayDiscard, waitFor.Contains(Faction.Choam), $"CHOAM at {g.CurrentPhase}");
                break;

            case Phase.BattleReport when g.CurrentMoment == MainPhaseMoment.End:
                Assert.AreEqual(choamMayDiscard, waitFor.Contains(Faction.Choam), "CHOAM after the last battle");
                break;

            case Phase.BattleReport:
                Assert.AreEqual(0, waitFor.Count, "Between battles");
                Assert.IsFalse(r.Countdown, "Between battles");
                break;

            case Phase.BeginningOfBattle:
                Assert.AreEqual(0, waitFor.Count);
                Assert.AreEqual(amal, r.Countdown, "Beginning of Battle counts down only for Amal");
                break;

            case Phase.WaitingForNextBiddingRound:
                var atreides = g.IsPlaying(Faction.Atreides) && !g.Prevented(FactionAdvantage.AtreidesBiddingPrescience);
                CollectionAssert.AreEqual(atreides ? new[] { Faction.Atreides } : [], waitFor, "Between cards");
                Assert.IsFalse(r.Countdown);
                break;

            case Phase.HarvesterA or Phase.HarvesterB or Phase.CancellingTraitor or Phase.Extortion or Phase.ClaimingCharity:
                Assert.IsTrue(r.Countdown, $"{g.CurrentPhase} counts down");
                break;

            case Phase.MetheorAndStormSpell:
                Assert.AreEqual(g.CurrentTurn > 1, r.Countdown, "Storm Spell and Metheor from turn 2");
                break;
        }
    }

    /// <summary>
    /// Waiting for a player must not reveal anything, so the result may not change when hidden information does:
    /// which cards are in each hand (not how many), spice, Nexus cards and Moritani's unplaced terror tokens.
    /// </summary>
    private static void CheckHiddenInformationIsIgnored(Game g, PauseRequirements expected, Random random)
    {
        var allCards = TreacheryCardManager.GetCardsInAndOutsidePlay().ToList();
        var saved = g.Players.Select(p => (p, p.Resources, p.TreacheryCards, p.Nexus)).ToList();
        var terror = g.UnplacedTerrorTokens.ToList();

        try
        {
            for (var attempt = 0; attempt < 3; attempt++)
            {
                foreach (var p in g.Players)
                {
                    p.Resources = random.Next(0, 25);
                    p.TreacheryCards = p.TreacheryCards.Select(_ => allCards[random.Next(allCards.Count)]).ToList();
                    p.Nexus = random.Next(2) == 0 ? Faction.None : EstablishPlayers.AvailableFactions().ElementAt(random.Next(EstablishPlayers.AvailableFactions().Count()));
                }

                g.UnplacedTerrorTokens.Clear();
                g.UnplacedTerrorTokens.AddRange(Enum.GetValues<TerrorType>().Where(_ => random.Next(2) == 0));

                var actual = PhaseControl.Determine(g)!;
                CollectionAssert.AreEqual(expected.WaitFor.Select(p => p.Faction).ToList(), actual.WaitFor.Select(p => p.Faction).ToList(),
                    $"What {g.CurrentPhase} waits for depends on hidden information");
                Assert.AreEqual(expected.Countdown, actual.Countdown, $"Whether {g.CurrentPhase} counts down depends on hidden information");
            }
        }
        finally
        {
            foreach (var (p, resources, cards, nexus) in saved)
            {
                p.Resources = resources;
                p.TreacheryCards = cards;
                p.Nexus = nexus;
            }

            g.UnplacedTerrorTokens.Clear();
            g.UnplacedTerrorTokens.AddRange(terror);
        }
    }

    private static Game CreateGame(int seed)
    {
        var game = new Game();
        var rules = Game.RulesetDefinition[Ruleset.AllExpansionsAdvancedGame].ToList();
        rules.Add(Rule.FillWithBots);

        var establishPlayers = new EstablishPlayers(game, Faction.None)
        {
            Seed = seed,
            Settings = new GameSettings
            {
                InitialRules = rules,
                AllowedFactionsInPlay = EstablishPlayers.AvailableFactions().ToList(),
                MaximumTurns = 3,
                NumberOfPlayers = 6
            }
        };

        Assert.IsNull(establishPlayers.Execute(true, true));
        return game;
    }

    private static GameEvent? NextBotEvent(Game game, Dictionary<Faction, IBot> bots)
    {
        var events = game.Players.ToDictionary(p => p, p => game.GetApplicableEvents(p, true));

        Func<IBot, List<Type>, GameEvent?>[] priorities =
        [
            (b, e) => b.DetermineHighestPriorityInPhaseAction(e),
            (b, e) => b.DetermineHighPriorityInPhaseAction(e),
            (b, e) => b.DetermineMiddlePriorityInPhaseAction(e),
            (b, e) => b.DetermineLowPriorityInPhaseAction(e),
            (b, e) => b.DetermineEndPhaseAction(e)
        ];

        foreach (var priority in priorities)
            foreach (var p in game.Players)
                if (priority(bots[p.Faction], events[p]) is { } e)
                    return e;

        return null;
    }
}
