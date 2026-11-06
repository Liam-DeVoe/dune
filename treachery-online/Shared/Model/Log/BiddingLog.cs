namespace Treachery.Shared;

// Records the Bidding log design draws from. Only normal auctions carry them;
// White's auctions and the black market stay plain lines.

// "N cards were drawn for bidding".
public record BiddingDrawn(int Count);

// The treachery discard pile shuffled into a new deck (any phase can do this).
public record TreacheryReshuffled(int Count);

// No faction can buy cards, so there is no auction.
public record BiddingSkipped;

// Part of a card's price paid by someone other than the winner: the ally, or
// the Emperor's cunning.
public record BidShare(Faction By, int Amount);

// A card sold. Price is the whole bid. PaidTo is the faction that received
// the spice, or Faction.None when it went to the bank. With Karama the card
// was won for free by playing KaramaCard (Price is 0).
public record CardSold(int CardNumber, Faction Winner, int Price, Faction PaidTo, IReadOnlyList<BidShare> Shares, TreacheryCard? KaramaCard);

// Private to the winner: the card they bought.
public record CardWonShown(TreacheryCard Card);

// Harkonnen's free extra card, and (private) which card it is.
public record ExtraCardReceived(Faction Receiver);

public record ExtraCardShown(TreacheryCard Card);

// Everyone passed on CardNumber; bidding ends.
public record BiddingPassedByAll(int CardNumber);
