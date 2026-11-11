using System.Security.Cryptography;

namespace Treachery.ServerOracle;

/// <summary>
/// A minimal JSON text builder for transcript lines. Strings are escaped exactly as System.Text.Json's default
/// encoder does (the encoder of Utilities.Serialize), and serialized values are inserted verbatim, so the Rust
/// runner (rust/server-equiv/src/transcript.rs) can produce byte-identical lines.
/// </summary>
public sealed class JsonText
{
    private readonly StringBuilder _sb = new();
    private readonly Stack<bool> _first = new();
    private bool _pendingValue;

    public override string ToString() => _sb.ToString();

    private void BeforeValue()
    {
        if (_pendingValue)
        {
            _pendingValue = false;
            return;
        }

        if (_first.Count > 0)
        {
            if (!_first.Peek()) _sb.Append(',');
            _first.Pop();
            _first.Push(false);
        }
    }

    public JsonText ObjStart()
    {
        BeforeValue();
        _sb.Append('{');
        _first.Push(true);
        return this;
    }

    public JsonText ObjEnd()
    {
        _first.Pop();
        _sb.Append('}');
        return this;
    }

    public JsonText ArrStart()
    {
        BeforeValue();
        _sb.Append('[');
        _first.Push(true);
        return this;
    }

    public JsonText ArrEnd()
    {
        _first.Pop();
        _sb.Append(']');
        return this;
    }

    public JsonText Key(string key)
    {
        BeforeValue();
        _sb.Append(Escape(key)).Append(':');
        _pendingValue = true;
        return this;
    }

    /// <summary>Inserts already serialized JSON.</summary>
    public JsonText Raw(string json)
    {
        BeforeValue();
        _sb.Append(json);
        return this;
    }

    public JsonText Str(string? s) => Raw(Escape(s));

    public JsonText Int(long i) => Raw(i.ToString(System.Globalization.CultureInfo.InvariantCulture));

    public JsonText Bool(bool b) => Raw(b ? "true" : "false");

    public JsonText Null() => Raw("null");

    /// <summary>Any value as Utilities.Serialize writes it.</summary>
    public JsonText Ser<T>(T value) => Raw(Utilities.Serialize(value));

    public JsonText Member(string key, string? s) => Key(key).Str(s);

    public JsonText MemberRaw(string key, string json) => Key(key).Raw(json);

    public JsonText MemberSer<T>(string key, T value) => Key(key).Ser(value);

    public static string Escape(string? s) => JsonSerializer.Serialize(s);

    public static string Sha256(string s) => Convert.ToHexStringLower(SHA256.HashData(Encoding.UTF8.GetBytes(s)));
}
