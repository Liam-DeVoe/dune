/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

using System.Linq;

namespace Treachery.Client;

// Actions that play a treachery card from your hand. Instead of a panel in
// the right column, the card glows in your faction card (PlayerPanel), and
// clicking it opens the panel in a popover (CardPlayPopover). An action
// whose card isn't in your hand stays in the right column, so nothing
// becomes unreachable.
public static class CardPlays
{
    private static readonly Dictionary<Type, TreacheryCardType> Plays = new()
    {
        [typeof(Karma)] = TreacheryCardType.Karma,
        [typeof(KarmaChoamDiscard)] = TreacheryCardType.Karma,
        [typeof(KarmaFreeRevival)] = TreacheryCardType.Karma,
        [typeof(KarmaHandSwapInitiated)] = TreacheryCardType.Karma,
        [typeof(KarmaHmsMovement)] = TreacheryCardType.Karma,
        [typeof(KarmaMonster)] = TreacheryCardType.Karma,
        [typeof(KarmaEcazDial)] = TreacheryCardType.Karma,
        [typeof(KarmaPrescience)] = TreacheryCardType.Karma,
        [typeof(KarmaRevivalPrevention)] = TreacheryCardType.Karma,
        [typeof(KarmaShipmentPrevention)] = TreacheryCardType.Karma,
        [typeof(KarmaRicheseBuy)] = TreacheryCardType.Karma,
        [typeof(AmalPlayed)] = TreacheryCardType.Amal,
        [typeof(Caravan)] = TreacheryCardType.Caravan,
        [typeof(ClairVoyancePlayed)] = TreacheryCardType.Clairvoyance,
        [typeof(DiscardedSearchedAnnounced)] = TreacheryCardType.SearchDiscarded,
        [typeof(DistransUsed)] = TreacheryCardType.Distrans,
        [typeof(HarvesterPlayed)] = TreacheryCardType.Harvester,
        [typeof(JuicePlayed)] = TreacheryCardType.Juice,
        [typeof(MetheorPlayed)] = TreacheryCardType.Metheor,
        [typeof(PortableAntidoteUsed)] = TreacheryCardType.PortableAntidote,
        [typeof(RaiseDeadPlayed)] = TreacheryCardType.RaiseDead,
        [typeof(RecruitsPlayed)] = TreacheryCardType.Recruits,
        [typeof(ResidualPlayed)] = TreacheryCardType.Residual,
        [typeof(StormSpellPlayed)] = TreacheryCardType.StormSpell,
        [typeof(ThumperPlayed)] = TreacheryCardType.Thumper,
    };

    // The cards in hand that play this action.
    private static IEnumerable<TreacheryCard> CardsFor(Type action, Game game, Player? player)
    {
        if (player == null || !Plays.TryGetValue(action, out var type)) return [];
        return type == TreacheryCardType.Karma
            ? Karma.ValidKarmaCards(game, player)
            : player.TreacheryCards.Where(c => c.Type == type);
    }

    // Whether the action is played from a card in hand, so the right column leaves it out.
    public static bool PlayedFromHand(Type action, Game game, Player? player) => CardsFor(action, game, player).Any();

    // Whether the action plays this card.
    public static bool PlaysCard(Type action, TreacheryCard card, Game game, Player? player) => CardsFor(action, game, player).Contains(card);

    // Whether the card can be played now.
    public static bool Playable(TreacheryCard card, IEnumerable<Type> actions, Game game, Player? player) =>
        actions.Any(a => PlaysCard(a, card, game, player));
}

// The trade and alliance actions, opened from the menu bar (TradePopover)
// instead of shown in the right column.
public static class Trade
{
    public static readonly HashSet<Type> Actions = [typeof(DealOffered), typeof(Donated), typeof(AllyPermission)];
}
