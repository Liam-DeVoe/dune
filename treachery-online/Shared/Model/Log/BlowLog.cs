/*
 * Copyright (C) 2020-2025 Ronald Ossendrijver (admin@treachery.online)
 * This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This
 * program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have
 * received a copy of the GNU General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

namespace Treachery.Shared;

// What the Spice Blow log (Client/OtherComponents/Log/BlowLog.razor) draws,
// attached to the messages the game already writes. Messages without one of
// these are drawn as plain lines.

// The heading over one spice deck's lines when Increased Spice Flow draws from two ("A" or "B").
public sealed record BlowDeckLog(string Deck);

// A spice card came up: Amount spice in Location, or lost because the storm is over it.
// Doubled is set when a sandtrout doubled the amount.
public sealed record BlowCardLog(ResourceCard Card, Location Location, int Amount, bool Lost, bool Doubled);

// Shai Hulud appears in Territory. Nth counts its appearances in this deck: from the
// second on the game's sentence doesn't name the territory, so it is kept here.
public sealed record WormAppearedLog(Territory Territory, int Nth);

// A faction sends a later worm to Territory.
public sealed record WormSentLog(Faction By, Territory Territory);

// Shai Hulud devours a faction's forces in Location.
public sealed record WormDevouredForcesLog(Faction Faction, int Forces, int SpecialForces, Location Location);

// A faction's forces in Location survive Shai Hulud.
public sealed record WormSurvivedLog(Faction Faction, Location Location);

// Shai Hulud devours the spice in Territory.
public sealed record WormDevouredSpiceLog(int Amount, Territory Territory);

// Shai Hulud came up on turn 1 and was ignored.
public sealed record WormIgnoredLog;

// The spice discard pile was shuffled into a new deck.
public sealed record BlowReshuffleLog(int Cards);

// A line of the Nexus that follows a worm: an alliance offered, made, ended or broken,
// or the Fremen riding (or passing a ride on) the worm. Ride is set for a ride,
// Alliance for a change of alliance.
public sealed record NexusLog(NexusRide? Ride = null, NexusAlliance? Alliance = null);

public enum NexusAllianceChange { Offered, Made, Ended }

// Offered: From offers an alliance to To. Made: From and To are now allies.
// Ended: the alliance of From and To is over (From ended it, or all alliances broke).
public sealed record NexusAlliance(NexusAllianceChange Change, Faction From, Faction To);

// Forces ridden on the worm, from From (null: from reserves) to To.
public sealed record NexusRide(Faction Faction, int Forces, int SpecialForces, Location? From, Location To);
