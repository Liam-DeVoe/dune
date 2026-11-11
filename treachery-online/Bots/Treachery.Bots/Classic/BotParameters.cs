/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Bots;

public class BotParameters
{
    public const float PenaltyForAttackingBots = 6.0f;

    public int Bidding_ResourcesToKeepWhenCardIsPerfect { get; set; }
    public int Bidding_ResourcesToKeepWhenCardIsntPerfect { get; set; }
    public int Bidding_PassingTreshold { get; set; }

    public bool Karma_SaveCardToUseSpecialKarmaAbility { get; set; }

    public int Shipment_MinimumOtherPlayersITrustToPreventAWin { get; set; }
    public float Shipment_DialShortageToAccept { get; set; }
    public int Shipment_MinimumResourcesToKeepForBattle { get; set; }
    public float Shipment_MaxEnemyForceStrengthFightingForSpice { get; set; }
    public int Shipment_ExpectedStormMovesWhenUnknown { get; set; }
    public int Shipment_DialForExtraForcesToShip { get; set; }

    public int Battle_MaximumUnsupportedForces { get; set; }
    public float Battle_MimimumChanceToAssumeEnemyHeroSurvives { get; set; }
    public float Battle_MimimumChanceToAssumeMyLeaderSurvives { get; set; }
    public float Battle_DialShortageThresholdForThrowing { get; set; }

    public static readonly BotParameters HarkonnenParameters = new()
    {
        Bidding_ResourcesToKeepWhenCardIsPerfect = 1,
        Bidding_ResourcesToKeepWhenCardIsntPerfect = 4,
        Bidding_PassingTreshold = 0,
        Karma_SaveCardToUseSpecialKarmaAbility = true,
        Shipment_MinimumOtherPlayersITrustToPreventAWin = 4,
        Shipment_DialShortageToAccept = 6,
        Shipment_MinimumResourcesToKeepForBattle = 1,
        Shipment_DialForExtraForcesToShip = 1,
        Shipment_MaxEnemyForceStrengthFightingForSpice = 4,
        Shipment_ExpectedStormMovesWhenUnknown = 2,
        Battle_MaximumUnsupportedForces = 5,
        Battle_MimimumChanceToAssumeEnemyHeroSurvives = 0.6f,
        Battle_MimimumChanceToAssumeMyLeaderSurvives = 0.4f,
        Battle_DialShortageThresholdForThrowing = 3
    };

    public static readonly BotParameters AtreidesParameters = new()
    {
        Bidding_ResourcesToKeepWhenCardIsPerfect = 1,
        Bidding_ResourcesToKeepWhenCardIsntPerfect = 4,
        Bidding_PassingTreshold = 2,
        Karma_SaveCardToUseSpecialKarmaAbility = false,
        Shipment_MinimumOtherPlayersITrustToPreventAWin = 3,
        Shipment_DialShortageToAccept = 2,
        Shipment_MinimumResourcesToKeepForBattle = 1,
        Shipment_DialForExtraForcesToShip = 1,
        Shipment_MaxEnemyForceStrengthFightingForSpice = 2,
        Shipment_ExpectedStormMovesWhenUnknown = 3,
        Battle_MaximumUnsupportedForces = 3,
        Battle_MimimumChanceToAssumeEnemyHeroSurvives = 0.5f,
        Battle_MimimumChanceToAssumeMyLeaderSurvives = 0.5f,
        Battle_DialShortageThresholdForThrowing = 3
    };

    public static readonly BotParameters FremenParameters = new()
    {
        Bidding_ResourcesToKeepWhenCardIsPerfect = 0,
        Bidding_ResourcesToKeepWhenCardIsntPerfect = 0,
        Bidding_PassingTreshold = 4,
        Karma_SaveCardToUseSpecialKarmaAbility = false,
        Shipment_MinimumOtherPlayersITrustToPreventAWin = 2,
        Shipment_DialShortageToAccept = 4,
        Shipment_MinimumResourcesToKeepForBattle = 0,
        Shipment_DialForExtraForcesToShip = 6,
        Shipment_MaxEnemyForceStrengthFightingForSpice = 8,
        Shipment_ExpectedStormMovesWhenUnknown = 0,
        Battle_MaximumUnsupportedForces = 20,
        Battle_MimimumChanceToAssumeEnemyHeroSurvives = 0.4f,
        Battle_MimimumChanceToAssumeMyLeaderSurvives = 0.5f,
        Battle_DialShortageThresholdForThrowing = 4
    };

    public static readonly BotParameters EmperorParameters = new()
    {
        Bidding_ResourcesToKeepWhenCardIsPerfect = 1,
        Bidding_ResourcesToKeepWhenCardIsntPerfect = 6,
        Bidding_PassingTreshold = 3,
        Karma_SaveCardToUseSpecialKarmaAbility = true,
        Shipment_MinimumOtherPlayersITrustToPreventAWin = 4,
        Shipment_DialShortageToAccept = 3,
        Shipment_MinimumResourcesToKeepForBattle = 2,
        Shipment_DialForExtraForcesToShip = 2,
        Shipment_MaxEnemyForceStrengthFightingForSpice = 0,
        Shipment_ExpectedStormMovesWhenUnknown = 6,
        Battle_MaximumUnsupportedForces = 3,
        Battle_MimimumChanceToAssumeEnemyHeroSurvives = 0.4f,
        Battle_MimimumChanceToAssumeMyLeaderSurvives = 0.6f,
        Battle_DialShortageThresholdForThrowing = 3
    };

    public static readonly BotParameters GuildParameters = new()
    {
        Bidding_ResourcesToKeepWhenCardIsPerfect = 1,
        Bidding_ResourcesToKeepWhenCardIsntPerfect = 8,
        Bidding_PassingTreshold = 0,
        Karma_SaveCardToUseSpecialKarmaAbility = true,
        Shipment_MinimumOtherPlayersITrustToPreventAWin = 2,
        Shipment_DialShortageToAccept = 0,
        Shipment_MinimumResourcesToKeepForBattle = 4,
        Shipment_DialForExtraForcesToShip = 2,
        Shipment_MaxEnemyForceStrengthFightingForSpice = 0,
        Shipment_ExpectedStormMovesWhenUnknown = 6,
        Battle_MaximumUnsupportedForces = 0,
        Battle_MimimumChanceToAssumeEnemyHeroSurvives = 0.4f,
        Battle_MimimumChanceToAssumeMyLeaderSurvives = 0.6f,
        Battle_DialShortageThresholdForThrowing = 6
    };

    public static readonly BotParameters BeneGesseritParameters = new()
    {
        Bidding_ResourcesToKeepWhenCardIsPerfect = 1,
        Bidding_ResourcesToKeepWhenCardIsntPerfect = 7,
        Bidding_PassingTreshold = 0,
        Karma_SaveCardToUseSpecialKarmaAbility = false,
        Shipment_MinimumOtherPlayersITrustToPreventAWin = 2,
        Shipment_DialShortageToAccept = 4,
        Shipment_MinimumResourcesToKeepForBattle = 2,
        Shipment_DialForExtraForcesToShip = 2,
        Shipment_MaxEnemyForceStrengthFightingForSpice = 4,
        Shipment_ExpectedStormMovesWhenUnknown = 6,
        Battle_MaximumUnsupportedForces = 1,
        Battle_MimimumChanceToAssumeEnemyHeroSurvives = 0.4f,
        Battle_MimimumChanceToAssumeMyLeaderSurvives = 0.55f,
        Battle_DialShortageThresholdForThrowing = 6
    };

    public static readonly BotParameters IxianParameters = new()
    {
        Bidding_ResourcesToKeepWhenCardIsPerfect = 1,
        Bidding_ResourcesToKeepWhenCardIsntPerfect = 5,
        Bidding_PassingTreshold = 4,
        Karma_SaveCardToUseSpecialKarmaAbility = false,
        Shipment_MinimumOtherPlayersITrustToPreventAWin = 3,
        Shipment_DialShortageToAccept = 4,
        Shipment_MinimumResourcesToKeepForBattle = 0,
        Shipment_DialForExtraForcesToShip = 1,
        Shipment_MaxEnemyForceStrengthFightingForSpice = 4,
        Shipment_ExpectedStormMovesWhenUnknown = 5,
        Battle_MaximumUnsupportedForces = 2,
        Battle_MimimumChanceToAssumeEnemyHeroSurvives = 0.6f,
        Battle_MimimumChanceToAssumeMyLeaderSurvives = 0.4f,
        Battle_DialShortageThresholdForThrowing = 4
    };

    public static readonly BotParameters TleilaxuParameters = new()
    {
        Bidding_ResourcesToKeepWhenCardIsPerfect = 1,
        Bidding_ResourcesToKeepWhenCardIsntPerfect = 5,
        Bidding_PassingTreshold = 3,
        Karma_SaveCardToUseSpecialKarmaAbility = false,
        Shipment_MinimumOtherPlayersITrustToPreventAWin = 2,
        Shipment_DialShortageToAccept = 8,
        Shipment_MinimumResourcesToKeepForBattle = 0,
        Shipment_DialForExtraForcesToShip = 3,
        Shipment_MaxEnemyForceStrengthFightingForSpice = 6,
        Shipment_ExpectedStormMovesWhenUnknown = 0,
        Battle_MaximumUnsupportedForces = 8,
        Battle_MimimumChanceToAssumeEnemyHeroSurvives = 0.3f,
        Battle_MimimumChanceToAssumeMyLeaderSurvives = 0.6f,
        Battle_DialShortageThresholdForThrowing = 6
    };

    private static readonly BotParameters ChoamParameters = new()
    {
        Bidding_ResourcesToKeepWhenCardIsPerfect = 1,
        Bidding_ResourcesToKeepWhenCardIsntPerfect = 8,
        Bidding_PassingTreshold = 3,
        Karma_SaveCardToUseSpecialKarmaAbility = true,
        Shipment_MinimumOtherPlayersITrustToPreventAWin = 2,
        Shipment_DialShortageToAccept = 4,
        Shipment_MinimumResourcesToKeepForBattle = 0,
        Shipment_DialForExtraForcesToShip = 2,
        Shipment_MaxEnemyForceStrengthFightingForSpice = 0,
        Shipment_ExpectedStormMovesWhenUnknown = 4,
        Battle_MaximumUnsupportedForces = 1,
        Battle_MimimumChanceToAssumeEnemyHeroSurvives = 0.4f,
        Battle_MimimumChanceToAssumeMyLeaderSurvives = 0.55f,
        Battle_DialShortageThresholdForThrowing = 3
    };

    private static readonly BotParameters RicheseParameters = new()
    {
        Bidding_ResourcesToKeepWhenCardIsPerfect = 1,
        Bidding_ResourcesToKeepWhenCardIsntPerfect = 5,
        Bidding_PassingTreshold = 4,
        Karma_SaveCardToUseSpecialKarmaAbility = false,
        Shipment_MinimumOtherPlayersITrustToPreventAWin = 4,
        Shipment_DialShortageToAccept = 4,
        Shipment_MinimumResourcesToKeepForBattle = 0,
        Shipment_DialForExtraForcesToShip = 1,
        Shipment_MaxEnemyForceStrengthFightingForSpice = 4,
        Shipment_ExpectedStormMovesWhenUnknown = 4,
        Battle_MaximumUnsupportedForces = 1,
        Battle_MimimumChanceToAssumeEnemyHeroSurvives = 0.4f,
        Battle_MimimumChanceToAssumeMyLeaderSurvives = 0.5f,
        Battle_DialShortageThresholdForThrowing = 6
    };

    public static readonly BotParameters EcazParameters = new()
    {
        Bidding_ResourcesToKeepWhenCardIsPerfect = 1,
        Bidding_ResourcesToKeepWhenCardIsntPerfect = 5,
        Bidding_PassingTreshold = 4,
        Karma_SaveCardToUseSpecialKarmaAbility = false,
        Shipment_MinimumOtherPlayersITrustToPreventAWin = 2,
        Shipment_DialShortageToAccept = 2,
        Shipment_MinimumResourcesToKeepForBattle = 0,
        Shipment_DialForExtraForcesToShip = 2,
        Shipment_MaxEnemyForceStrengthFightingForSpice = 0,
        Shipment_ExpectedStormMovesWhenUnknown = 4,
        Battle_MaximumUnsupportedForces = 2,
        Battle_MimimumChanceToAssumeEnemyHeroSurvives = 0.5f,
        Battle_MimimumChanceToAssumeMyLeaderSurvives = 0.5f,
        Battle_DialShortageThresholdForThrowing = 3
    };

    public static readonly BotParameters MoritaniParameters = new()
    {
        Bidding_ResourcesToKeepWhenCardIsPerfect = 1,
        Bidding_ResourcesToKeepWhenCardIsntPerfect = 5,
        Bidding_PassingTreshold = 4,
        Karma_SaveCardToUseSpecialKarmaAbility = false,
        Shipment_MinimumOtherPlayersITrustToPreventAWin = 4,
        Shipment_DialShortageToAccept = 4,
        Shipment_MinimumResourcesToKeepForBattle = 0,
        Shipment_DialForExtraForcesToShip = 2,
        Shipment_MaxEnemyForceStrengthFightingForSpice = 5,
        Shipment_ExpectedStormMovesWhenUnknown = 4,
        Battle_MaximumUnsupportedForces = 2,
        Battle_MimimumChanceToAssumeEnemyHeroSurvives = 0.4f,
        Battle_MimimumChanceToAssumeMyLeaderSurvives = 0.5f,
        Battle_DialShortageThresholdForThrowing = 4
    };

    public static BotParameters GetDefaultParameters(Faction f)
    {
        var result = f switch
        {
            Faction.Harkonnen => HarkonnenParameters.MemberwiseClone(),
            Faction.BeneGesserit => BeneGesseritParameters.MemberwiseClone(),
            Faction.Atreides => AtreidesParameters.MemberwiseClone(),
            Faction.Fremen => FremenParameters.MemberwiseClone(),
            Faction.Emperor => EmperorParameters.MemberwiseClone(),
            Faction.Guild => GuildParameters.MemberwiseClone(),
            Faction.Ixian => IxianParameters.MemberwiseClone(),
            Faction.Tleilaxu => TleilaxuParameters.MemberwiseClone(),
            Faction.Choam => ChoamParameters.MemberwiseClone(),
            Faction.Richese => RicheseParameters.MemberwiseClone(),
            Faction.Ecaz => EcazParameters.MemberwiseClone(),
            Faction.Moritani => MoritaniParameters.MemberwiseClone(),
            _ => HarkonnenParameters.MemberwiseClone()
        };

        return (BotParameters)result;
    }
}