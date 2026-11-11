using System.Buffers;
using System.Buffers.Text;
using System.Globalization;

namespace Treachery.Oracle.Canon;

/// <summary>
/// Minimal JSON writer producing the canonical byte format described in rust/docs/trace-format.md.
/// No whitespace anywhere. Strings are escaped by <see cref="WriteString"/>'s rules only.
/// The caller is responsible for commas: use <see cref="Comma"/> / <see cref="Key"/>.
/// </summary>
public sealed class JsonOut
{
    private readonly ArrayBufferWriter<byte> _buf;

    public JsonOut(int initialCapacity = 1 << 16)
    {
        _buf = new ArrayBufferWriter<byte>(initialCapacity);
    }

    public int Length => _buf.WrittenCount;

    public ReadOnlySpan<byte> WrittenSpan => _buf.WrittenSpan;

    public void Clear() => _buf.Clear();

    public void Raw(byte b)
    {
        var span = _buf.GetSpan(1);
        span[0] = b;
        _buf.Advance(1);
    }

    public void Raw(ReadOnlySpan<byte> bytes)
    {
        bytes.CopyTo(_buf.GetSpan(bytes.Length));
        _buf.Advance(bytes.Length);
    }

    /// <summary>Writes already-valid JSON text verbatim (UTF-8 encoded).</summary>
    public void RawJson(string json)
    {
        var max = Encoding.UTF8.GetMaxByteCount(json.Length);
        var span = _buf.GetSpan(max);
        var n = Encoding.UTF8.GetBytes(json, span);
        _buf.Advance(n);
    }

    public void ObjStart() => Raw((byte)'{');
    public void ObjEnd() => Raw((byte)'}');
    public void ArrStart() => Raw((byte)'[');
    public void ArrEnd() => Raw((byte)']');
    public void Comma() => Raw((byte)',');
    public void Null() => Raw("null"u8);
    public void Bool(bool b) => Raw(b ? "true"u8 : "false"u8);

    /// <summary>Writes "key": (key is escaped like any string).</summary>
    public void Key(string key)
    {
        WriteString(key);
        Raw((byte)':');
    }

    public void Int(long v)
    {
        var span = _buf.GetSpan(24);
        Utf8Formatter.TryFormat(v, span, out var n);
        _buf.Advance(n);
    }

    public void UInt(ulong v)
    {
        var span = _buf.GetSpan(24);
        Utf8Formatter.TryFormat(v, span, out var n);
        _buf.Advance(n);
    }

    public void Double(double d)
    {
        if (double.IsNaN(d) || double.IsInfinity(d))
        {
            WriteString(d.ToString("R", CultureInfo.InvariantCulture));
            return;
        }

        RawAscii(d.ToString("R", CultureInfo.InvariantCulture));
    }

    public void Single(float f)
    {
        if (float.IsNaN(f) || float.IsInfinity(f))
        {
            WriteString(f.ToString("R", CultureInfo.InvariantCulture));
            return;
        }

        RawAscii(f.ToString("R", CultureInfo.InvariantCulture));
    }

    private void RawAscii(string s)
    {
        var span = _buf.GetSpan(s.Length);
        for (var i = 0; i < s.Length; i++) span[i] = (byte)s[i];
        _buf.Advance(s.Length);
    }

    private static readonly byte[] Hex = "0123456789abcdef"u8.ToArray();

    /// <summary>
    /// Canonical string escaping: '"' and '\' are backslash-escaped; \b \f \n \r \t use their short forms;
    /// other code units below U+0020 and lone surrogates are written as \u followed by four lowercase hex digits;
    /// everything else is written as raw UTF-8.
    /// </summary>
    public void WriteString(string? s)
    {
        if (s == null)
        {
            Null();
            return;
        }

        Raw((byte)'"');
        var start = 0;
        for (var i = 0; i < s.Length; i++)
        {
            var c = s[i];
            var needsEscape = c < 0x20 || c == '"' || c == '\\';
            var loneSurrogate = false;
            if (char.IsHighSurrogate(c))
            {
                if (i + 1 < s.Length && char.IsLowSurrogate(s[i + 1]))
                {
                    i++;
                    continue;
                }

                loneSurrogate = true;
            }
            else if (char.IsLowSurrogate(c))
            {
                loneSurrogate = true;
            }

            if (!needsEscape && !loneSurrogate) continue;

            if (i > start) Utf8Chunk(s.AsSpan(start, i - start));
            start = i + 1;

            switch (c)
            {
                case '"': Raw("\\\""u8); break;
                case '\\': Raw("\\\\"u8); break;
                case '\b': Raw("\\b"u8); break;
                case '\f': Raw("\\f"u8); break;
                case '\n': Raw("\\n"u8); break;
                case '\r': Raw("\\r"u8); break;
                case '\t': Raw("\\t"u8); break;
                default:
                    var span = _buf.GetSpan(6);
                    span[0] = (byte)'\\';
                    span[1] = (byte)'u';
                    span[2] = Hex[(c >> 12) & 0xF];
                    span[3] = Hex[(c >> 8) & 0xF];
                    span[4] = Hex[(c >> 4) & 0xF];
                    span[5] = Hex[c & 0xF];
                    _buf.Advance(6);
                    break;
            }
        }

        if (start < s.Length) Utf8Chunk(s.AsSpan(start));
        Raw((byte)'"');
    }

    private void Utf8Chunk(ReadOnlySpan<char> chars)
    {
        var max = Encoding.UTF8.GetMaxByteCount(chars.Length);
        var span = _buf.GetSpan(max);
        var n = Encoding.UTF8.GetBytes(chars, span);
        _buf.Advance(n);
    }

    public string ToUtf8String() => Encoding.UTF8.GetString(_buf.WrittenSpan);
}
