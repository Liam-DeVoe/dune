# dotnet-compat

`rust/dotnet-compat` reimplements the .NET 10 behavior that the engine depends on: `System.Random`, `Dictionary`, `HashSet`, `List.Sort`, the LINQ operators that are easy to get wrong, and number rounding, formatting and parsing. Engine code must use these wherever the C# uses the .NET equivalent.

Everything here is checked against real .NET 10 output (see [Verification](#verification)).

## Exceptions

Where .NET throws, the compat API panics with a `CsException { type_name, message }` payload (`dotnet_compat::exception`) carrying the exception's type name and exact .NET 10 message, for example `ArgumentException: An item with the same key has already been added. Key: 5`. The engine re-exports this type as `cs::CsException`, and its `cs::catch` turns these panics back into values. `dotnet_compat::throw!(Type, "format", args)` raises one.

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
| `index(&k)`, `d[&k]`, `index_mut(&k)`, `index_with(&k, key_text)` | `d[k]` read. Throws KeyNotFoundException if missing. |
| `get_value_or_default(&k)`, `get_value_or(&k, v)` | `d.GetValueOrDefault(k)`, `d.GetValueOrDefault(k, v)` |
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
| `add`, `remove`, `contains`, `clear`, `len`, `iter` | `Add`, `Remove`, `Contains`, `Clear`, `Count`, `foreach` |
| `NetHashSet::new()`, `with_capacity(n)` | `new HashSet<T>()`, `new HashSet<T>(n)` |
| `NetHashSet::from_set(&s)` | `new HashSet<T>(otherHashSet)` |
| `NetHashSet::from_collection(&list)` | `new HashSet<T>(list)` where the source is an `ICollection<T>` such as a List, array or `Keys` |
| `NetHashSet::from_enumerable(iter)`, `collect()`, `linq::to_hash_set` | `new HashSet<T>(query)` / `query.ToHashSet()` for a lazy source |
| `union_with(iter)` | `UnionWith`: adds in the argument's order |
| `except_with(iter)` | `ExceptWith`: removes in the argument's order. Does nothing on an empty set. |
| `except_with_self()` | `s.ExceptWith(s)`: clears |
| `intersect_with_collection(&list)` | `IntersectWith(list or array)`. An empty argument calls `Clear()`. |
| `intersect_with_enumerable(iter)` | `IntersectWith(lazy query)`. An empty argument removes items one by one, which leaves free slots. |
| `intersect_with_set(&s)` | `IntersectWith(otherHashSet)` |
| `symmetric_except_with(iter)` | `SymmetricExceptWith(list or query)` |
| `symmetric_except_with_set(&s)` | `SymmetricExceptWith(otherHashSet)` |
| `remove_where(pred)` | `RemoveWhere`: tests and removes in slot order |
| `trim_excess()` | `TrimExcess()`: compacts |
| `is_subset_of`, `is_superset_of`, `overlaps`, `set_equals` | same names |

Pick the variant that matches the static type of the C# argument. `IntersectWith(emptyList)` and `IntersectWith(emptyQuery)` leave different slot layouts, and so give different orders after the next adds.

**Copying.** Unlike Dictionary, `new HashSet<T>(otherSet)` sometimes copies the slot array exactly, free list included. It does so when the source isn't oversized compared to its count. Otherwise it compacts. "Oversized" depends on .NET's internal capacity, which `NetHashSet` tracks with .NET's prime table. That is why `from_collection` and `from_enumerable` are separate: they leave different capacities. A List source is presized to its length, and a lazy source grows as it goes. The capacity only matters if the set is later copied with `from_set`.

Serde writes a JSON array in enumeration order and reads one with `Add` semantics.

## Sorting: `sort`

| Rust | C# |
|---|---|
| `sort::list_sort_by(&mut slice, cmp)` | `List<T>.Sort(Comparison<T>)`, `List<T>.Sort(IComparer<T>)`, `Array.Sort(arr, comparison)` |
| `sort::list_sort_by_int(&mut slice, cmp)` | the same with a C#-style `int` comparison |
| `sort::list_sort(&mut slice)` | `List<T>.Sort()` for `T : IComparable<T>` |
| `sort::order_by(&mut vec, cmp)` | a stable sort, for LINQ ordering |

`List.Sort` is unstable. Elements that compare equal end up in an order that depends on every detail of .NET's introsort (`ArraySortHelper<T>`), so `list_sort_by` ports it line by line. It uses a depth limit of `2 * (floor(log2 n) + 1)`, insertion sort for partitions of 16 or fewer, fixed compare-and-swap sequences for sizes 2 and 3, a median-of-three pivot parked at `hi - 1`, and heapsort when the depth limit runs out.

`List<T>.Sort()` with no arguments goes through `GenericArraySortHelper<T>`, which runs the same algorithm through `CompareTo`. `list_sort` gives the same order for any consistent `Ord`. Two exceptions don't matter in practice:

- For `float`/`double` elements, .NET first moves NaNs to the front by swapping, which can reorder `-0.0` and `0.0`.
- For primitive types, equal elements are indistinguishable, so their order can't be observed.

Exceptions match .NET, and in both cases the slice is left partially sorted, exactly as .NET leaves it:

- An inconsistent comparer can make the partition loop run off the array. That throws ArgumentException "Unable to sort because the IComparer.Compare() method returns inconsistent results. ...". The message ends with ``IComparer: 'System.Comparison`1[T]'``. The port fills in the last segment of the Rust type name, so the message only matches when the Rust and C# type names agree.
- A C# exception from the comparer is wrapped in InvalidOperationException "Failed to compare two elements in the array." The inner exception isn't kept.

Shared and Bots don't currently call `List.Sort` or `Array.Sort`; they only use LINQ ordering.

## LINQ: `linq`

Use plain iterator adapters where the semantics already match: `Where` is `filter`, `Select` is `map`, and `Any`, `All`, `Count`, `Concat`, `SelectMany` and `Contains` map directly. Use `linq` helpers for everything below.

| Rust | C# | Trap |
|---|---|---|
| `max_by_key_first`, `min_by_key_first`, `max_by_first`, `min_by_first` | `MaxBy`, `MinBy` | .NET returns the **first** extreme element. `Iterator::max_by_key` returns the last maximum. These return `None` when empty, which is what .NET does for reference types. For value types .NET throws: wrap the call in `or_no_elements(...)`. |
| `max`, `min` | `Max()`/`Min()` on `int`, enums, `Max(selector)` returning `int` | Throws on empty |
| `max_nullable`, `min_nullable` | `Max()` on `int?`, or `Max(selector)` returning a reference type such as `IComparable` | Skips nulls, returns `None` instead of throwing |
| `max_f64`, `min_f64`, `max_f32`, `min_f32` | `Max()`/`Min()` on `double`/`float` | `Max` skips NaN unless every element is NaN. `Min` returns NaN as soon as it sees one. |
| `sum_i32`, `sum_i64` | `Sum()` on ints | Checked: overflow throws OverflowException |
| `sum_f32`, `average_f32` | `Sum()`/`Average()` on floats | Accumulates in `double`, converts to `float` at the end |
| `average_i32` | `Average()` on ints | Returns `f64`; throws on empty |
| `average_f64`, `sum_f64` | the same on doubles | |
| `aggregate`, `aggregate_seed` | `Aggregate(func)`, `Aggregate(seed, func)` | Unseeded form throws on empty |
| `order_by`, `order_by_descending`, `order_by_cmp`, `order_by_f64`, `order_by_descending_f64` | `OrderBy`, `OrderByDescending`, `ThenBy` chains | All stable. Descending is **not** ascending reversed: ties keep source order. The key selector runs once per element, in source order, before sorting. That matters for `OrderBy(_ => random.Next())`. |
| `reverse` | `Reverse()` | |
| `distinct`, `distinct_by`, `union`, `intersect`, `except` | same | Results are in first-occurrence order. `Intersect` and `Except` return distinct elements of the first sequence. |
| `group_by` | `GroupBy` | Groups in order of each key's first appearance |
| `first`, `last`, `single`, `single_or_default` | `First()`, `Last()`, `Single()`, `SingleOrDefault()` | `SingleOrDefault` throws on more than one element. |
| `first_where`, `last_where`, `single_where`, `single_or_default_where` | the predicate overloads | Use these rather than `filter` + `first`: the messages differ ("no matching element", "more than one matching element"). |
| `element_at(iter, i)`, `element_at_list(&slice, i)` | `ElementAt(i)` on a lazy sequence / on a List or array | The two throw different ArgumentOutOfRangeException messages. Which one applies depends on the runtime type of the source; for example, `Where` over an empty array returns an empty array, which counts as a list. |
| `take`, `skip` | `Take(n)`, `Skip(n)` | A negative count means 0 |
| `sequence_equal`, `to_dictionary`, `to_hash_set` | same | |

For `ThenBy`, build one comparison with `Ordering::then_with`, for example `order_by_cmp(xs, |a, b| a.x.cmp(&b.x).then_with(|| b.y.cmp(&a.y)))` for `OrderBy(x).ThenByDescending(y)`.

For float keys, compare with `math::cmp_f32`/`cmp_f64`, never `partial_cmp` or `total_cmp`. .NET's `CompareTo` puts NaN first and treats `-0.0` as equal to `0.0`.

`OrderBy(...).First()` and `OrderBy(...).FirstOrDefault(pred)` give the same element as sorting and then searching. .NET computes it with a linear scan instead, which only matters if the key selector or predicate has side effects.

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

`docs/hazards.md` refers to some of these by other names: its `max_f32`/`min_f32` (meaning `Math.Max`/`Math.Min`) are `math_max_f32`/`math_min_f32`, `f32_compare` is `cmp_f32`, and `float_equals` is `equals_f32`.

These need no helper:

- `Math.Ceiling` and `Math.Floor` only take `double`. `Math.Ceiling(0.5f * x)` is `((0.5f32 * x as f32) as f64).ceil()`, because the multiply happens in `float`.
- C# `float` arithmetic is IEEE single precision. Use `f32` wherever C# uses `float`.
- Integer `/` and `%` truncate toward zero in both languages.
- C# `int` arithmetic wraps (the projects don't enable overflow checking). Rust debug builds panic on overflow, so use `wrapping_add`/`wrapping_mul` wherever C# can overflow.
- `int.ToString()` is Rust's `to_string()` under the invariant culture.

### Formatting: `fmt::f32_to_string`, `fmt::f64_to_string`

Both match `ToString()` and string interpolation under the invariant culture. They print the shortest digits that round-trip. On an exact tie between two shortest candidates, .NET picks the even last digit; Rust's formatter rounds up, and these helpers correct for that. A double switches to `E` notation when its decimal exponent reaches max(17, digit count): `1E+17`, but `10000000000000000` for 1e16. A float does the same at max(9, digit count): `1E+09`. Small numbers switch below 1e-4: `0.0001` but `1E-05`. Special values print as `Infinity`, `-Infinity`, `NaN` and `-0`. The same functions are also exposed as `math::format_f32`/`format_f64`.

### Composite formatting: `fmt::string_format`

`string.Format(format, args)` for string (or null) arguments, with .NET 10's parser: `{{`/`}}` escapes, `{index[,alignment][:format]}` items with optional spaces, padding counted in UTF-16 units, the 1,000,000 limit on index and alignment, and no `{` inside a format string. A string argument ignores the format string. Every error that throws `FormatException` in .NET is an `Err(FormatError)`. Checked against .NET by the 4,000 random templates of the skin's static corpus ([skin.md](skin.md)).

### Parsing: `parse`

Each function takes `Option<&str>`, where `None` is a C# `null`, and throws like .NET. The `try_` variants (`try_int_parse` and so on) return `Result<_, CsException>` instead.

| Rust | C# |
|---|---|
| `int_parse(s)` | `int.Parse(s)` |
| `convert_to_int32(s)` | `Convert.ToInt32(s)`: null gives 0 |
| `float_parse_invariant(s)`, `double_parse_invariant(s)` | `float.Parse(s, CultureInfo.InvariantCulture)`, `double.Parse(...)` |
| `enum_parse(s, &[(name, value), ...])` | `Enum.Parse<T>(s)` for an `int` enum |

Some .NET grammar that Rust's `from_str` doesn't accept or gets wrong:

- Surrounding whitespace is allowed, and so are trailing `'\0'` characters.
- `"1,5"` parses as 15 because `,` is the thousands separator.
- `Infinity` and `NaN` are accepted in any case, and out-of-range floats become ±Infinity.
- An int overflow is an `OverflowException`, but a format error elsewhere in the same string takes priority.
- `Enum.Parse` accepts any integer, defined or not, and ORs comma-separated names even for non-`[Flags]` enums. Its error messages quote the input with leading whitespace removed.

`math::parse_i32`, `parse_f32` and `parse_f64` do the same parsing but return a bare `ParseError`.

## Verification

`rust/dotnet-compat/csharp-ref` is a .NET 10 console program. It reads a JSON array of cases on stdin and prints one result per case. A case is a random seed plus a draw sequence, a dictionary or set operation script, a sort input, a LINQ call, or a batch of numbers or strings. Floats travel as bit patterns, so formatting never gets in the way. `tests/common/mod.rs` runs the same cases through the Rust code and compares the JSON. Exceptions are compared as `"Type: message"` strings, so every exception type and message above is checked too.

There are three test layers:

- **Golden tests** (`tests/golden.rs`, run by `cargo test`) compare against committed .NET output in `tests/golden/*.json`. They cover:
  - Random: 18 edge seeds with 3000 mixed draws each, 3 seeds with 10,000 draws, and 100 random seeds.
  - Dictionary and HashSet: 200 scripts each of 300 interleaved add/set/remove/clear/copy operations, including every set operation. Separately, about 1600 HashSet cases walk the capacity boundaries that decide whether a copy keeps free slots.
  - Sort: every length from 0 to 300 and eight inputs of length 5000, with heavy key duplication and sorted, reversed, organ-pipe and nearly sorted patterns. Plus McIlroy "killer" inputs, with and without duplicate keys, that drive the introsort into its heapsort fallback. A test asserts that they really reach it. Each input runs through one of the C# entry points (`List.Sort` with a `Comparison`, an `IComparer` or `IComparable`, and `Array.Sort`) or a descending comparison. Short and long inputs also run through a comparer that is inconsistent and one that throws, checking the exception and the partially sorted result.
  - LINQ: 2000 operator calls with duplicates, empty inputs, overflow and NaN.
  - Numbers: about 1500 doubles, floats and strings each through rounding, casts, comparison, formatting and parsing.

  Regenerate the golden files with `cargo test -p dotnet-compat --test golden -- --ignored regenerate`.
- **Differential fuzzing** (`tests/differential.rs`, ignored by default) generates about 6200 random cases per batch and runs each batch through one C# process. Run it with `cargo test -p dotnet-compat --test differential -- --ignored --nocapture`. Set `COMPAT_FUZZ_SECONDS=N` to loop for N seconds and `COMPAT_FUZZ_SEED` to replay a failing batch. A 9-minute run of 213 batches (about 1.3 million cases) against the current code found no mismatch.
- **API tests** (`tests/api.rs`) cover what the oracle can't see: serde, `Clone` versus the copy constructor, and exceptions for key types and closures the oracle doesn't model.

To check that the golden tests have teeth, each of these deliberate bugs was tried, and each made a golden test fail:

- an insertion-sort threshold of 15
- a different pivot index
- a FIFO free list
- an off-by-one in the HashSet copy threshold
- a different shrink threshold after construction

The C# tests need .NET 10 at `$DOTNET_ROOT` or `~/dotnet10`. The C# program pins the invariant culture.

## Known differences

- **`int[].Sum()` overflow on long arrays.** For an `int[]` or `List<int>` of 16 or more elements on arm64 (32 with AVX2 on x64), .NET sums with SIMD lanes and checks overflow per lane. It can therefore miss an overflow that a sequential sum would catch, or catch one it wouldn't. `sum_i32` models the sequential path, which `Sum(selector)` and shorter arrays always use. This is only observable when a sum overflows `int`.
- **NaN payloads.** `Math.Max`/`Math.Min` on arm64 quiet a signaling NaN input; `math_max_f64` and friends return it unchanged. Arithmetic never produces signaling NaNs, so only bit-level inspection of crafted values can tell the difference.
- **Culture.** All formatting and parsing assumes the invariant culture. Shared and Bots use the host culture for `int.Parse` and `ToString()`, so the oracle must run under the invariant culture too (see `docs/hazards.md` 11.2).
