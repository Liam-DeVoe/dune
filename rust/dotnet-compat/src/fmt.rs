//! Number formatting with .NET's `ToString()` under the invariant culture.
//! Same functions as `math::format_f32`/`math::format_f64`.

/// `float.ToString()` (invariant culture): shortest round-trip digits,
/// `"1E+09"`/`"1E-05"` style exponents, `"Infinity"`, `"NaN"`, `"-0"`.
pub fn f32_to_string(x: f32) -> String {
    crate::math::format_f32(x)
}

/// `double.ToString()` (invariant culture); see [`f32_to_string`].
pub fn f64_to_string(x: f64) -> String {
    crate::math::format_f64(x)
}
