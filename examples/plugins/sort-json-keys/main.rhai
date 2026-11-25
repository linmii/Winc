// Sort object keys recursively without converting JSON numbers to floating point.
// Parse bytes through `this` so recursive calls share one cursor; arrays keep their order.
fn fail(message) {
    throw "JSON 格式错误 / Invalid JSON (byte " + (this.pos + 1) + "): " + message;
}

fn peek() {
    if this.pos < this.data.len { this.data[this.pos] } else { -1 }
}

fn skip_space() {
    while this.peek() in [32, 9, 10, 13] { this.pos += 1; }
}

fn expect(value) {
    if this.peek() != value { this.fail("unexpected character or end of input"); }
    this.pos += 1;
}

fn digit(value) { value >= 48 && value <= 57 }

fn hex4() {
    let value = 0;
    for unused in 0..4 {
        let byte = this.peek();
        let n = if digit(byte) { byte - 48 }
            else if byte >= 65 && byte <= 70 { byte - 55 }
            else if byte >= 97 && byte <= 102 { byte - 87 }
            else { this.fail("invalid Unicode escape"); 0 };
        value = value * 16 + n;
        this.pos += 1;
    }
    value
}

// Encode an escaped code point for key comparison; the original spelling is kept in output.
fn push_utf8(code) {
    if code < 128 {
        this.push(code);
    } else if code < 2048 {
        this.push(192 + (code >> 6));
        this.push(128 + (code & 63));
    } else if code < 65536 {
        this.push(224 + (code >> 12));
        this.push(128 + ((code >> 6) & 63));
        this.push(128 + (code & 63));
    } else {
        this.push(240 + (code >> 18));
        this.push(128 + ((code >> 12) & 63));
        this.push(128 + ((code >> 6) & 63));
        this.push(128 + (code & 63));
    }
}

fn read_string() {
    let start = this.pos;
    this.expect(34);
    let decoded = blob(0);
    loop {
        let byte = this.peek();
        if byte < 32 { this.fail("unterminated string or unescaped control character"); }
        this.pos += 1;
        if byte == 34 { break; }
        if byte != 92 {
            decoded.push(byte);
            continue;
        }
        let escape = this.peek();
        this.pos += 1;
        if escape == 117 {
            let code = this.hex4();
            if code >= 55296 && code <= 56319 {
                this.expect(92);
                this.expect(117);
                let low = this.hex4();
                if low < 56320 || low > 57343 { this.fail("invalid Unicode surrogate pair"); }
                code = 65536 + (code - 55296) * 1024 + low - 56320;
            } else if code >= 56320 && code <= 57343 {
                this.fail("unpaired Unicode surrogate");
            }
            decoded.push_utf8(code);
        } else {
            let value = switch escape {
                34 => 34, 92 => 92, 47 => 47,
                98 => 8, 102 => 12, 110 => 10, 114 => 13, 116 => 9,
                _ => { this.fail("invalid string escape"); 0 }
            };
            decoded.push(value);
        }
    }
    ["" + this.data.extract(start, this.pos - start), "" + decoded]
}

fn read_number() {
    let start = this.pos;
    if this.peek() == 45 { this.pos += 1; }
    if this.peek() == 48 {
        this.pos += 1;
    } else {
        if !digit(this.peek()) { this.fail("expected a number"); }
        while digit(this.peek()) { this.pos += 1; }
    }
    if this.peek() == 46 {
        this.pos += 1;
        if !digit(this.peek()) { this.fail("expected digits after decimal point"); }
        while digit(this.peek()) { this.pos += 1; }
    }
    if this.peek() in [69, 101] {
        this.pos += 1;
        if this.peek() in [43, 45] { this.pos += 1; }
        if !digit(this.peek()) { this.fail("expected exponent digits"); }
        while digit(this.peek()) { this.pos += 1; }
    }
    "" + this.data.extract(start, this.pos - start)
}

fn read_value(depth, pad) {
    // Leave room below the host's call limit for string validation and error reporting.
    if depth > 24 { this.fail("nesting exceeds 24 levels"); }
    this.skip_space();
    let first = this.peek();
    if first == 34 { return this.read_string()[0]; }
    if first == 45 || digit(first) { return this.read_number(); }
    if first == 123 || first == 91 {
        let object = first == 123;
        let close = if object { 125 } else { 93 };
        let open_text = if object { "{" } else { "[" };
        let close_text = if object { "}" } else { "]" };
        this.pos += 1;
        this.skip_space();
        if this.peek() == close {
            this.pos += 1;
            return open_text + close_text;
        }
        let items = [];
        let keys = [];
        let members = #{};
        let child_pad = pad + this.indent;
        loop {
            this.skip_space();
            if object {
                let key = this.read_string();
                // Refuse duplicate decoded keys instead of silently discarding a member.
                if members.contains(key[1]) { this.fail("duplicate object key: " + key[0]); }
                this.skip_space();
                this.expect(58);
                let value = this.read_value(depth + 1, child_pad);
                members[key[1]] = key[0] + ": " + value;
                keys.push(key[1]);
            } else {
                items.push(this.read_value(depth + 1, child_pad));
            }
            this.skip_space();
            if this.peek() == close { this.pos += 1; break; }
            this.expect(44);
        }
        if object {
            // Case-sensitive Unicode order: A-Z before a-z; escaped keys use their decoded value.
            keys.sort();
            for key in keys { items.push(members[key]); }
        }
        let output = open_text + this.eol;
        let first_item = true;
        for item in items {
            if !first_item { output += "," + this.eol; }
            first_item = false;
            output += child_pad + item;
        }
        return output + this.eol + pad + close_text;
    }
    let literal = switch first {
        116 => "true", 102 => "false", 110 => "null",
        _ => { this.fail("expected a JSON value"); "" }
    };
    if "" + this.data.extract(this.pos, literal.len) != literal {
        this.fail("invalid literal");
    }
    this.pos += literal.len;
    literal
}

let width = options.indent;
if type_of(width) != "i64" || width < 1 || width > 8 {
    throw "indent 必须是 1 到 8 的整数 / indent must be an integer from 1 to 8";
}
let indent = "";
for unused in 0..width { indent += " "; }
let parser = #{ data: text.to_blob(), pos: 0, indent: indent, eol: line_ending };
let result = parser.read_value(0, "");
parser.skip_space();
if parser.pos != parser.data.len { parser.fail("extra content after JSON value"); }
// Preserve whether the selected text/document ended with a newline, using its current EOL style.
if text.ends_with("\n") || text.ends_with("\r") { result += line_ending; }
result
