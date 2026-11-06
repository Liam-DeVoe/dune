namespace Treachery.Shared;

// A faction claiming CHOAM Charity: the spice it received from the bank.
// Brown's controlled charity lines stay plain and carry no record.
public record CharityClaim(Faction Faction, int Amount);
