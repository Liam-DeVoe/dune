//! C# runtime behavior the engine relies on: exceptions and the LINQ
//! operators that throw. Support module (no C# counterpart).
//!
//! A C# exception becomes a Rust panic whose payload is a [`CsException`].
//! The replayer runs each event inside [`catch`], which turns that panic back
//! into a value, so partially applied state changes stay visible exactly as
//! they do in C# when an exception escapes halfway through a handler.
//!
//! Any other panic (a Rust `unwrap`, an index out of bounds) is a porting bug
//! and is not caught.

use std::cell::{Cell, Ref, RefCell, RefMut};
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::Once;

/// `CsException` and `throw` live in dotnet-compat so that its throwing APIs
/// (`NetDictionary::add`, `linq::first`, ...) raise the same payload.
pub use dotnet_compat::exception::{CsException, throw};

/// `throw new X(message)`:
///
/// ```should_panic
/// treachery_engine::throw!(InvalidOperationException, "Cannot draw from an empty deck");
/// ```
///
/// The first argument is the C# exception type name; the rest is `format!`
/// arguments for the message (C# `$"..."` interpolation). With no message,
/// the exception gets the .NET default text of its parameterless constructor
/// (`throw new NullReferenceException()` says "Object reference not set to an
/// instance of an object."). Format numbers in messages with
/// `dotnet_compat::fmt`, never with Rust's `{}` for floats.
#[macro_export]
macro_rules! throw {
    ($ty:ident) => {
        $crate::cs::throw(stringify!($ty), $crate::cs::default_message(stringify!($ty)).to_string())
    };
    ($ty:ident, $($arg:tt)+) => {
        $crate::cs::throw(stringify!($ty), format!($($arg)+))
    };
}

thread_local! {
    static CATCH_DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// Wraps the panic hook so that C# exceptions thrown inside [`catch`] print
/// nothing, and uncaught ones print their type and message.
fn install_panic_hook() {
    static HOOK: Once = Once::new();
    HOOK.call_once(|| {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            if let Some(e) = info.payload().downcast_ref::<CsException>() {
                // Debugging aid: the Rust call stack of every C# exception.
                if std::env::var_os("CS_EXCEPTION_BACKTRACE").is_some() {
                    eprintln!("C# exception {}: {}\n{}", e.type_name, e.message, std::backtrace::Backtrace::force_capture());
                }
                if CATCH_DEPTH.with(|d| d.get()) > 0 {
                    return;
                }
                let at = info.location().map(|l| l.to_string()).unwrap_or_default();
                eprintln!("uncaught C# exception {}: {} at {}", e.type_name, e.message, at);
                return;
            }
            previous(info)
        }));
    });
}

/// Runs `f`, returning `Err` if it threw a C# exception. Panics that are not
/// C# exceptions are propagated.
pub fn catch<R>(f: impl FnOnce() -> R) -> Result<R, CsException> {
    install_panic_hook();
    install_null_key_check();
    CATCH_DEPTH.with(|d| d.set(d.get() + 1));
    let result = panic::catch_unwind(AssertUnwindSafe(f));
    CATCH_DEPTH.with(|d| d.set(d.get() - 1));
    match result {
        Ok(r) => Ok(r),
        Err(payload) => match payload.downcast::<CsException>() {
            Ok(e) => Err(*e),
            Err(other) => panic::resume_unwind(other),
        },
    }
}

pub const NULL_REFERENCE_MESSAGE: &str = "Object reference not set to an instance of an object.";
pub const INDEX_OUT_OF_RANGE_MESSAGE: &str =
    "Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')";

/// Dereferencing a C# reference that may be null (`x.Foo` or `x!.Foo` where
/// `x` is nullable) throws NullReferenceException when it is null.
pub trait NullRef<T> {
    /// `x!` followed by a member access: the value, or NullReferenceException.
    fn nre(self) -> T;
}

impl<T> NullRef<T> for Option<T> {
    #[track_caller]
    fn nre(self) -> T {
        match self {
            Some(v) => v,
            None => throw("NullReferenceException", NULL_REFERENCE_MESSAGE.to_string()),
        }
    }
}

/// `throw new NullReferenceException()` as the runtime raises it.
#[track_caller]
pub fn null_reference() -> ! {
    throw("NullReferenceException", NULL_REFERENCE_MESSAGE.to_string())
}

/// `ArgumentNullException` for a null argument named `parameter`:
/// "Value cannot be null. (Parameter 'key')".
#[track_caller]
pub fn argument_null(parameter: &str) -> ! {
    throw("ArgumentNullException", format!("Value cannot be null. (Parameter '{parameter}')"))
}

/// The source of a LINQ operator (`x.Where(...)`, `x.Sum()`, `x.ToList()`,
/// ...) that may be null: .NET checks it before anything else and throws
/// ArgumentNullException "Value cannot be null. (Parameter 'source')".
#[track_caller]
pub fn linq_source<T: ?Sized>(source: Option<&T>) -> &T {
    match source {
        Some(s) => s,
        None => argument_null("source"),
    }
}

/// A static-data handle (`TreacheryCard`, `Leader`, `Hero`, `Location`,
/// `Territory`, `ResourceCard`) that can be a C# null reference. See
/// engine-conventions.md, "Null handles".
///
/// The null handle can be stored in any collection, equals only itself (like
/// `null == null`), dumps as `null`, is `null` in messages and `-1` as a JSON
/// id, and throws NullReferenceException from every member.
pub trait CsNull: Copy + PartialEq {
    const NULL: Self;

    fn is_null(self) -> bool {
        self == Self::NULL
    }

    /// The handle as a nullable value: `None` for the null handle.
    fn opt(self) -> Option<Self> {
        if self.is_null() { None } else { Some(self) }
    }

    /// A member access: NullReferenceException for the null handle.
    #[track_caller]
    fn nn(self) -> Self {
        if self.is_null() {
            null_reference()
        }
        self
    }
}

/// Whether `v` is a null handle, for generic code (`Self::Item` of an iterator,
/// which may be a reference, so `TypeId` can't be used). Recognizes every
/// handle type and references to them by `type_name`, which is consistent
/// within one build.
pub fn is_null_value<T>(v: &T) -> bool {
    use crate::{
        i_hero::Hero, leader::Leader, location::Location, player::PlayerIndex, resource_card::ResourceCard,
        territory::Territory, treachery_card::TreacheryCard,
    };
    use std::any::type_name;
    fn check<T, H: CsNull>(v: &T) -> Option<bool> {
        let t = type_name::<T>();
        if t == type_name::<H>() {
            // SAFETY: T and H are the same type.
            return Some(unsafe { std::mem::transmute_copy::<T, H>(v) }.is_null());
        }
        if t == type_name::<&H>() {
            // SAFETY: T is &H.
            let r: &H = unsafe { std::mem::transmute_copy::<T, &H>(v) };
            return Some(r.is_null());
        }
        None
    }
    check::<T, TreacheryCard>(v)
        .or_else(|| check::<T, Leader>(v))
        .or_else(|| check::<T, Hero>(v))
        .or_else(|| check::<T, Location>(v))
        .or_else(|| check::<T, Territory>(v))
        .or_else(|| check::<T, ResourceCard>(v))
        .or_else(|| check::<T, PlayerIndex>(v))
        .unwrap_or(false)
}

/// A C# "OrDefault" result as an `Option`: a null element becomes `None`, the
/// same null that an empty source gives (engine-conventions.md, "Null handles").
pub fn null_to_none<T>(v: Option<T>) -> Option<T> {
    match v {
        Some(x) if is_null_value(&x) => None,
        other => other,
    }
}

/// `Option<Handle>` to a handle that may be null, for storing a nullable
/// value in a collection (`hand.Add(card)` where `card` may be null).
pub trait OrNull<T> {
    fn or_null(self) -> T;
}

impl<T: CsNull> OrNull<T> for Option<T> {
    fn or_null(self) -> T {
        match self {
            Some(v) => v,
            None => T::NULL,
        }
    }
}

/// Makes `NetDictionary` throw ArgumentNullException for a null handle key,
/// as .NET does for a null key. Called by `catch` and `Map::new`, so it is in
/// place before any game exists.
pub fn install_null_key_check() {
    fn is_null_key(k: &dyn std::any::Any) -> bool {
        use crate::{
            i_hero::Hero, leader::Leader, location::Location, resource_card::ResourceCard, territory::Territory,
            treachery_card::TreacheryCard,
        };
        k.downcast_ref::<Location>().is_some_and(|v| v.is_null())
            || k.downcast_ref::<Territory>().is_some_and(|v| v.is_null())
            || k.downcast_ref::<TreacheryCard>().is_some_and(|v| v.is_null())
            || k.downcast_ref::<Leader>().is_some_and(|v| v.is_null())
            || k.downcast_ref::<Hero>().is_some_and(|v| v.is_null())
            || k.downcast_ref::<ResourceCard>().is_some_and(|v| v.is_null())
    }
    dotnet_compat::dictionary::set_null_key_check(is_null_key);
}

/// `list.Remove(item)` on a `List<T>` of reference type: removes the first
/// element equal to `item` (null matches null). `None` is C# null.
pub fn list_remove<T: CsNull>(list: &mut Vec<T>, item: Option<T>) -> bool {
    let item = item.or_null();
    match list.iter().position(|c| *c == item) {
        Some(i) => {
            list.remove(i);
            true
        }
        None => false,
    }
}

pub const ARRAY_INDEX_OUT_OF_RANGE_MESSAGE: &str = "Index was outside the bounds of the array.";

/// `list[i]` on a `List<T>`: ArgumentOutOfRangeException when out of range.
#[track_caller]
pub fn index<T>(list: &[T], i: i32) -> &T {
    if i < 0 || i as usize >= list.len() {
        throw("ArgumentOutOfRangeException", INDEX_OUT_OF_RANGE_MESSAGE.to_string())
    }
    &list[i as usize]
}

/// `list[i] = x` on a `List<T>`: same exception as reading.
#[track_caller]
pub fn index_mut<T>(list: &mut [T], i: i32) -> &mut T {
    if i < 0 || i as usize >= list.len() {
        throw("ArgumentOutOfRangeException", INDEX_OUT_OF_RANGE_MESSAGE.to_string())
    }
    &mut list[i as usize]
}

/// `array[i]` on a C# array (`T[]`, including `string.Split` results):
/// IndexOutOfRangeException "Index was outside the bounds of the array.".
/// Use [`index`] for `List<T>`, which throws a different exception.
#[track_caller]
pub fn index_array<T>(array: &[T], i: i32) -> &T {
    if i < 0 || i as usize >= array.len() {
        throw("IndexOutOfRangeException", ARRAY_INDEX_OUT_OF_RANGE_MESSAGE.to_string())
    }
    &array[i as usize]
}

/// `array[i] = x` on a C# array.
#[track_caller]
pub fn index_array_mut<T>(array: &mut [T], i: i32) -> &mut T {
    if i < 0 || i as usize >= array.len() {
        throw("IndexOutOfRangeException", ARRAY_INDEX_OUT_OF_RANGE_MESSAGE.to_string())
    }
    &mut array[i as usize]
}

/// `ElementAt(i)` on a `List<T>` or array: the list indexer's exception.
#[track_caller]
pub fn element_at_list<T>(list: &[T], i: i32) -> &T {
    index(list, i)
}

/// `Last(predicate)` on a `List<T>` or array. .NET walks an `IList<T>`
/// backwards and stops at the first match, so the predicate runs on fewer
/// elements (in reverse order) than for a lazy sequence; see
/// [`CsIterExt::cs_last_where`] for that case.
#[track_caller]
pub fn last_where_list<T>(list: &[T], mut predicate: impl FnMut(&T) -> bool) -> &T {
    match list.iter().rev().find(|v| predicate(v)) {
        Some(v) => v,
        None => throw("InvalidOperationException", NO_MATCH.to_string()),
    }
}

/// `LastOrDefault(predicate)` on a `List<T>` or array (walks backwards).
pub fn last_or_default_where_list<T>(list: &[T], mut predicate: impl FnMut(&T) -> bool) -> Option<&T> {
    null_to_none(list.iter().rev().find(|v| predicate(v)))
}

// Exception messages of the LINQ operators.
pub const NO_ELEMENTS: &str = "Sequence contains no elements";
pub const NO_MATCH: &str = "Sequence contains no matching element";
pub const MORE_THAN_ONE_ELEMENT: &str = "Sequence contains more than one element";
pub const MORE_THAN_ONE_MATCH: &str = "Sequence contains more than one matching element";

/// The message of `new T()` for a .NET exception type: what C# `throw new
/// NullReferenceException()` carries. Unknown types get the generic
/// `Exception` pattern, which is what .NET uses for types without their own
/// resource string.
pub fn default_message(type_name: &str) -> String {
    match type_name {
        "InvalidOperationException" => "Operation is not valid due to the current state of the object.".into(),
        "ArgumentException" => "Value does not fall within the expected range.".into(),
        "ArgumentOutOfRangeException" => "Specified argument was out of the range of valid values.".into(),
        "ArgumentNullException" => "Value cannot be null.".into(),
        "NullReferenceException" => NULL_REFERENCE_MESSAGE.into(),
        "KeyNotFoundException" => "The given key was not present in the dictionary.".into(),
        "IndexOutOfRangeException" => ARRAY_INDEX_OUT_OF_RANGE_MESSAGE.into(),
        "DivideByZeroException" => "Attempted to divide by zero.".into(),
        "OverflowException" => "Arithmetic operation resulted in an overflow.".into(),
        "FormatException" => "One of the identified items was in an invalid format.".into(),
        "NotImplementedException" => "The method or operation is not implemented.".into(),
        "NotSupportedException" => "Specified method is not supported.".into(),
        _ => format!("Exception of type '{}' was thrown.", full_type_name(type_name)),
    }
}

/// The namespace-qualified name of a .NET exception type, as
/// `Exception.ToString()` prints it.
pub fn full_type_name(type_name: &str) -> String {
    match type_name {
        "KeyNotFoundException" => "System.Collections.Generic.KeyNotFoundException".into(),
        t => format!("System.{t}"),
    }
}

/// `e.ToString()` of an exception that was never thrown (no stack trace):
/// "System.InvalidOperationException: message", or just the type name when
/// the message is empty. A thrown exception adds a stack trace that Rust
/// cannot reproduce.
pub fn exception_to_string(e: &CsException) -> String {
    let name = full_type_name(e.type_name);
    if e.message.is_empty() { name } else { format!("{name}: {}", e.message) }
}

/// The message of an `ArgumentException` (or subclass) built with a
/// parameter name: `new ArgumentException("bad", "x")` says "bad (Parameter 'x')".
pub fn with_parameter(message: &str, param_name: &str) -> String {
    format!("{message} (Parameter '{param_name}')")
}

/// `new ArgumentOutOfRangeException(paramName, actualValue, null)`.
/// `actual_value` is the C# `actualValue.ToString()` (an enum's name, or its
/// number when undefined).
#[track_caller]
pub fn argument_out_of_range_actual(param_name: &str, actual_value: &str) -> ! {
    throw(
        "ArgumentOutOfRangeException",
        format!(
            "{}\nActual value was {actual_value}.",
            with_parameter("Specified argument was out of the range of valid values.", param_name)
        ),
    )
}

/// The LINQ operators that throw on empty or ambiguous input. Use these
/// instead of `next().unwrap()` wherever C# calls `First()`/`Single()` so the
/// exception (and not a Rust panic) reaches the replayer.
/// `LastOrDefault(predicate)` without mapping a null element to `None`.
fn last_where_raw<I: Iterator>(it: I, mut predicate: impl FnMut(&I::Item) -> bool) -> Option<I::Item> {
    let mut found = None;
    for v in it {
        if predicate(&v) {
            found = Some(v);
        }
    }
    found
}

/// `SingleOrDefault(predicate)` without mapping a null element to `None`.
#[track_caller]
fn single_where_raw<I: Iterator>(it: I, mut predicate: impl FnMut(&I::Item) -> bool) -> Option<I::Item> {
    let mut found = None;
    for v in it {
        if predicate(&v) {
            if found.is_some() {
                throw("InvalidOperationException", MORE_THAN_ONE_MATCH.to_string())
            }
            found = Some(v);
        }
    }
    found
}

pub trait CsIterExt: Iterator + Sized {
    /// `First()`
    #[track_caller]
    fn cs_first(mut self) -> Self::Item {
        match self.next() {
            Some(v) => v,
            None => throw("InvalidOperationException", NO_ELEMENTS.to_string()),
        }
    }

    /// `First(predicate)`
    #[track_caller]
    fn cs_first_where(mut self, predicate: impl FnMut(&Self::Item) -> bool) -> Self::Item {
        match self.find(predicate) {
            Some(v) => v,
            None => throw("InvalidOperationException", NO_MATCH.to_string()),
        }
    }

    /// `FirstOrDefault(predicate)`: the same as `find`, listed so that the
    /// LINQ table test covers it.
    fn cs_first_or_default_where(mut self, predicate: impl FnMut(&Self::Item) -> bool) -> Option<Self::Item> {
        null_to_none(self.find(predicate))
    }

    /// `Last()` on a lazy sequence. On a `List<T>` use `.last()` of the slice.
    #[track_caller]
    fn cs_last(self) -> Self::Item {
        match self.last() {
            Some(v) => v,
            None => throw("InvalidOperationException", NO_ELEMENTS.to_string()),
        }
    }

    /// `Last(predicate)` on a lazy sequence: the predicate runs on every
    /// element, front to back. For a `List<T>` or array use
    /// [`last_where_list`], which walks backwards like .NET does.
    #[track_caller]
    fn cs_last_where(self, predicate: impl FnMut(&Self::Item) -> bool) -> Self::Item {
        match last_where_raw(self, predicate) {
            Some(v) => v,
            None => throw("InvalidOperationException", NO_MATCH.to_string()),
        }
    }

    /// `LastOrDefault(predicate)` on a lazy sequence (every element is tested).
    fn cs_last_or_default_where(self, predicate: impl FnMut(&Self::Item) -> bool) -> Option<Self::Item> {
        null_to_none(last_where_raw(self, predicate))
    }

    /// `Single()`
    #[track_caller]
    fn cs_single(mut self) -> Self::Item {
        let Some(v) = self.next() else {
            throw("InvalidOperationException", NO_ELEMENTS.to_string())
        };
        if self.next().is_some() {
            throw("InvalidOperationException", MORE_THAN_ONE_ELEMENT.to_string())
        }
        v
    }

    /// `SingleOrDefault()`: `None` when empty, throws when there is more than one.
    #[track_caller]
    fn cs_single_or_default(mut self) -> Option<Self::Item> {
        let v = self.next()?;
        if self.next().is_some() {
            throw("InvalidOperationException", MORE_THAN_ONE_ELEMENT.to_string())
        }
        null_to_none(Some(v))
    }

    /// `Single(predicate)`: "no matching element" when nothing matches,
    /// "more than one matching element" at the second match. Do not port it
    /// as `filter(p).cs_single()`: that throws the messages of `Single()`.
    #[track_caller]
    fn cs_single_where(self, predicate: impl FnMut(&Self::Item) -> bool) -> Self::Item {
        match single_where_raw(self, predicate) {
            Some(v) => v,
            None => throw("InvalidOperationException", NO_MATCH.to_string()),
        }
    }

    /// `SingleOrDefault(predicate)`: `None` when nothing matches, throws
    /// "more than one matching element" at the second match. The predicate
    /// runs on every element up to the second match, as in .NET.
    #[track_caller]
    fn cs_single_or_default_where(self, predicate: impl FnMut(&Self::Item) -> bool) -> Option<Self::Item> {
        null_to_none(single_where_raw(self, predicate))
    }

    /// `ElementAt(index)` on a lazy sequence. For a `List<T>` or array use
    /// [`element_at_list`]: the exception message differs. Careful: .NET
    /// turns `Where` over an empty array into an empty array, so
    /// `emptyArray.Where(p).ElementAt(i)` throws the list message.
    #[track_caller]
    fn cs_element_at(mut self, index: i32) -> Self::Item {
        let out_of_range = || -> ! {
            throw(
                "ArgumentOutOfRangeException",
                "Specified argument was out of the range of valid values. (Parameter 'index')".to_string(),
            )
        };
        if index < 0 {
            out_of_range()
        }
        match self.nth(index as usize) {
            Some(v) => v,
            None => out_of_range(),
        }
    }

    /// `Max()` of a non-nullable value type: throws when empty. Returns the
    /// first maximal element (.NET keeps the first on ties; for plain numbers
    /// the distinction is invisible).
    #[track_caller]
    fn cs_max(self) -> Self::Item
    where
        Self::Item: Ord,
    {
        let mut it = self;
        let Some(mut best) = it.next() else {
            throw("InvalidOperationException", NO_ELEMENTS.to_string())
        };
        for v in it {
            if v > best {
                best = v;
            }
        }
        best
    }

    /// `Min()` of a non-nullable value type: throws when empty.
    #[track_caller]
    fn cs_min(self) -> Self::Item
    where
        Self::Item: Ord,
    {
        let mut it = self;
        let Some(mut best) = it.next() else {
            throw("InvalidOperationException", NO_ELEMENTS.to_string())
        };
        for v in it {
            if v < best {
                best = v;
            }
        }
        best
    }

    /// `Distinct()`: first occurrence wins, order kept. Uses `==` (C# `Equals`).
    fn cs_distinct(self) -> Vec<Self::Item>
    where
        Self::Item: PartialEq,
    {
        let mut out: Vec<Self::Item> = Vec::new();
        for v in self {
            if !out.contains(&v) {
                out.push(v);
            }
        }
        out
    }
}

impl<I: Iterator> CsIterExt for I {}

/// `a / b` on C# `int`: DivideByZeroException when `b == 0`,
/// OverflowException for `int.MinValue / -1`. Truncates toward zero like Rust.
#[track_caller]
pub fn div(a: i32, b: i32) -> i32 {
    if b == 0 {
        throw("DivideByZeroException", "Attempted to divide by zero.".to_string())
    }
    if a == i32::MIN && b == -1 {
        throw("OverflowException", "Arithmetic operation resulted in an overflow.".to_string())
    }
    a / b
}

/// `a % b` on C# `int`: DivideByZeroException when `b == 0`. On x64,
/// `int.MinValue % -1` throws OverflowException.
#[track_caller]
pub fn rem(a: i32, b: i32) -> i32 {
    if b == 0 {
        throw("DivideByZeroException", "Attempted to divide by zero.".to_string())
    }
    if a == i32::MIN && b == -1 {
        throw("OverflowException", "Arithmetic operation resulted in an overflow.".to_string())
    }
    a % b
}

/// `dict[key]` read of a missing key. `key_text` is C# `key.ToString()`.
#[track_caller]
pub fn key_not_found(key_text: &str) -> ! {
    throw("KeyNotFoundException", format!("The given key '{key_text}' was not present in the dictionary."))
}

/// A C# reference to a mutable object that more than one owner holds (an
/// event stored in game state, a list shared between an event and the game).
/// Cloning shares the object. Equality is reference equality.
pub struct Shared<T>(Rc<RefCell<T>>);

impl<T> Shared<T> {
    pub fn new(value: T) -> Self {
        Shared(Rc::new(RefCell::new(value)))
    }

    #[track_caller]
    pub fn borrow(&self) -> Ref<'_, T> {
        self.0.borrow()
    }

    #[track_caller]
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        self.0.borrow_mut()
    }

    /// C# `ReferenceEquals`.
    pub fn ptr_eq(&self, other: &Shared<T>) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared(Rc::clone(&self.0))
    }
}

/// Reference equality, like `==` on a C# class without an override.
impl<T> PartialEq for Shared<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl<T> Eq for Shared<T> {}

impl<T: std::fmt::Debug> std::fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.borrow().fmt(f)
    }
}

/// `string.Compare(a, b)` / `a.CompareTo(b)` under the invariant culture
/// (and under en-US, which ICU does not tailor): the comparison
/// `Comparer.Default` uses for string keys in `OrderBy`, `Max` and the
/// `HighestOrDefault` family.
///
/// .NET delegates to ICU's root collation. For ASCII text that is:
/// - control characters other than tab, line feed, vertical tab, form feed and
///   carriage return are ignored;
/// - every other character has a primary weight in the order of
///   [`COLLATION_ORDER`]: whitespace, punctuation, symbols, digits, then
///   letters, with each upper-case letter sharing the primary weight of its
///   lower-case form;
/// - strings are compared by their primary weights first and, only when those
///   are equal, by case, with lower case first at the first difference.
///
/// So `"a" < "B" < "b"`, `"a-b" < "ab"` and `"ab" < "Ab" < "ac"`. Distinct
/// strings can compare equal ("a\u{1}b" and "ab").
///
/// Limit: only ASCII is implemented. Enum names and the leader and card
/// names in the skin are ASCII. A non-ASCII character panics (a porting bug,
/// not a C# exception) rather than giving a wrong order silently. Checked
/// against .NET in tests/random_helpers.rs and tests/linq_table.rs.
pub fn compare_invariant(a: &str, b: &str) -> std::cmp::Ordering {
    fn weights(s: &str) -> Vec<(u8, bool)> {
        s.chars()
            .filter_map(|c| {
                assert!(c.is_ascii(), "cs::compare_invariant: culture comparison of non-ASCII text is not implemented ({s:?})");
                let lower = c.to_ascii_lowercase() as u8;
                let primary = COLLATION_ORDER.iter().position(|&x| x == lower)?;
                Some((primary as u8, c.is_ascii_uppercase()))
            })
            .collect()
    }
    let (wa, wb) = (weights(a), weights(b));
    let primary = wa.iter().map(|w| w.0).cmp(wb.iter().map(|w| w.0));
    primary.then_with(|| wa.iter().map(|w| w.1).cmp(wb.iter().map(|w| w.1)))
}

/// ASCII characters (lower-case letters only) in ICU root collation order,
/// as measured with .NET 10. Characters missing here are ignorable.
pub const COLLATION_ORDER: &[u8] =
    b"\t\n\x0b\x0c\r _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$0123456789abcdefghijklmnopqrstuvwxyz";

/// A key that a C# selector returns as `IComparable` (`Func<T, IComparable>`
/// in the `HighestOrDefault` family, or an `OrderBy` key). C# compares the
/// boxed keys with `Comparer.Default` and tests them with `object.Equals`;
/// these two can disagree (strings compare by culture but are equal only
/// ordinally), so a key provides both.
pub trait CsComparable {
    /// `Comparer.Default.Compare(self, other)`.
    fn cs_compare(&self, other: &Self) -> std::cmp::Ordering;
    /// `self.Equals(other)` on the boxed values.
    fn cs_equals(&self, other: &Self) -> bool;
}

macro_rules! cs_comparable_ord {
    ($($t:ty),*) => {$(
        impl CsComparable for $t {
            fn cs_compare(&self, other: &Self) -> std::cmp::Ordering { self.cmp(other) }
            fn cs_equals(&self, other: &Self) -> bool { self == other }
        }
    )*};
}
cs_comparable_ord!(i32, i64, bool);

/// `float.CompareTo`: NaN is smaller than everything and equal to itself;
/// `Equals` treats NaN as equal to NaN and -0 as equal to 0.
impl CsComparable for f32 {
    fn cs_compare(&self, other: &Self) -> std::cmp::Ordering {
        dotnet_compat::math::cmp_f32(*self, *other)
    }
    fn cs_equals(&self, other: &Self) -> bool {
        dotnet_compat::math::equals_f32(*self, *other)
    }
}

impl CsComparable for f64 {
    fn cs_compare(&self, other: &Self) -> std::cmp::Ordering {
        dotnet_compat::math::cmp_f64(*self, *other)
    }
    fn cs_equals(&self, other: &Self) -> bool {
        dotnet_compat::math::equals_f64(*self, *other)
    }
}

/// Culture comparison, ordinal equality. See [`compare_invariant`].
impl CsComparable for String {
    fn cs_compare(&self, other: &Self) -> std::cmp::Ordering {
        compare_invariant(self, other)
    }
    fn cs_equals(&self, other: &Self) -> bool {
        self == other
    }
}

impl CsComparable for &str {
    fn cs_compare(&self, other: &Self) -> std::cmp::Ordering {
        compare_invariant(self, other)
    }
    fn cs_equals(&self, other: &Self) -> bool {
        self == other
    }
}

impl<T: Default> Default for Shared<T> {
    fn default() -> Self {
        Shared::new(T::default())
    }
}

/// An `Rc` used as a dictionary key or set element with C# reference
/// semantics: equality and hashing by address, not by contents.
pub struct RcKey<T>(pub Rc<T>);

impl<T> Clone for RcKey<T> {
    fn clone(&self) -> Self {
        RcKey(Rc::clone(&self.0))
    }
}

impl<T> PartialEq for RcKey<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Eq for RcKey<T> {}

impl<T> std::hash::Hash for RcKey<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (Rc::as_ptr(&self.0) as *const () as usize).hash(state)
    }
}

impl<T> std::fmt::Debug for RcKey<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RcKey({:p})", Rc::as_ptr(&self.0))
    }
}
