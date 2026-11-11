# Collections inside messages

A C# message stores its arguments as raw objects. When one of them is a collection, the message keeps a reference to that collection object, not a copy. The oracle's state dump ([trace-format.md](trace-format.md) section 7.5) writes every message at every step and enumerates such a collection each time. So if a `List` in game state changes after it was logged, the dump of the old message shows the new contents. A lazy LINQ query is run again at each dump, against whatever its source holds then.

The Rust port copies an element when the message is built. That is the same as C# unless the collection can change after the message is logged. This file lists every C# site where a collection can end up in a message or log record, and says why the Rust port matches.

## How the sites are found

`engine/csharp-gen` lists them from the Roslyn operation trees of all of Shared:

```sh
cd rust/engine/csharp-gen && dotnet run -c Release -- --message-collections ../tests/coverage/message_collections.txt
```

A site is any of these:

- a value whose static type is a collection (anything `IEnumerable` other than `string`), or a type parameter, converted to `object`. Every argument of `Message.Express` and the logging helpers passes through `params object?[]`, so this is how a collection gets into a message. It also catches `object` fields such as `BattleFact.Cause` and `BeneGesseritReaction.Place`. Conversions inside `==` and `!=` are skipped, since they store nothing.
- an array passed as is to a `params object?[]` parameter. The array becomes the message's element array.
- a collection passed to a constructor of a record in `Model/Message.cs` or `Model/Log/`, or assigned to one of their properties.

Roslyn fails on three constructors of `Location` subclasses. The tool lists them as `UNANALYZED` and they were checked by hand.

`engine/tests/live_lists.rs` fails if a site in `message_collections.txt` has no row below, or if a row names a site that is no longer in the file. Regenerate the file after any change to Shared.

## Classes

- **copy**: the collection is fresh, or nothing changes it after the message is built. A Rust copy is equivalent.
- **live**: the collection belongs to game state and changes later. Rust stores the element with an `expression::LiveList` source, and the state dump reads the game's current contents (see [engine-conventions.md](engine-conventions.md#messages)).
- **lazy**: a LINQ query that the dump runs again each time. Rust stores its result once. That is equivalent when nothing the query reads changes, and the query has no side effects and can't throw. The row says why that holds. Its runtime type name only matters for `ToString()`, and each one was checked against .NET 10.
- **forward**: a `params` array passed on to another `params` method. The array that ends up in the message is created by the outermost caller, which is covered by its own row or is a list of separate arguments.
- **none**: nothing stores the collection in a message.

What the dump does with a lazy query: `StateDumper.Sequence` enumerates it with `foreach` and writes the items. If enumeration throws, it writes `{"$enumerationError":...}` and stops. The three lazy queries in Shared run `Where` and `Select` lambdas that only read their own items and build a `MessagePart` and a `Payment`. Neither draws a random number or touches the static `Message.Counter` (only `Message` has `Nr`, not `MessagePart`). None of them can throw on the inputs they get.

## Sites

| Site | Class | Why |
|---|---|---|
| `Enumerations.cs:21:defaultValue` | none | `v.Equals(defaultValue)` boxes an enum. No message. |
| `Events/AmbassadorActivated.cs:333:victimPlayer.TreacheryCards` | live | The victim's hand, which changes in later events. `LiveList::PlayerTreacheryCards`. |
| `Events/AuditCancelled.cs:89:Game.AuditedCards` | live | Cleared and refilled by the next audit (`Game_Battle.cs:427-429`). `LiveList::GameAuditedCards`. |
| `Events/Bid.cs:222:[]` | copy | An empty collection literal for `CardSold.Shares`. Nothing else holds it. |
| `Events/Bid.cs:296:Game.Bids.Select(b => MessagePart.Express(b.Key, Payment.Of(b.Value.TotalAmount), " ")).ToList()` | copy | `ToList()` of new message parts. |
| `Events/BlackMarketBid.cs:117:Game.Bids.Select(b => MessagePart.Express(b.Key, Payment.Of(b.Value.TotalAmount), " ")).ToList()` | copy | As above. |
| `Events/BeneGesseritAccompanies.cs:70:[]` | copy | `WithRoute` copies its argument with `ToList()` (`Message.cs:41`). |
| `Events/CardsDetermined.cs:80:TreacheryCards` | copy | The getter builds a new `List` from the id string on every call (`GameEvent.IdStringToObjects`). |
| `Events/CardsDetermined.cs:80:RicheseCards` | copy | As above. |
| `Events/CardsDetermined.cs:81:TreacheryCards` | copy | As above. |
| `Events/DealAccepted.cs:152:offer.Player.DiscardedTraitors` | live | The player's get-only `HashSet<IHero>`, which `TraitorsSelected` adds to. `LiveList::PlayerDiscardedTraitors`. |
| `Events/DealOffered.cs:184:To` | copy | The event's own `Faction[]`. Nothing in Shared writes its elements or reassigns it. |
| `Events/EstablishPlayers.cs:222:customRules` | copy | A local `ToList()`. |
| `Events/FaceDanced.cs:191:ForceLocations.Where(fl => fl.Value.TotalAmountOfForces > 0).Select(fl => MessagePart.Express(", ", fl.Value.AmountOfForces, " from ", fl.Key))` | lazy | Runs over the dictionary that the `ForceLocations` getter parsed for this call, with new `Battalion` objects that nothing else holds. Type ``System.Linq.Enumerable+IEnumerableWhereSelectIterator`2[System.Collections.Generic.KeyValuePair`2[Treachery.Shared.Location,Treachery.Shared.Battalion],Treachery.Shared.MessagePart]``. |
| `Events/GameEvent.cs:319:expression` | forward | `GameEvent.Log` to `Game.Log`. |
| `Events/GameEvent.cs:324:expression` | forward | `GameEvent.LogIf` to `Game.LogIf`. |
| `Events/GameEvent.cs:329:expression` | forward | `GameEvent.LogTo` to `Game.LogTo`. |
| `Events/Karma.cs:251:Prevented is FactionAdvantage.None ? [] : ["prevent ", Prevented]` | copy | A new array (or the shared empty array) that becomes the `effect` elements. Nothing writes to it. |
| `Events/KarmaChoamDiscard.cs:68:Cards.Select(c => MessagePart.Express(" ", c, " "))` | lazy | Runs over the `List` that the `Cards` getter built for this call. Type ``System.Linq.Enumerable+ListSelectIterator`2[Treachery.Shared.TreacheryCard,Treachery.Shared.MessagePart]``. |
| `Events/KarmaChoamDiscard.cs:78:Cards.Select(c => MessagePart.Express(" ", c, " "))` | lazy | As above. |
| `Events/EmperorBidSupport.cs:52:Amounts.Where(kvp => kvp.Value > 0).Select(f => MessagePart.Express(f.Key, ":", Payment.Of(f.Value), " "))` | lazy | Runs over the event's `Amounts`. `Game.PermittedUseOfEmperorSpice` holds the same dictionary, but it is only read (`Game_Alliances.cs:160`) and later replaced, never changed. Type ``System.Linq.Enumerable+IEnumerableWhereSelectIterator`2[System.Collections.Generic.KeyValuePair`2[Treachery.Shared.Faction,System.Int32],Treachery.Shared.MessagePart]``. |
| `Events/SetIncreasedRevivalLimits.cs:59:Factions` | copy | The event's own `Faction[]`. `Game.FactionsWithIncreasedRevivalLimits` holds the same array, but only reads it or replaces it (`EndPhase.cs:340`). |
| `Events/SetShipmentPermission.cs:71:Factions` | copy | The event's own `Faction[]`, only read. |
| `Events/SetShipmentPermission.cs:72:Factions` | copy | As above. |
| `Events/Shipment.cs:669:Sources` | copy | `WithRoute` copies with `ToList()`. |
| `Events/Shipment.cs:670:Sources` | copy | As above. |
| `Events/Shipment.cs:689:Sources` | copy | As above. |
| `Events/Shipment.cs:691:Sources` | copy | As above. |
| `Events/Shipment.cs:694:Sources` | copy | As above. |
| `Events/Shipment.cs:696:Sources` | copy | As above. |
| `Events/FremenRidesMonster.cs:159:[from]` | copy | As above. |
| `Events/FremenRidesMonster.cs:173:[]` | copy | As above. |
| `Events/FremenRidesMonster.cs:194:ForceLocations.Keys` | copy | As above. |
| `Game.cs:693:effect` | forward | `PlayKarma` to `KarmaPlayed`. |
| `Game.cs:703:effect` | copy | The `params` array of `KarmaPlayed` becomes the elements of a `MessagePart`. Every caller passes separate arguments, or the new array from `Karma.cs:251`. |
| `Game.cs:787:expression` | forward | `Game.Log` to `Report.Express`. |
| `Game.cs:792:expression` | forward | `Game.LogIf` to `Report.Express`. |
| `Game.cs:797:expression` | forward | `Game.LogTo` to `Report.ExpressTo`. |
| `Game_Bidding.cs:192:shares` | copy | A local `List` that only `CardSold` holds. |
| `Game_Bidding.cs:207:expression` | forward | `LogBidding` to `Message.Express`. |
| `Game_Bidding.cs:215:expression` | forward | `LogBiddingTo` to `Message.ExpressTo`. |
| `Game_Contemplate.cs:135:Winners.Select(w => w.Faction).ToList()` | copy | `ToList()`. |
| `Game_Contemplate.cs:277:expression` | forward | `LogWinReason` to `Message.Express`. |
| `Game_Move.cs:195:fromLocations` | copy | `WithRoute` copies with `ToList()`. |
| `Game_Other.cs:166:messageElements` | copy | The `params` array of `PlayNexusCard` becomes the elements of a `MessagePart`. Every caller passes separate arguments. |
| `Model/DiscoveryStronghold.cs:16:UNANALYZED` | none | A constructor that sets two fields. |
| `Model/Expression.cs:18:list` | forward | `Expression.Express` to the constructor. |
| `Model/GameStatus.cs:820:toShow` | none | `FlashInfo` for the client. `GameStatus` is not game state. |
| `Model/GameStatus.cs:870:elements` | none | As above. |
| `Model/HiddenMobileStronghold.cs:14:UNANALYZED` | none | A constructor that sets one field. |
| `Model/Homeworld.cs:30:UNANALYZED` | none | A constructor that sets fields from its value parameters. |
| `Model/Message.cs:101:list` | forward | `Message.Express` to `new Expression`, which stores the array. |
| `Model/Message.cs:106:list` | forward | `Message.ExpressTo`, as above. |
| `Model/Message.cs:121:list` | forward | `MessagePart.Express`, as above. |
| `Model/Message.cs:127:list` | forward | `MessagePart.ExpressIf`, as above. |
| `Model/Message.cs:143:Expression.Elements` | none | `string.Join` in `ToString`. |
| `Model/Message.cs:41:from.ToList()` | copy | `Route.From` is a new list. |
| `Model/Message.cs:89:Expression.Elements` | none | `string.Join` in `ToString`. |
| `Model/Report.cs:36:list` | forward | `Report.Express` to `Message.Express`. |
| `Model/Report.cs:41:list` | forward | `Report.ExpressTo` to `Message.ExpressTo`. |

Counts: 3 live, 4 lazy, 28 copy, 17 forward, 8 none (60 sites).

## What this does not cover

- A `LiveList` names its collection by where it sits in the game, such as player 2's hand. C# keeps the collection object. The two agree because none of the three fields is ever reassigned during a game. `Player.Clone` copies `TreacheryCards`, but only `Game.Clone` calls it, and nothing in Shared, Bots or the oracle calls that.
- A message whose own fields change after it is logged (`Message.Target` has a public setter, `WithLog` and `AsHeading` return the same object) is a different kind of aliasing. It is not a collection, so the tool doesn't list it.
