namespace Treachery.Shared;

public enum ShipKind
{
    Passed,
    Normal,
    SiteToSite,
    BackToReserves
}

// A faction's shipment, or its pass. Cost is the total paid, AllyShare the part
// Ally paid. GuildPaid when the Guild received spice for it. Karama is the card
// played to ship at Guild rates; BlockedBy the Karama card that kept the faction
// from shipping (null when unknown). Plain when the shipment uses something the
// design doesn't draw (No-Field, Karama on the Guild's income): the client
// shows the game's sentence instead.
public record ShipLogged(
    Faction Faction,
    ShipKind Kind,
    int Forces,
    int SpecialForces,
    Location? From,
    Location? To,
    int Cost,
    int AllyShare,
    Faction Ally,
    bool GuildPaid,
    TreacheryCard? Karama,
    bool Blocked,
    TreacheryCard? BlockedBy,
    bool Plain);

// Forces of Faction moving from one territory to To. A move from several
// territories logs one of these per territory, all with the same Move number.
// Hajr when moved by the Hajr card. AsAdvisors is set for Bene Gesserit moves
// under the advisor rules: whether they arrive as advisors.
public record MoveLogged(
    Faction Faction,
    int Move,
    Territory From,
    int Forces,
    int SpecialForces,
    Location To,
    bool Hajr,
    bool? AsAdvisors);

// Faction passed their move.
public record MovePassed(Faction Faction);

public enum BlueReactionKind
{
    Accompany,
    DontAccompany,
    Become,
    StayAs
}

// The Bene Gesserit answering another faction's shipment or move. Place is
// where they accompanied to, or the territory they flipped (or stayed) in.
// Advisors tells which side they flipped to or stayed as.
public record BlueReaction(BlueReactionKind Kind, object? Place, bool Advisors);

// Forces of Faction lost during their turn: Forces killed by the storm
// (Storm, logged before the shipment or move that caused it; Travelling when
// on the way) or every force in Territory killed because their ally is there.
public record ShipMoveLoss(Faction Faction, bool Storm, bool Travelling, int Forces, Territory? Territory);

// The Guild delayed their turn.
public record GuildDelayed;
