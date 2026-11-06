namespace Treachery.Shared;

// One faction reviving: what came back and what it cost. Forces and
// SpecialForces are each counted once. Cost is the total paid, PaidByOther the
// part of it someone else (the Emperor, for an ally) paid. Card is set when a
// card did the reviving (Tleilaxu Ghola, Karama) and nothing was paid.
public record RevivalRow(Faction Faction, int Forces, int SpecialForces, int Cost)
{
    public IHero? Hero { get; init; }
    public bool AsGhola { get; init; }
    public int PaidByOther { get; init; }
    public Faction Payer { get; init; }
    public TreacheryCard? Card { get; init; }
    public int PurpleIncome { get; init; }
}
