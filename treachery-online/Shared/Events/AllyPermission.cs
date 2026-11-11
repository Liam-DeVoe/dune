/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */


namespace Treachery.Shared;

public class AllyPermission : GameEvent
{
    #region Construction

    public AllyPermission(Game game, Faction initiator) : base(game, initiator)
    {
    }

    public AllyPermission()
    {
    }

    #endregion Construction

    #region Properties

    //Needed for versions < 155
    public bool AllyMayShipAsGuild { get; set; }

    public bool GuildAllowsShippingDiscount
    {
        get => AllyMayShipAsGuild;
        set => AllyMayShipAsGuild = value;
    }

    public int EmperorWillPayForExtraRevival { get; set; }

    public bool FremenWillProtectFromMonster { get; set; }

    public bool FremenAllowsThreeFreeRevivals { get; set; }

    public bool FremenSharesPrescience { get; set; }

    public bool FremenRefundsBattleDial { get; set; }

    public bool TleilaxuAllowsRevivalDiscount
    {
        get => AllyMayReviveAsTleilaxu;
        set => AllyMayReviveAsTleilaxu = value;
    }

    //Needed for versions < 155
    public bool AllyMayReviveAsTleilaxu { get; set; }

    public bool IxianAllowsReplacingCards
    {
        get => AllyMayReplaceCards;
        set => AllyMayReplaceCards = value;
    }

    //Needed for versions < 155
    public bool AllyMayReplaceCards { get; set; }

    public bool AtreidesSharesPrescience { get; set; }

    public bool BeneGesseritAllowsUseOfVoice { get; set; }

    public bool RicheseAllowsUseOfNoField { get; set; }

    public bool MoritaniAllowsKeepingCards { get; set; }

    public bool EcazSharesAmbassadors { get; set; }

    public int PermittedResources { get; set; }

    public int _permittedKarmaCardId;

    [JsonIgnore]
    public TreacheryCard? PermittedKarmaCard
    {
        get => TreacheryCardManager.Lookup.Find(_permittedKarmaCardId);
        set => _permittedKarmaCardId = TreacheryCardManager.GetId(value);
    }

    #endregion Properties

    #region Validation

    public override Message? Validate()
    {
        return null;
    }

    #endregion Validation

    #region Execution

    protected override void ExecuteConcreteEvent()
    {
        var ally = Player.Ally;

        switch (Initiator)
        {
            case Faction.Guild:
                Game.GuildAllowsShippingDiscount = GuildAllowsShippingDiscount;
                break;

            case Faction.Tleilaxu:
                Game.TleilaxuAllowsRevivalDiscount = TleilaxuAllowsRevivalDiscount;
                break;

            case Faction.Ixian:
                Game.IxianAllowsReplacingCards = IxianAllowsReplacingCards;
                break;

            case Faction.Emperor:
                Game.EmperorWillPayForExtraRevival = EmperorWillPayForExtraRevival;
                break;

            case Faction.Fremen:
                Game.FremenWillProtectFromMonster = FremenWillProtectFromMonster;
                Game.FremenAllowsThreeFreeRevivals = FremenAllowsThreeFreeRevivals;
                Game.FremenSharesPrescience = FremenSharesPrescience;
                Game.FremenRefundsBattleDial = FremenRefundsBattleDial;
                break;

            case Faction.Atreides:
                Game.AtreidesSharesPrescience = AtreidesSharesPrescience;
                break;

            case Faction.BeneGesserit:
                Game.BeneGesseritAllowsUseOfVoice = BeneGesseritAllowsUseOfVoice;
                break;

            case Faction.Richese:
                Game.RicheseAllowsUseOfNoField = RicheseAllowsUseOfNoField;
                break;

            case Faction.Moritani:
                Game.MoritaniAllowsKeepingCards = MoritaniAllowsKeepingCards;
                break;

            case Faction.Ecaz:
                Game.EcazSharesAmbassadors = EcazSharesAmbassadors;
                break;

        }

        Game.PermittedUseOfAllySpice[ally] = PermittedResources;
        if (PermittedKarmaCard != null) Game.PermittedUseOfAllyKarma[ally] = PermittedKarmaCard;
    }

    public override Message GetMessage()
    {
        return Message.Express(Initiator, " change ally permissions");
    }

    #endregion Execution
}