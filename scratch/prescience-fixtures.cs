#:project ../treachery-online/Shared/Treachery.Shared.csproj
#:project ../treachery-online/Bots/Treachery.Bots/Treachery.Bots.csproj
#:property JsonSerializerIsReflectionEnabledByDefault=true
#:property NoWarn=IL2026;IL3050

// Builds the prescience screenshot fixtures from base fixtures by appending battle events.
// Run from the repo root: DOTNET_ROOT=~/dotnet10 ~/dotnet10/dotnet run scratch/prescience-fixtures.cs

using Treachery.Shared;
using Treachery.Bots;
using Treachery.Shared.Model;

Message.DefaultDescriber = DefaultSkin.Default;
var fixtures = Path.Combine(Path.GetDirectoryName(ScriptPath())!, "..", "scripts", "fixtures");

// 1. Atreides asks for the Guild's dial; the Guild submits. Seen from Atreides.
{
    var g = Load("mc-prescience");
    Do(g, new Prescience(g, Faction.Atreides) { Aspect = PrescienceAspect.Dial });
    Do(g, BotPlan(g, Faction.Guild));
    Save(g, "prescience-seen");

    // 2. The Guild withdraws that plan. Seen from the Guild, which may now resubmit.
    Do(g, new BattleRevision(g, Faction.Guild));
    Save(g, "prescience-revised");
}

// 3. Atreides uses Karma to see the Bene Gesserit's whole plan; the Bene Gesserit submit. Seen from a third party.
{
    var g = Load("karma-prescience-base");
    Do(g, new KarmaPrescience(g, Faction.Atreides));
    var opponent = g.CurrentBattle!.OpponentOf(Faction.Atreides)!.Faction;
    Do(g, BotPlan(g, opponent));
    Save(g, "karma-prescience");
}

// 4. The Emperor plays the Atreides Nexus card as a secret ally for the Guild's dial; the Guild submits. Seen from the Emperor.
{
    var g = Load("nexus-prescience-base");
    var user = g.Players.First(p => p.Nexus == Faction.Atreides);
    Do(g, new NexusPlayed(g, user.Faction) { Faction = Faction.Atreides, AtreidesPrescienceAspect = PrescienceAspect.Dial });
    Do(g, BotPlan(g, g.CurrentBattle!.OpponentOf(user)!.Faction));
    Save(g, "nexus-prescience");
}

Game Load(string name)
{
    var state = GameState.Load(File.ReadAllText(Path.Combine(fixtures, name + ".json")));
    var game = new Game();
    foreach (var evt in state.Events)
    {
        evt.Initialize(game);
        var error = evt.Execute(false, true);
        if (error != null) throw new Exception($"replaying {name}: {evt.GetType().Name}: {error.ToString(DefaultSkin.Default)}");
    }
    return game;
}

void Do(Game g, GameEvent evt)
{
    evt.Time = new DateTimeOffset(2026, 1, 1, 0, 0, 0, TimeSpan.Zero).AddSeconds(g.History.Count);
    var error = evt.Execute(true, false);
    if (error != null) throw new Exception($"{evt.GetType().Name} by {evt.Initiator}: {error.ToString(DefaultSkin.Default)}");
    Console.WriteLine($"  {evt.GetMessage().ToString(DefaultSkin.Default)}");
}

// The plan the bot for this faction would submit now.
Battle BotPlan(Game g, Faction f)
{
    var p = g.GetPlayer(f)!;
    var bot = new ClassicBot(g, p, BotParameters.GetDefaultParameters(f), new LoggedRandom(1));
    var events = new List<Type> { typeof(Battle) };
    foreach (var pick in new Func<GameEvent?>[]
             {
                 () => bot.DetermineHighestPriorityInPhaseAction(events),
                 () => bot.DetermineHighPriorityInPhaseAction(events),
                 () => bot.DetermineMiddlePriorityInPhaseAction(events),
                 () => bot.DetermineLowPriorityInPhaseAction(events),
             })
        if (pick() is Battle plan) return plan;
    throw new Exception($"the {f} bot has no battle plan");
}

void Save(Game g, string name)
{
    File.WriteAllText(Path.Combine(fixtures, name + ".json"), GameState.GetStateAsString(g));
    var plan = g.CurrentBattle!;
    Console.WriteLine($"wrote {name}: battle {plan.Aggressor} vs {plan.Defender}, aggressor plan {(g.AggressorPlan != null)}, defender plan {(g.DefenderPlan != null)}");
}

static string ScriptPath([System.Runtime.CompilerServices.CallerFilePath] string path = "") => path;
