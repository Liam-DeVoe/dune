/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

using System;
using System.Runtime.Intrinsics.X86;

namespace Treachery.Shared;

public static class Enumerations
{
    public static IEnumerable<T> GetValues<T>() where T : struct, Enum => 
        Enum.GetValues<T>();

    public static IEnumerable<T> GetValuesExceptDefault<T>(T defaultValue) where T : struct, Enum =>
        GetValues<T>().Where(v => !v.Equals(defaultValue));
}

public enum Faction
{
    None = 0,
    Fremen = 10,
    Atreides = 20,
    Harkonnen = 30,
    Emperor = 40,
    Guild = 50,
    BeneGesserit = 60,
    Ixian = 70,
    Tleilaxu = 80,
    Choam = 90,
    Richese = 100,
    Ecaz = 110,
    Moritani = 120
}

public static class EnumerationExtensions
{
    public static Nexus ToNexus(this Faction faction) => faction switch
    {
        Faction.None => Nexus.None,
        Faction.Fremen => Nexus.Fremen,
        Faction.Atreides => Nexus.Atreides,
        Faction.Harkonnen => Nexus.Harkonnen,
        Faction.Emperor => Nexus.Emperor,
        Faction.Guild => Nexus.Guild,
        Faction.BeneGesserit => Nexus.BeneGesserit,
        Faction.Ixian => Nexus.Ixian,
        Faction.Tleilaxu => Nexus.Tleilaxu,
        Faction.Choam => Nexus.Choam,
        Faction.Richese => Nexus.Richese,
        Faction.Ecaz => Nexus.Ecaz,
        Faction.Moritani => Nexus.Moritani,
        _ => throw new ArgumentOutOfRangeException(nameof(faction), faction, null)
    };
    
    public static Faction ToFaction(this Nexus nexus) => nexus switch
    {
        Nexus.None => Faction.None,
        Nexus.Fremen => Faction.Fremen,
        Nexus.Atreides => Faction.Atreides,
        Nexus.Harkonnen => Faction.Harkonnen,
        Nexus.Emperor => Faction.Emperor,
        Nexus.Guild => Faction.Guild,
        Nexus.BeneGesserit => Faction.BeneGesserit,
        Nexus.Ixian => Faction.Ixian,
        Nexus.Tleilaxu => Faction.Tleilaxu,
        Nexus.Choam => Faction.Choam,
        Nexus.Richese => Faction.Richese,
        Nexus.Ecaz => Faction.Ecaz,
        Nexus.Moritani => Faction.Moritani,
        _ => throw new ArgumentOutOfRangeException(nameof(nexus), nexus, null)
    };
} 

public enum Nexus
{
    None = 0,
    Fremen = 10,
    Atreides = 20,
    Harkonnen = 30,
    Emperor = 40,
    Guild = 50,
    BeneGesserit = 60,
    Ixian = 70,
    Tleilaxu = 80,
    Choam = 90,
    Richese = 100,
    Ecaz = 110,
    Moritani = 120
}

public enum Ambassador
{
    None = 0,
    Fremen = 10,
    Atreides = 20,
    Harkonnen = 30,
    Emperor = 40,
    Guild = 50,
    BeneGesserit = 60,
    Ixian = 70,
    Tleilaxu = 80,
    Choam = 90,
    Richese = 100,
    Ecaz = 110,
    Moritani = 120
}

public enum TechToken
{
    None = 0,
    Graveyard = 10,
    Ships = 20,
    Resources = 30
}

public enum MainPhase
{
    None = 0,
    Started = 10,
    Setup = 20,
    Storm = 30,
    Blow = 40,
    Charity = 50,
    Bidding = 60,
    Resurrection = 70,
    ShipmentAndMove = 80,
    Battle = 90,
    Collection = 95,
    Contemplate = 100,
    Ended = 110
}

public enum MainPhaseMoment
{
    None = 0,
    Start = 10,
    Middle = 20,
    End = 30
}

public enum ChoamEconomicsStatus
{
    None = 0,
    Double = 10,
    DoubleFlipped = 20,
    Cancel = 30,
    CancelFlipped = 40,
    RemovedFromGame = 50
}

public enum Phase
{
    None = 00000,
    AwaitingPlayers = 10000,

    SelectingFactions = 29000,
    TradingFactions = 30000,
    CustomizingDecks = 31000,

    BeneGesseritPredicting = 50000,

    HarkonnenMulligan = 60000,

    AssigningInitialSkills = 64000,
    SelectingTraitors = 65000,
    PerformCustomSetup = 66000,

    FremenSettingUp = 70000,

    BeneGesseritSettingUp = 90000,

    MoritaniSettingUp = 91000,

    BeginningOfStorm = 93000,

    MetheorAndStormSpell = 95000,

    HmsPlacement = 100000,
    HmsMovement = 101000,

    DiallingStorm = 105000,
    StormLosses = 110000,
    StormReport = 113000,

    Thumper = 115000,
    BlowA = 120000,
    HarvesterA = 125000,
    BlowB = 130000,
    HarvesterB = 135000,

    FremenSendingMonsterA = 140000,

    VoteAllianceA = 149900,
    AllianceA = 150000,
    FremenRidingMonsterA = 160000,
    FremenSendingMonsterB = 170000,
    VoteAllianceB = 179900,
    AllianceB = 180000,

    FremenRidingMonsterB = 190000,
    NexusCards = 191000,

    BlowReport = 194000,

    BeginningOfCharity = 194500,
    ClaimingCharity = 195000,
    CharityReport = 195100,

    BeginningOfBidding = 195300,
    BlackMarketAnnouncement = 195500,
    BlackMarketBidding = 195600,
    RicheseAnnouncingAuction = 195700,
    RicheseSpecifyingAuction = 195800,
    RicheseKeepingUnsoldCard = 195900,

    IxianSelectingCard = 196000,
    IxianRemovingCardFromBid = 197000,
    IxianSwappingCard = 198000,

    Bidding = 200000,
    ReplacingCardJustWon = 201000,
    WaitingForNextBiddingRound = 205000,
    BiddingReport = 208000,

    BeginningOfResurrection = 209000,
    Resurrection = 210000,
    BeneGesseritIntrudedByRevival = 211000,
    TerrorTriggeredByRevival = 211100,
    AmbassadorTriggeredByRevival = 211110,
    ResurrectionReport = 215000,

    BeginningOfShipAndMove = 219000,

    NonGuildShip = 220000,

    GuildShip = 230000,

    BeneGesseritAccompaniesNonGuildShip = 240000,
    TerrorTriggeredByBeneGesseritAccompaniesNonGuildShip = 241000,
    AmbassadorTriggeredByBeneGesseritAccompaniesNonGuildShip = 241010,
    BeneGesseritAccompaniesGuildShip = 250000,
    TerrorTriggeredByBeneGesseritAccompaniesGuildShip = 251000,
    AmbassadorTriggeredByBeneGesseritAccompaniesGuildShip = 251010,
    BeneGesseritIntrudedByNonGuildShip = 255000,
    TerrorTriggeredByGuildShip = 255100,
    AmbassadorTriggeredByGuildShip = 255110,
    BeneGesseritIntrudedByGuildShip = 256000,
    TerrorTriggeredByNonGuildShip = 256100,
    AmbassadorTriggeredByNonGuildShip = 256110,

    NonGuildMove = 260000,

    GuildMove = 270000,

    BeneGesseritIntrudedByNonGuildMove = 280000,
    TerrorTriggeredByNonGuildMove = 281000,
    AmbassadorTriggeredByNonGuildMove = 281010,
    BeneGesseritIntrudedByGuildMove = 290000,
    TerrorTriggeredByGuildMove = 291000,
    AmbassadorTriggeredByGuildMove = 291010,
    BeneGesseritIntrudedByCaravan = 295000,
    TerrorTriggeredByCaravan = 295100,
    AmbassadorTriggeredByCaravan = 295110,
    BeneGesseritIntrudedByFremenRidingMonsterA = 296000,
    TerrorTriggeredByFremenRidingMonsterA = 296100,
    AmbassadorTriggeredByFremenRidingMonsterA = 296110,
    BeneGesseritIntrudedByFremenRidingMonsterB = 297000,
    TerrorTriggeredByFremenRidingMonsterB = 297100,
    AmbassadorTriggeredByFremenRidingMonsterB = 297110,

    AllianceByTerror = 298000,
    AllianceByAmbassador = 298010,

    ShipmentAndMoveConcluded = 299000,

    BeginningOfBattle = 299900,
    BattlePhase = 300000,
    ClaimingBattle = 305000,
    CallTraitorOrPass = 310000,
    CancellingTraitor = 310100,
    CaptureDecision = 311000,
    AvoidingAudit = 312000,

    Auditing = 313000,
    Retreating = 314000,

    BattleConclusion = 315000,

    RevealingFacedancer = 319000,
    Facedancing = 320000,

    BattleReport = 330000,

    BeginningOfCollection = 339000,
    DividingCollectedResources = 339100,
    AcceptingResourceDivision = 339150,
    CollectionReport = 340000,

    PerformingKarmaHandSwap = 350000,

    TradingCards = 351000,

    Clairvoyance = 360000,

    SearchingDiscarded = 370000,

    ReplacingFaceDancer = 395000,

    Extortion = 397000,
    Contemplate = 398000,
    TurnConcluded = 399000,
    GameEnded = 400000,

    Bureaucracy = 508000,
    AssigningSkill = 509000,
    Thought = 510000,
    MeltingRock = 511000,
    Discarding = 600000,
    DiscardingTraitor = 700000
}

public enum Milestone
{
    None = 0,
    GameStarted = 50,
    Monster = 100,
    GreatMonster = 110,
    BabyMonster = 150,
    Resource = 200,
    MetheorUsed = 300,
    AuctionWon = 500,
    Shipment = 550,
    LeaderKilled = 600,
    Messiah = 650,
    Explosion = 700,
    GameWon = 800,
    CharityClaimed = 850,
    CardOnBidSwapped = 890,
    Bid = 900,
    CardWonSwapped = 910,
    Shuffled = 1100,
    Revival = 1200,
    Move = 1300,
    TreacheryCalled = 1400,
    FaceDanced = 1450,
    Clairvoyance = 1500,
    Karma = 1600,
    Bribe = 1700,
    Amal = 1800,
    Thumper = 1900,
    Harvester = 2000,
    HmsMovement = 2100,
    RaiseDead = 2200,
    WeatherControlled = 2300,
    Storm = 2400,
    Voice = 2500,
    Prescience = 2600,
    ResourcesReceived = 2700,
    Economics = 2800,
    CardTraded = 2900,
    Discard = 3000,
    SpecialUselessPlayed = 3100,
    TerrorPlanted = 3200,
    TerrorRevealed = 3300,
    AmbassadorPlaced = 3400,
    AmbassadorActivated = 3500,
    NexusPlayed = 3600,
    DiscoveryAppeared = 3700,
    DiscoveryRevealed = 3800,
    Assassination = 3900,
    Bureaucracy = 4000,
    Audited = 4100
}

public enum FactionAdvantage
{
    None = 0,

    AtreidesBiddingPrescience = 20,
    AtreidesSpiceBlowPrescience = 30,
    AtreidesBattlePlanPrescience = 35,
    AtreidesUseMessiah = 38,

    HarkonnenFreeCard = 40,
    HarkonnenCaptureLeader = 50,
    HarkonnenCallTraitorForAlly = 51,

    BeneGesseritAccompanies = 60,
    BeneGesseritUsingVoice = 65,
    BeneGesseritWorthlessAsKarma = 70,
    BeneGesseritAnnouncesBattle = 71,
    BeneGesseritIntrusion = 72,
    BeneGesseritCharity = 73,

    FremenControlsMonster = 80,
    FremenRidesMonster = 81,
    FremenNotPayingForBattles = 85,
    FremenSpecialForceBonus = 90,
    FremenExtraMove = 95,
    FremenProtectedFromStorm = 96,
    FremenProtectedFromMonster = 97,
    FremenProtectedFromMonsterAlly = 98,
    FremenStormPrescience = 99,

    EmperorSpecialForceBonus = 100,
    EmperorReceiveBid = 105,
    EmperorGiveSpiceToAlly = 106,
    EmperorLetAllyReviveExtraForces = 107,

    GuildDetermineMoveMoment = 110,
    GuildSpecialShipments = 111,
    GuildShipmentsDiscount = 112,
    GuildShipmentsDiscountAlly = 113,
    GuildReceiveShipment = 114,

    TleilaxuRevivalDiscount = 120,
    TleilaxuRevivalDiscountAlly = 121,
    TleilaxuReplacingFaceDancer = 122,//*
    TleilaxuIncreasingRevivalLimits = 123,//*
    TleilaxuReceiveRevive = 124,
    TleilaxuEarlyLeaderRevive = 125, //*
    TleilaxuReviveGhola = 126, //*

    IxianMovingHms = 130,
    IxianSpecialForceBonus = 131,
    IxianSelectingCardsOnAuction = 132,
    IxianCyborgExtraMove = 133,
    IxianReplacingSpecialForces = 134, //*
    IxianAllyDiscardingCard = 135, //*
    IxianSwappingCard = 136, //*

    ChoamControllingCharity = 140,
    ChoamDiscarding = 141,
    ChoamRevival = 142,
    ChoamEconomics = 143,
    ChoamReceiveForcePayment = 144,
    ChoamAudit = 145,

    RicheseAuction = 150,
    RicheseNofield = 151,
    RicheseBlackMarket = 152,

    EcazAmbassadors = 160,
    EcazOccupation = 161,
    EcazCollection = 162,

    MoritaniPlantingTerror = 170,
    MoritaniGainingVidal = 171,
    MoritaniEnemyOfEnemy = 172
}

public enum FactionForce
{
    None = 0,
    Fremen = 10,
    Atreides = 20,
    Harkonnen = 30,
    Emperor = 40,
    Guild = 50,
    BeneGesserit = 60,
    Ixian = 70,
    Tleilaxu = 80,
    Choam = 90,
    Richese = 100,
    Ecaz = 110,
    Moritani = 120
}

public enum FactionSpecialForce
{
    None = 0,
    Fremen = 10,
    Emperor = 20,
    BeneGesserit = 30,
    Ixian = 40,
    Richese = 50
}

public enum Ruleset
{
    None = 0,
    BasicGame = 10,

    AdvancedGame = 30,

    ExpansionBasicGame = 110,
    ExpansionAdvancedGame = 130,

    ServerClassic = 140,

    Expansion2BasicGame = 150,
    Expansion2AdvancedGame = 170,

    Expansion3BasicGame = 300,
    Expansion3AdvancedGame = 310,

    AllExpansionsBasicGame = 180,
    AllExpansionsAdvancedGame = 200,

    Custom = 1000
}

public enum DiscoveryToken
{
    None = 0,
    Jacurutu = 10,
    Shrine = 20,
    TestingStation = 30,
    Cistern = 40,
    ProcessingStation = 50,
    CardStash = 60,
    ResourceStash = 70,
    Flight = 80
}

public enum DiscoveryTokenType
{
    None = 0,
    Fremen = 10,
    Guild = 20
}

public enum Rule
{
    None = 0,

    //Basic classic game
    BasicTreacheryCards = 10000,
    HasCharityPhase = 10001,





    //Advanced Game
    AdvancedCombat = 10,
    IncreasedResourceFlow = 15,
    AdvancedKarama = 20,

    FremenDeterminesStorm = 25,
    FremenStormLosses = 30,
    FremenSendingMonster = 35,
    FremenSpecialForces = 40,

    AtreidesMessiah = 45,

    HarkonnenCapturesOrKillsLeaders = 50,

    BeneGesseritFirstForceInAnyTerritory = 55, //needed for versions < 144
    BeneGesseritAutoCharity = 60,
    BeneGesseritWorthlessAsKarma = 65,
    BeneGesseritAdvisors = 70,
    BeneGesseritAccompaniesToShipmentLocation = 75,  //needed for versions < 144

    GuildDetermineShipment = 80,

    EmperorSpecialForces = 85,

    //Exceptions
    BribesAreImmediate = 90,
    ContestedStongholdsCountAsOccupied = 95,
    AdvisorsDontConflictWithAlly = 96,
    GuildShipmentContributionsFlowBack = 97,

    //Expansion
    TechTokens = 200,
    ExpansionTreacheryCards = 209,
    ExpansionTreacheryCardsExceptPBandSSandAmal = 210,
    ExpansionTreacheryCardsPBandSs = 211,
    ExpansionTreacheryCardsAmal = 212,
    CheapHeroTraitor = 220,
    SandTrout = 230,

    //Expansion, Advanced Game
    IxianSwappingCardOnBid = 300,
    TleilaxuGholas = 301,

    //Expansion 2
    LeaderSkills = 400,
    StrongholdBonus = 401,
    RicheseTreacheryCards = 402,
    Expansion2TreacheryCards = 403,

    //Expansion 2, Advanced Game
    ChoamAuditor = 500,
    RicheseBlackMarket = 501,

    //Expansion 3
    GreatMaker = 600,
    DiscoveryTokens = 601,
    Homeworlds = 602,
    NexusCards = 603,
    Expansion3TreacheryCards = 604,

    //Expansion 3, Advanced Game
    MoritaniAssassinate = 700,
    EcazLoyalty = 701,

    //Bots
    FillWithBots = 998,
    GuildBot = 1000,
    EmperorBot = 1001,
    HarkonnenBot = 1002,
    TleilaxuBot = 1003,
    BeneGesseritBot = 1004,
    AtreidesBot = 1005,
    FremenBot = 1006,
    IxianBot = 1007,
    BotsCannotAlly = 1008,
    //ChoamBot = 1009,
    //RicheseBot = 1013,

    //House Rules
    CustomInitialForcesAndResources = 100,
    CustomDecks = 101,
    HmSwithoutIxian = 104,
    Ssw = 105,
    HarkonnenMulligan = 106,
    FullPhaseKarma = 107,
    FremenMayMoveIntoStorm = 108,
    BeneGesseritVoiceMustNameSpecialCards = 109,
    BattlesUnderStorm = 110,
    MovementBonusRequiresOccupationBeforeMovement = 111,
    AssistedNotekeeping = 112,
    ResourceBonusForStrongholds = 113,
    DisableGuildSpecialVictory = 114,
    DisableResourceTransfers = 115,
    StormDeckWithoutFremen = 116,
    AssistedNotekeepingForAtreides = 117,
    FremenAllyGetsDialedResourcesRefunded = 118,
    DisableNovaFlipping = 119,
    BreakAlliancesOnAlliancePhase = 120,

    ExtraKaramaCards = 999,

    CardsCanBeTraded = 1010,
    PlayersChooseFactions = 1011,
    EmperorSupportingNonAllyBids = 1012,
    BattleWithoutLeader = 1013,
    CapturedLeadersAreTraitorsToOwnFaction = 1014,
    DisableEndOfGameReport = 1015

}

public enum RuleGroup
{
    None = 0,
    CoreBasic = 100,
    CoreBasicExceptions = 101,
    CoreAdvanced = 110,
    CoreAdvancedExceptions = 111,
    ExpansionIxAndBtBasic = 200,
    ExpansionIxAndBtBasicExceptions = 201,
    ExpansionIxAndBtAdvanced = 210,
    ExpansionIxAndBtAdvancedExceptions = 211,
    ExpansionChoamAndRicheseBasic = 300,
    ExpansionChoamAndRicheseBasicExceptions = 301,
    ExpansionChoamAndRicheseAdvanced = 310,
    ExpansionChoamAndRicheseAdvancedExceptions = 311,
    ExpansionEcazAndMoritaniBasic = 400,
    ExpansionEcazAndMoritaniBasicExceptions = 401,
    ExpansionEcazAndMoritaniAdvanced = 410,
    ExpansionEcazAndMoritaniAdvancedExceptions = 411,
    House = 1000,
    Bots = 2000
}

public enum TreacheryCardType
{
    None = 0,
    Laser = 10,

    ProjectileDefense = 19,
    Projectile = 20,
    WeirdingWay = 25,

    PoisonDefense = 29,
    Poison = 30,
    PoisonTooth = 31,
    Chemistry = 35,

    Shield = 40,
    Antidote = 50,

    ProjectileAndPoison = 55,
    ShieldAndAntidote = 56,

    Mercenary = 60,
    Karma = 70,
    Useless = 80,
    StormSpell = 90,
    RaiseDead = 100,
    Metheor = 110,
    Caravan = 120,
    Clairvoyance = 130,

    Amal = 140,
    ArtilleryStrike = 150,
    Thumper = 160,
    Harvester = 170,

    Distrans = 180,
    Juice = 190,
    MirrorWeapon = 200,
    PortableAntidote = 210,
    Flight = 220,
    SearchDiscarded = 230,
    TakeDiscarded = 240,
    Residual = 250,
    Rockmelter = 260,
    Recruits = 261,
    Reinforcements = 262,
    HarassAndWithdraw = 263
}

public enum Concept
{
    None = 0,
    Messiah = 10,
    Resource = 20,
    Monster = 30,
    Graveyard = 40,
    BabyMonster = 50,
    GreatMonster = 60
}

public enum WinMethod
{
    None = 0,
    Strongholds = 10,
    Prediction = 20,
    FremenSpecial = 30,
    GuildSpecial = 40,
    Forfeit = 50,
    Timeout = 60
}

public enum HeroType
{
    None = 0,
    Normal = 10,
    Messiah = 20,
    Mercenary = 30,
    Auditor = 40,
    Vidal = 50,
    VariableValue = 60
}

public enum AuctionType
{
    None = 0,
    Normal = 10,
    BlackMarketNormal = 20,
    BlackMarketOnceAround = 30,
    BlackMarketSilent = 40,
    RicheseOnceAround = 60,
    RicheseSilent = 70
}

public enum JuiceType
{
    None = 0,
    GoFirst = 10,
    GoLast = 20,
    Aggressor = 30
}

public enum LeaderSkill
{
    None = 0,
    Bureaucrat = 10,
    Diplomat = 20,
    Decipherer = 30,
    Smuggler = 40,
    Graduate = 50,
    Planetologist = 60,
    Warmaster = 70,
    Adept = 80,
    Swordmaster = 90,
    KillerMedic = 100,
    MasterOfAssassins = 110,
    Sandmaster = 120,
    Thinker = 130,
    Banker = 140
}

public enum StrongholdAdvantage
{
    None = 0,
    FreeResourcesForBattles = 10,
    CollectResourcesForUseless = 20,
    CountDefensesAsAntidote = 30,
    WinTies = 40,
    CollectResourcesForDial = 50,
    AnyOtherAdvantage = 60
}

public enum World
{
    None = 0,
    Fremen = 10,
    Atreides = 20,
    Harkonnen = 30,
    Emperor = 40,
    SalusaSecundus = 45,
    Guild = 50,
    BeneGesserit = 60,
    Ixian = 70,
    Tleilaxu = 80,
    Choam = 90,
    Richese = 100,
    Ecaz = 110,
    Moritani = 120
}


public enum CaptureDecision
{
    None = 0,
    DontCapture = 10,
    Capture = 20,
    Kill = 30
}

public enum DealType
{
    None = 0,
    DontShipOrMoveTo = 10,
    ShareBiddingPrescience = 30,
    ShareResourceDeckPrescience = 50,
    ShareStormPrescience = 60,
    ForfeitBattle = 70,
    TellDiscardedTraitors = 80
}

public enum TerrorType
{
    None = 0,
    Assassination = 10,
    Atomics = 30,
    Extortion = 50,
    Robbery = 60,
    Sabotage = 70,
    SneakAttack = 80
}

public enum IntrusionType
{
    None = 0,
    BeneGesseritIntrusion = 10,
    Terror = 20,
    Ambassador = 30
}

public enum VidalMoment
{
    None = 0,
    AfterUsedInBattle = 10,
    EndOfTurn = 20,
    WhileEcazWorldIsOccupied = 30,
    Never = 40
}

public enum ClairVoyanceAnswer
{
    None = 0,
    Yes = 10,
    No = 20,
    Unknown = 30
}

public enum ClairvoyanceQuestion
{
    None = 0,

    Prediction = 10,
    LeaderAsTraitor = 20,
    LeaderAsFacedancer = 30,
    HasCardTypeInHand = 40,

    LeaderInBattle = 100,
    CardTypeInBattle = 110,
    CardTypeAsDefenseInBattle = 111,
    CardTypeAsWeaponInBattle = 112,
    DialOfMoreThanXInBattle = 120,

    WillAttackX = 200
}

public enum PrescienceAspect
{
    None = 0,
    Dial = 10,
    Leader = 20,
    Weapon = 30,
    Defense = 40
}

[Flags]
public enum ShipmentPermission
{
    None = 0,
    Cross = 1,
    ToHomeworld = 2,
    GuildRate = 4
}

public enum UserStatus
{
    None = 0,
    Online = 10,
    Away = 20,
    Lfg = 30,
    Lfm = 40,
    InGame = 50,
}

public enum SubscriptionType
{
    None = 0,
    DontParticipate = 10,
    MaybeAsPlayer = 20,
    CertainAsPlayer = 30,
    MaybeAsHost = 40,
    CertainAsHost = 50
}

public enum GameListScope
{
    None = 1,
    Active = 2,
    All = 3
}

public enum ShipmentType
{
    Unknown = 0,
    ShipmentNormal = 1,
    ShipmentSiteToSite = 2,
    ShipmentBack = 3,
    ShipmentRicheseSecretAlly = 4
}

public enum AutomationRuleType
{
    Unknown = 0,
    
    CharityAutoClaim = 201,
    
    BiddingPassWhenAtreidesOrAtreidesAllyPassed = 301,
    BiddingPassAboveAmount = 302,
    BiddingPassWhenHighestBidByFaction = 303,

    RevivalAutoClaimFreeRevival = 401,
    
    ShipmentGuildAutoDelay = 502,
}

public enum ItemAction
{
    Create = 0,
    Update = 1,
    Delete = 2
}