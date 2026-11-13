# dotnet-compat

`rust/dotnet-compat` reimplements the .NET 10 behavior that the engine still depends on: `System.Random`, `Dictionary` and `HashSet` (for the game fields whose .NET order players see), and number rounding, formatting and parsing. Since native-plan.md step 3c the engine uses std and `indexmap` everywhere else; [engine-conventions.md](engine-conventions.md#what-is-left-of-net) says what still needs this crate and why. `List.Sort` and the LINQ operators were removed then, because nothing used them any more.

Everything here is checked against .NET 10 output recorded in snapshot files (see [Verification](#verification)).

## Exceptions

Where .NET throws, the compat API panics with the exception's type name and exact .NET 10 message, for example `ArgumentException: An item with the same key has already been added. Key: 5`. Calling it so that .NET would throw is a bug in the caller. `dotnet_compat::throw!(Type, "format", args)` raises such a panic. The parsing functions are the exception: their `try_` variants return the exception as a `CsException { type_name, message }` value, because their input comes from outside. The engine itself no longer panics with .NET exception texts; only this crate's panics carry one.

Messages that embed a value use the `CsToString` trait for its C# `ToString()`. It is implemented for the primitive types and `String`; implement it for every dictionary key type. `NetDictionary::add_with` and `index_with` take the key text as a closure instead.

Some messages depend on the static type of the C# receiver. The table entries below say which variant to use.

## Random: `NetRandom`

| Rust | C# |
|---|---|
| `NetRandom::new(seed)` | `new Random(seed)` |
| `next()` | `Next()` |
| `next_max(max)` | `Next(max)`. Throws ArgumentOutOfRangeException if `max < 0`. |
| `next_range(min, max)` | `Next(min, max)`. Throws ArgumentOutOfRangeException if `min > max`. |
| `next_double()` | `NextDouble()` |

This is .NET's `Net5CompatSeedImpl` (Knuth's subtractive generator), which seeded `Random` still uses in .NET 10. Every edge seed works, including `int.MinValue`, `int.MaxValue`, 0 and negative seeds. `Next(0)` and `Next(x, x)` return without randomness but still consume a draw, as in .NET. Ranges wider than `int.MaxValue` use .NET's two-draw large-range path.

## Dictionary: `NetDictionary<K, V>`

.NET enumerates a dictionary in slot order. A removed entry's slot goes on a LIFO free list, and the next new key takes the most recently freed slot. `Clear` resets everything, so the next key goes to slot 0. Hash codes never affect order. `NetDictionary` keeps the same slot array and uses a std `HashMap` only to find a key's slot.

| Rust | C# |
|---|---|
| `insert(k, v)` | `d[k] = v`. An existing key keeps its slot. |
| `add(k, v)`, `add_with(k, v, key_text)` | `d.Add(k, v)`. Throws ArgumentException on a duplicate key. |
| `try_add(k, v)` | `d.TryAdd(k, v)` |
| `get(&k)`, `get_mut(&k)` | `d.TryGetValue(k, out v)` |
| `index(&k)`, `d[&k]`, `&mut d[&k]`, `index_with(&k, key_text)` | `d[k]` read. Throws KeyNotFoundException if missing. |
| `get_value_or_default(&k)` | `d.GetValueOrDefault(k)` |
| `contains_key`, `contains_value` | `ContainsKey`, `ContainsValue` |
| `remove(&k)` | `d.Remove(k)` / `d.Remove(k, out v)` |
| `clear()`, `len()`, `is_empty()` | `Clear()`, `Count` |
| `iter()`, `keys()`, `values()`, `iter_mut()`, `values_mut()` | `foreach`, `Keys`, `Values` |
| `NetDictionary::from_dictionary(&d)` | `new Dictionary<K,V>(d)` |
| `NetDictionary::to_dictionary(src, key, value)`, `linq::to_dictionary` | `src.ToDictionary(key, value)`. Throws on a duplicate key, like `Add`. |
| `collect()` (`FromIterator`) | `ToDictionary`: `Add` semantics, throws on duplicates |

**Copying.** `new Dictionary<K,V>(other)` and `ToDictionary` build a compact copy: entries in enumeration order with no free slots. Rust's `.clone()` instead copies the free list too, so later adds fill old holes. Use `from_dictionary` when porting a C# copy, and `clone()` only where you need a snapshot of the exact object.

**Serde.** A dictionary serializes as a JSON object in enumeration order. Deserializing inserts in document order with indexer semantics, so a repeated key keeps its first position and takes its last value.

`PartialEq` compares the enumerated pairs in order. C# `==` on a dictionary is reference equality, so this is only a Rust convenience.

## HashSet: `NetHashSet<T>`

Slots work the same as in `NetDictionary`. The set operations also follow .NET's internal order of removals, because removal order decides which slot the next add reuses.

| Rust | C# |
|---|---|
| `add` (also `insert`), `remove`, `contains`, `clear`, `len`, `iter` | `Add`, `Remove`, `Contains`, `Clear`, `Count`, `foreach` |
| `NetHashSet::new()`, `with_capacity(n)` | `new HashSet<T>()`, `new HashSet<T>(n)` |
| `NetHashSet::from_set(&s)` | `new HashSet<T>(otherHashSet)` |
| `NetHashSet::from_collection(&list)` | `new HashSet<T>(list)` where the source is an `ICollection<T>` such as a List, array or `Keys` |
| `NetHashSet::from_enumerable(iter)`, `collect()`, `linq::to_hash_set` | `new HashSet<T>(query)` / `query.ToHashSet()` for a lazy source |
| `union_with(iter)` | `UnionWith`: adds in the argument's order |
| `except_with(iter)` | `ExceptWith`: removes in the argument's order. Does nothing on an empty set. |
| `intersect_with_collection(&list)` | `IntersectWith(list or array)`. An empty argument calls `Clear()`. |
| `intersect_with_enumerable(iter)` | `IntersectWith(lazy query)`. An empty argument removes items one by one, which leaves free slots. |
| `intersect_with_set(&s)` | `IntersectWith(otherHashSet)` |
| `symmetric_except_with(iter)` | `SymmetricExceptWith(list or query)` |
| `symmetric_except_with_set(&s)` | `SymmetricExceptWith(otherHashSet)` |
| `remove_where(pred)` | `RemoveWhere`: tests and removes in slot order |
| `trim_excess()` | `TrimExcess()`: compacts |

Pick the variant that matches the static type of the C# argument. `IntersectWith(emptyList)` and `IntersectWith(emptyQuery)` leave different slot layouts, and so give different orders after the next adds.

**Copying.** Unlike Dictionary, `new HashSet<T>(otherSet)` sometimes copies the slot array exactly, free list included. It does so when the source isn't oversized compared to its count. Otherwise it compacts. "Oversized" depends on .NET's internal capacity, which `NetHashSet` tracks with .NET's prime table. That is why `from_collection` and `from_enumerable` are separate: they leave different capacities. A List source is presized to its length, and a lazy source grows as it goes. The capacity only matters if the set is later copied with `from_set`.

Serde writes a JSON array in enumeration order and reads one with `Add` semantics.

## Numbers: `math`, `fmt`, `parse`

### Rounding and conversion

| Rust | C# |
|---|---|
| `round_f64(x)` | `Math.Round(double)`: ties to even |
| `round_f32(x)` | `MathF.Round(float)`. C# `Math.Round(someFloat)` binds to the `double` overload, so use `round_f64(x as f64)` for that. |
| `round_f64_mode(x, MidpointRounding::AwayFromZero)` etc. | `Math.Round(x, MidpointRounding)` |
| `round_f64_digits(x, d)`, `round_f64_digits_mode(x, d, mode)` | `Math.Round(x, digits)`, `Math.Round(x, digits, mode)`. Throws ArgumentOutOfRangeException for digits outside 0..=15. |
| `abs_i32(x)` | `Math.Abs(int)`: throws OverflowException on `int.MinValue` |
| `f64_to_i32`, `f32_to_i32` | `(int)x`: truncates, NaN gives 0, saturates out of range. Same as Rust `as`. |
| `cmp_f64`, `cmp_f32` | `double.CompareTo`, `Comparer<double>.Default`: NaN first, `-0.0 == 0.0` |
| `equals_f64`, `equals_f32` | `double.Equals`, `object.Equals` on boxed floats: NaN equals NaN |
| `math_max_f64`, `math_min_f64`, `math_max_f32`, `math_min_f32` | `Math.Max`/`Math.Min` on floats: NaN propagates, `Max(-0, 0) = 0`, `Min(0, -0) = -0` |
| `NET_NAN_F64`, `NET_NAN_F32` | `double.NaN`/`float.NaN`, which have the sign bit set, unlike Rust's `f64::NAN` |

These need no helper:

- `Math.Ceiling` and `Math.Floor` only take `double`. `Math.Ceiling(0.5f * x)` is `((0.5f32 * x as f32) as f64).ceil()`, because the multiply happens in `float`.
- C# `float` arithmetic is IEEE single precision. Use `f32` wherever C# uses `float`.
- Integer `/` and `%` truncate toward zero in both languages.
- C# `int` arithmetic wraps (the projects don't enable overflow checking). Rust debug builds panic on overflow, so use `wrapping_add`/`wrapping_mul` wherever C# can overflow.
- `int.ToString()` is Rust's `to_string()` under the invariant culture.

### Formatting: `fmt::f32_to_string`, `fmt::f64_to_string`

Both match `ToString()` and string interpolation under the invariant culture. They print the shortest digits that round-trip. On an exact tie between two shortest candidates, .NET picks the even last digit; Rust's formatter rounds up, and these helpers correct for that. A double switches to `E` notation when its decimal exponent reaches max(17, digit count): `1E+17`, but `10000000000000000` for 1e16. A float does the same at max(9, digit count): `1E+09`. Small numbers switch below 1e-4: `0.0001` but `1E-05`. Special values print as `Infinity`, `-Infinity`, `NaN` and `-0`. The same functions are also exposed as `math::format_f32`/`format_f64`.

### Composite formatting: `fmt::string_format`

`string.Format(format, args)` for string (or null) arguments, with .NET 10's parser: `{{`/`}}` escapes, `{index[,alignment][:format]}` items with optional spaces, padding counted in UTF-16 units, the 1,000,000 limit on index and alignment, and no `{` inside a format string. A string argument ignores the format string. Every error that throws `FormatException` in .NET is an `Err(FormatError)`. Checked by the 4,000 random templates of the skin's static corpus ([skin.md](skin.md)), whose expected texts were recorded from .NET.

### Parsing: `parse`

Each function takes `Option<&str>`, where `None` is a C# `null`, and throws like .NET. The `try_` variants (`try_int_parse` and so on) return `Result<_, CsException>` instead.

| Rust | C# |
|---|---|
| `int_parse(s)` | `int.Parse(s)` |
| `convert_to_int32(s)` | `Convert.ToInt32(s)`: null gives 0 |
| `float_parse_invariant(s)` | `float.Parse(s, CultureInfo.InvariantCulture)` |
| `enum_parse(s, &[(name, value), ...])` | `Enum.Parse<T>(s)` for an `int` enum |

Some .NET grammar that Rust's `from_str` doesn't accept or gets wrong:

- Surrounding whitespace is allowed, and so are trailing `'\0'` characters.
- `"1,5"` parses as 15 because `,` is the thousands separator.
- `Infinity` and `NaN` are accepted in any case, and out-of-range floats become ±Infinity.
- An int overflow is an `OverflowException`, but a format error elsewhere in the same string takes priority.
- `Enum.Parse` accepts any integer, defined or not, and ORs comma-separated names even for non-`[Flags]` enums. Its error messages quote the input with leading whitespace removed.

`math::parse_i32`, `parse_f32` and `parse_f64` do the same parsing but return a bare `ParseError`.

## Verification

`tests/golden/{random,dict,set,math}.json` hold fixed cases whose expected results were recorded from .NET 10:

- Random: 18 edge seeds with 3000 mixed draws each, 3 seeds with 10,000 draws, and 100 random seeds.
- Dictionary and HashSet: 200 scripts each of 300 interleaved add/set/remove/clear/copy operations, including every set operation. Separately, about 1600 HashSet cases walk the capacity boundaries that decide whether a copy keeps free slots.
- Numbers: about 1500 doubles, floats and strings each through rounding, casts, comparison, formatting and parsing.

They pin the random sequence of each seed and the dictionary and set slot order, which replaying a savegame depends on. `cargo test -p dotnet-compat --test golden` runs the cases through the Rust code (`tests/common/mod.rs`) and compares. Floats travel as bit patterns, and exceptions compare as `"Type: message"` strings, so every exception type and message above is checked too. Rewrite the expected values with `UPDATE_GOLDENS=1` only together with a deliberate change to randomness or ordering, and review the diff.

`tests/api.rs` covers what the cases don't: serde, `Clone` versus the copy constructor, and the panics that stand for .NET exceptions.

To check that the golden tests have teeth, each of these deliberate bugs was tried, and each made a golden test fail:

- a FIFO free list
- an off-by-one in the HashSet copy threshold
- a different shrink threshold after construction

## Known differences

- **NaN payloads.** `Math.Max`/`Math.Min` on arm64 quiet a signaling NaN input; `math_max_f64` and friends return it unchanged. Arithmetic never produces signaling NaNs, so only bit-level inspection of crafted values can tell the difference.
- **Culture.** All formatting and parsing assumes the invariant culture. The C# engine used the host culture for `int.Parse` and `ToString()`; the Rust engine always behaves as under the invariant culture.
