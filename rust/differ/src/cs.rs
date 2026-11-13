//! .NET exception names in panic messages, which the C# traces record. The engine no longer
//! panics with .NET exception texts, so only panics from dotnet-compat (parsing, dictionaries)
//! still carry one.

pub use dotnet_compat::exception::CsException;
use treachery_engine::error::panic_message;

/// The .NET exception a panic payload names, if it names one.
pub fn exception_of(payload: &(dyn std::any::Any + Send)) -> Option<CsException> {
    const TYPES: &[&str] = &[
        "Exception", "InvalidOperationException", "ArgumentException", "ArgumentOutOfRangeException", "ArgumentNullException",
        "NullReferenceException", "KeyNotFoundException", "IndexOutOfRangeException", "DivideByZeroException", "OverflowException",
        "FormatException", "InvalidCastException", "NotSupportedException", "NotImplementedException", "JsonException",
    ];
    let text = panic_message(payload);
    let (type_name, message) = text.split_once(": ")?;
    let type_name = TYPES.iter().copied().find(|t| *t == type_name)?;
    Some(CsException { type_name, message: message.to_string() })
}

/// The namespace-qualified name of a .NET exception type, as `Exception.ToString()` prints it.
pub fn full_type_name(type_name: &str) -> String {
    match type_name {
        "KeyNotFoundException" => "System.Collections.Generic.KeyNotFoundException".into(),
        t => format!("System.{t}"),
    }
}
