/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared;

public class EstablishPlayers : GameEvent
{
    #region Construction

    public EstablishPlayers(Game game, Faction initiator) : base(game, initiator)
    {
    }

    public EstablishPlayers()
    {
    }

    #endregion Construction

    #region Properties

    public GameSettings Settings { get; set; } = new();

    public int Seed { get; set; }

    [Obsolete]
    public int MaximumNumberOfPlayers { get; set; } = 6;

    [Obsolete]
    public int MaximumTurns { get; set; } = 10;

    [Obsolete] 
    public Rule[] ApplicableRules { get; set; } = [];

    public string _players = string.Empty;

    [JsonIgnore]
    public IEnumerable<string> Players
    {
        get => _players == "" ? [] : _players.Split('>');
        init => _players = string.Join('>', value);
    }

    [Obsolete]
    public string _factionsInPlay = "";

    [Obsolete]
    [JsonIgnore]
    public List<Faction> FactionsInPlay
    {
        get => string.IsNullOrEmpty(_factionsInPlay) ? [] : _factionsInPlay.Split(',').Select(Enum.Parse<Faction>).ToList();
        set => _factionsInPlay = string.Join(',', value);
    }

    #endregion Properties

    #region Validation

    public override Message? Validate()
    {
        if (Game.Version < 170)
            return null;
        
        if (Game.CurrentPhase != Phase.AwaitingPlayers) return Message.Express("Invalid game phase");
        if (Settings.NumberOfPlayers < 2) return Message.Express("At least two players required");
        if (Settings.NumberOfPlayers > Settings.AllowedFactionsInPlay.Count) return Message.Express("More factions required");
        if (Game.Participation.SeatedPlayers.Count > Settings.NumberOfPlayers) return Message.Express("More players than seats at the table");            
        if (Settings.AllowedFactionsInPlay.Any(f => !AvailableFactions().Contains(f))) return Message.Express("Invalid faction");

        return null;
    }

    public static int GetMaximumNumberOfPlayers()
    {
        return AvailableFactions().Count();
    }

    public static int GetMaximumNumberOfTurns()
    {
        return 20;
    }

    public static IEnumerable<Faction> AvailableFactions()
    {
        var result = new List<Faction>();

        if (Game.ExpansionLevel >= 0)
        {
            result.Add(Faction.Atreides);
            result.Add(Faction.Harkonnen);
            result.Add(Faction.Fremen);
            result.Add(Faction.Emperor);
            result.Add(Faction.Guild);
            result.Add(Faction.BeneGesserit);
        }

        if (Game.ExpansionLevel >= 1)
        {
            result.Add(Faction.Ixian);
            result.Add(Faction.Tleilaxu);
        }

        if (Game.ExpansionLevel >= 2)
        {
            result.Add(Faction.Choam);
            result.Add(Faction.Richese);
        }

        if (Game.ExpansionLevel >= 3)
        {
            result.Add(Faction.Ecaz);
            result.Add(Faction.Moritani);
        }

        return result;
    }

    public static IEnumerable<RuleGroup> AvailableRuleGroups()
    {
        var result = new List<RuleGroup>();

        if (Game.ExpansionLevel >= 0)
        {
            result.Add(RuleGroup.CoreAdvanced);
            result.Add(RuleGroup.CoreBasicExceptions);
            result.Add(RuleGroup.CoreAdvancedExceptions);
        }

        if (Game.ExpansionLevel >= 1)
        {
            result.Add(RuleGroup.ExpansionIxAndBtBasic);
            result.Add(RuleGroup.ExpansionIxAndBtAdvanced);
        }

        if (Game.ExpansionLevel >= 2)
        {
            result.Add(RuleGroup.ExpansionChoamAndRicheseBasic);
            result.Add(RuleGroup.ExpansionChoamAndRicheseAdvanced);
        }

        if (Game.ExpansionLevel >= 3)
        {
            result.Add(RuleGroup.ExpansionEcazAndMoritaniBasic);
            result.Add(RuleGroup.ExpansionEcazAndMoritaniAdvanced);
        }

        if (Game.ExpansionLevel >= 0) result.Add(RuleGroup.House);

        return result;
    }

    #endregion Validation

    #region Execution

    public override Message? Execute(bool performValidation, bool isHost)
    {
        if (performValidation)
        {
            var result = Validate();
            if (result == null)
            {
                Game.PerformPreEventTasks(this);
                ExecuteConcreteEvent();
                Game.PerformPostEventTasks(this, true);
            }
            return result;
        }

        Game.PerformPreEventTasks(this);
        ExecuteConcreteEvent();
        Game.PerformPostEventTasks(this, true);
        return null;
    }

    protected override void ExecuteConcreteEvent()
    {
        Game.StartReport(MainPhase.Setup);
        Log("Game started!");
        Game.Stone(Milestone.GameStarted);
        
        Game.CurrentMainPhase = MainPhase.Setup;
        Game.Seed = Seed;
        Game.Random = new LoggedRandom(Seed);
        
        //Settings

        if (Game.Version >= 170)
        {
            Game.Settings = Settings;
        }
        else
        {
            #pragma warning disable CS0612 // Type or member is obsolete
            Game.Settings.NumberOfPlayers = MaximumNumberOfPlayers;
            Game.Settings.MaximumTurns = MaximumTurns;
            Game.Settings.AllowedFactionsInPlay = FactionsInPlay;
            Game.Settings.InitialRules = ApplicableRules.ToList();
            #pragma warning restore CS0612 // Type or member is obsolete
        }

        //Rules
        
        #pragma warning disable CS0612 // Type or member is obsolete
        Game.AllRules = Game.Version < 170 ? ApplicableRules.ToList() : Game.Settings.InitialRules;
        #pragma warning restore CS0612 // Type or member is obsolete
        
        Game.Rules = Game.AllRules.Where(r => Game.GetRuleGroup(r) != RuleGroup.Bots).ToList();
        Game.RulesForBots = Game.AllRules.Where(r => Game.GetRuleGroup(r) == RuleGroup.Bots).ToList();
        Game.Rules.AddRange(Game.GetRulesInGroup(RuleGroup.CoreBasic, Game.ExpansionLevel));

        Game.Ruleset = Game.DetermineApproximateRuleset(Game.Settings.AllowedFactionsInPlay, Game.Rules, Game.ExpansionLevel);
        Log("Ruleset: ", Game.Ruleset);

        var customRules = Game.GetCustomRules().ToList();
        LogIf(customRules.Count != 0, "House rules: ", customRules);

        if (Game.Applicable(Rule.ExpansionTreacheryCards))
        {
            if (!Game.Rules.Contains(Rule.ExpansionTreacheryCardsExceptPBandSSandAmal)) Game.Rules.Add(Rule.ExpansionTreacheryCardsExceptPBandSSandAmal);
            if (!Game.Rules.Contains(Rule.ExpansionTreacheryCardsPBandSs)) Game.Rules.Add(Rule.ExpansionTreacheryCardsPBandSs);
            if (!Game.Rules.Contains(Rule.ExpansionTreacheryCardsAmal)) Game.Rules.Add(Rule.ExpansionTreacheryCardsAmal);
        }
        
        //Decks and card piles

        Game.ResourceCardDeck = CreateAndShuffleResourceCardDeck();
        Game.TreacheryDeck = TreacheryCardManager.CreateTreacheryDeck(Game, Game.Random);
        CreateDiscoveryTokens();

        if (!Game.Applicable(Rule.CustomDecks))
        {
            Game.TreacheryDeck.Shuffle();
            Game.Stone(Milestone.Shuffled);
        }

        Game.TreacheryDiscardPile = new Deck<TreacheryCard>(Game.Random);
        Game.ResourceCardDiscardPileA = new Deck<ResourceCard>(Game.Random);
        Game.ResourceCardDiscardPileB = new Deck<ResourceCard>(Game.Random);

        if (Game.Applicable(Rule.NexusCards)) 
            CreateNexusDeck();

        CreateTerrorTokens();
        
        Game.UnassignedAmbassadors = new Deck<Ambassador>(AvailableFactions().Where(f => f != Faction.Moritani).Select(Game.AmbassadorOf), Game.Random);

        //Initial alliance settings
        
        Game.GuildAllowsShippingDiscount = true;
        Game.TleilaxuAllowsRevivalDiscount = true;
        Game.IxianAllowsReplacingCards = true;
        Game.EmperorWillPayForExtraRevival = 0;
        Game.FremenWillProtectFromMonster = true;
        Game.FremenAllowsThreeFreeRevivals = true;
        Game.FremenSharesPrescience = true;
        Game.FremenRefundsBattleDial = true;
        Game.AtreidesSharesPrescience = true;
        Game.BeneGesseritAllowsUseOfVoice = true;
        Game.RicheseAllowsUseOfNoField = true;

        Game.FactionsInPlay = Game.Settings.AllowedFactionsInPlay;

        AddPlayersToGame();
        FillSeats();
        RemoveClaimedFactions();
        
        if (Game.Version >= 172)
            Game.DeterminePositionsAtTable();        
        
        Game.Enter(Game.Applicable(Rule.PlayersChooseFactions), Phase.SelectingFactions, Game.AssignFactionsAndEnterFactionTrade);
    }

    private void CreateTerrorTokens()
    {
        Game.UnplacedTerrorTokens =
        [
            TerrorType.Assassination,
            TerrorType.Atomics,
            TerrorType.Extortion,
            TerrorType.Robbery,
            TerrorType.Sabotage,
            TerrorType.SneakAttack
        ];
    }

    private void CreateNexusDeck()
    {
        Game.NexusCardDeck = new Deck<Faction>(AvailableFactions(), Game.Random!);
        Game.NexusCardDeck.Shuffle();
    }

    private void RemoveClaimedFactions()
    {
        foreach (var f in Game.Players.Where(p => !p.Is(Faction.None)).Select(p => p.Faction)) 
            Game.Settings.AllowedFactionsInPlay.Remove(f);
    }

    private Deck<ResourceCard> CreateAndShuffleResourceCardDeck()
    {
        var result = new Deck<ResourceCard>(Game.Random!);
        foreach (var c in Map.GetResourceCardsInPlay(Game)) result.PutOnTop(c);

        Game.Stone(Milestone.Shuffled);
        result.Shuffle();
        return result;
    }

    private void AddPlayersToGame()
    {
        if (Game.Version < 113)
        {
            AddBots();
        } 
        
        if (Game.Version < 170)
        {
            #pragma warning disable CS0612 // Type or member is obsolete
            foreach (var newPlayer in Players)
            {
                var p = new Player(Game);
                Game.LegacyNames.Add(p, newPlayer);
                Game.Players.Add(p);
                Log(p.Name, " joins the game");
            }
            #pragma warning restore CS0612 // Type or member is obsolete
        }
    }

    private void AddBots()
    {
        //Can be removed later, this was replaced by filling empty seats with bots.
        
        if (Game.Applicable(Rule.GuildBot)) Game.Players.Add(new Player(Game, Faction.Guild));
        if (Game.Applicable(Rule.EmperorBot)) Game.Players.Add(new Player(Game, Faction.Emperor));
        if (Game.Applicable(Rule.HarkonnenBot)) Game.Players.Add(new Player(Game, Faction.Harkonnen));
        if (Game.Applicable(Rule.TleilaxuBot)) Game.Players.Add(new Player(Game, Faction.Tleilaxu));
        if (Game.Applicable(Rule.BeneGesseritBot)) Game.Players.Add(new Player(Game, Faction.BeneGesserit));
        if (Game.Applicable(Rule.AtreidesBot)) Game.Players.Add(new Player(Game, Faction.Atreides));
        if (Game.Applicable(Rule.FremenBot)) Game.Players.Add(new Player(Game, Faction.Fremen));
        if (Game.Applicable(Rule.IxianBot)) Game.Players.Add(new Player(Game,  Faction.Ixian));
    }

    private void CreateDiscoveryTokens()
    {
        Game.FremenDiscoveryTokens = new Deck<DiscoveryToken>(Game.Random!);
        Game.GuildDiscoveryTokens = new Deck<DiscoveryToken>(Game.Random!);

        if (Game.Applicable(Rule.DiscoveryTokens))
        {
            Game.FremenDiscoveryTokens.Items.Add(DiscoveryToken.Jacurutu);
            Game.FremenDiscoveryTokens.Items.Add(DiscoveryToken.Shrine);
            Game.FremenDiscoveryTokens.Items.Add(DiscoveryToken.TestingStation);
            Game.FremenDiscoveryTokens.Items.Add(DiscoveryToken.Cistern);
            Game.FremenDiscoveryTokens.Shuffle();

            Game.GuildDiscoveryTokens.Items.Add(DiscoveryToken.ProcessingStation);
            Game.GuildDiscoveryTokens.Items.Add(DiscoveryToken.CardStash);
            Game.GuildDiscoveryTokens.Items.Add(DiscoveryToken.ResourceStash);
            Game.GuildDiscoveryTokens.Items.Add(DiscoveryToken.Flight);
            Game.GuildDiscoveryTokens.Shuffle();
        }
    }

    private void FillSeats()
    {
        if (Game.Version >= 170 || Game.Applicable(Rule.FillWithBots))
        {
            if (Game.Version > 125)
            {
                while (Game.Players.Count < Game.Settings.NumberOfPlayers)
                {
                    var player = new Player(Game);
                    Game.Players.Add(player);
                }
            }
            else
            {
                var available = new Deck<Faction>(Game.Settings.AllowedFactionsInPlay.Where(f => !Game.IsPlaying(f)), Game.Random!);
                available.Shuffle();

                while (Game.Players.Count < Game.Settings.NumberOfPlayers)
                {
                    var bot = available.Draw() switch
                    {
                        Faction.Harkonnen => new Player(Game, Faction.Harkonnen),
                        Faction.BeneGesserit => new Player(Game, Faction.BeneGesserit),
                        Faction.Atreides => new Player(Game, Faction.Atreides),
                        Faction.Fremen => new Player(Game, Faction.Fremen),
                        Faction.Emperor => new Player(Game, Faction.Emperor),
                        Faction.Guild => new Player(Game, Faction.Guild),
                        Faction.Ixian => new Player(Game, Faction.Ixian),
                        Faction.Tleilaxu => new Player(Game, Faction.Tleilaxu),
                        Faction.Choam => new Player(Game, Faction.Choam),
                        Faction.Richese => new Player(Game, Faction.Richese),
                        Faction.Ecaz => new Player(Game, Faction.Ecaz),
                        Faction.Moritani => new Player(Game, Faction.Moritani),
                        _ => new Player(Game, Faction.Harkonnen)
                    };

                    Game.Players.Add(bot);
                }
            }
        }
    }

    #endregion Execution
}