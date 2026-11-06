using Treachery.Bots;
using Treachery.Shared;

namespace Treachery.Client.Test;

// Plays a bot game forward until a condition holds, for tests that need a real game state.
internal sealed class BotDriver
{
    public Game Game { get; } = BotGeneratedComponentSmokeTests.CreateGame();

    private readonly Dictionary<Faction, ClassicBot> _bots;

    private int _events;

    public BotDriver()
    {
        _bots = Game.Players.ToDictionary(
            player => player.Faction,
            player => new ClassicBot(Game, player, BotParameters.GetDefaultParameters(player.Faction)));
    }

    public void PlayUntil(Func<Game, bool> done)
    {
        while (!done(Game))
        {
            Assert.AreNotEqual(Phase.GameEnded, Game.CurrentPhase);
            var gameEvent = BotGeneratedComponentSmokeTests.DetermineBotEvent(Game, _bots);
            Assert.IsNotNull(gameEvent, $"No bot could act in phase {Game.CurrentPhase}.");
            gameEvent.Time = DateTimeOffset.UnixEpoch.AddSeconds(_events++);
            Assert.IsNull(gameEvent.Execute(true, true));
        }
    }
}
