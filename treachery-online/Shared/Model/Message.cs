/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared;

public class Message
{
    public static IDescriber? DefaultDescriber = null;

    private static int Counter { get; set; }

    public int Nr { get; } = Counter++;

    public Faction Target { get; set; }

    private Message(Expression e)
    {
        Expression = e;
    }

    private Message(Faction to, Expression e)
    {
        Expression = e;
        Target = to;
    }

    public Expression Expression { get; }

    // Set when the message tells of forces travelling, so the client can
    // draw the journey while the message is hovered.
    public Route? Route { get; private set; }

    public Message WithRoute(Faction faction, IEnumerable<Location> from, Location? to)
    {
        Route = new Route(faction, from.ToList(), to);
        return this;
    }

    // Set when the message heads the lines after it, such as the spice deck a
    // blow was drawn from. The concept picks the icon drawn beside it.
    public Concept? Heading { get; private set; }

    public Message AsHeading(Concept icon)
    {
        Heading = icon;
        return this;
    }

    // Set when the message reveals both battle plans, so the client can lay
    // them out side by side.
    public BattlePlans? Plans { get; private set; }

    public Message WithPlans(PlanSummary aggressor, PlanSummary defender)
    {
        Plans = new BattlePlans(aggressor, defender);
        return this;
    }

    // Set when the message states one fact of a battle's outcome, so the
    // client can gather the facts into the battle's report block.
    public BattleFact? Fact { get; private set; }

    public Message WithFact(BattleFact fact)
    {
        Fact = fact;
        return this;
    }

    public override string ToString()
    {
        if (DefaultDescriber != null)
            return ToString(DefaultDescriber) + "*";
        return string.Join("", Expression.Elements);
    }

    public string ToString(IDescriber describer)
    {
        return string.Join("", Expression.Elements.Select(x => x == null 
            ? string.Empty 
            : describer.Describe(x)));
    }

    public static Message Express(params object?[] list)
    {
        return new Message(new Expression(list));
    }

    public static Message ExpressTo(Faction to, params object?[] list)
    {
        return new Message(to, new Expression(list));
    }
}

public class MessagePart
{
    private MessagePart(Expression e)
    {
        Expression = e;
    }

    public Expression Expression { get; }

    public static MessagePart Express(params object[] list)
    {
        return new MessagePart(new Expression(list));
    }

    public static MessagePart ExpressIf(bool condition, params object?[] list)
    {
        if (condition)
            return new MessagePart(new Expression(list));
        
        return new MessagePart(new Expression());
    }

    public string ToString(IDescriber describer)
    {
        return string.Join("", Expression.Elements.Select(x => x == null 
            ? string.Empty 
            : describer.Describe(x)));
    }

    public override string ToString()
    {
        if (Message.DefaultDescriber != null)
            return ToString(Message.DefaultDescriber) + "*";
        return string.Join("", Expression.Elements);
    }
}
// Where forces went in a message: from the listed locations, or from reserves
// when the list is empty, to a location, or back to reserves when To is null.
public record Route(Faction Faction, List<Location> From, Location? To);

// Both revealed battle plans, aggressor first.
public record BattlePlans(PlanSummary Aggressor, PlanSummary Defender);

// One side's revealed plan. Spice is null when spice isn't dialed (no
// advanced combat). AllyContribution is spice paid by Ally.
public record PlanSummary(
    Faction Faction,
    IHero? Leader,
    bool Messiah,
    float Dial,
    int? Spice,
    int AllyContribution,
    Faction Ally,
    TreacheryCard? Weapon,
    TreacheryCard? Defense);

public enum BattleFactKind
{
    // Faction's total strength was Strength.
    Strength,

    // Faction won the battle.
    Winner,

    // Hero was killed by Cause; Faction got Value spice for it.
    Killed,

    // Hero turned out to be a traitor working for Faction.
    Traitor,

    // Faction lost Forces and SpecialForces; All when that was every force they had there.
    ForcesLost,

    // Faction discarded Card.
    Discard
}

// One fact of a battle's outcome. Only the fields its kind describes are set.
public record BattleFact(BattleFactKind Kind, Faction Faction)
{
    public float Strength { get; init; }
    public IHero? Hero { get; init; }
    public object? Cause { get; init; }
    public int Value { get; init; }
    public int Forces { get; init; }
    public int SpecialForces { get; init; }
    public bool All { get; init; }
    public TreacheryCard? Card { get; init; }
}
