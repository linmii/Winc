// Sort object keys recursively without converting JSON numbers to floating point.
// Parse bytes through `this` so recursive calls share one cursor; arrays keep their order.
fn fail(message) {
    // Compute source coordinates only on failure; count UTF-8 characters and CRLF as one newline.
    let line = 1;
    let column = 1;
    let previous = -1;
    let end = if this.pos < this.data.len { this.pos } else { this.data.len };
    for i in 0..end {
        let byte = this.data[i];
        if byte == 13 {
            line += 1;
            column = 1;
        } else if byte == 10 {
            if previous != 13 { line += 1; }
            column = 1;
        } else if (byte & 192) != 128 {
            column += 1;
        }
        previous = byte;
    }
    throw "JSON 格式错误 / Invalid JSON: " + message
        + " (第 " + line + " 行，第 " + column + " 列 / line " + line
        + ", column " + column + "; byte " + (end + 1) + ")";
}

fn peek() {
    if this.pos < this.data.len { this.data[this.pos] } else { -1 }
}

fn skip_space() {
    while this.peek() in [32, 9, 10, 13] { this.pos += 1; }
}

// Keep comments as tokens, never search/replace inside string values (URLs can contain //).
// Same-line comments belong to the previous value; standalone comments lead the next one.
fn read_comments(inline_only) {
    let comments = [];
    loop {
        while this.peek() in [32, 9, 10, 13] {
            if inline_only && this.peek() in [10, 13] { return comments; }
            this.pos += 1;
        }
        if this.peek() != 47 { return comments; }
        let start = this.pos;
        this.pos += 1;
        let kind = this.peek();
        this.pos += 1;
        if kind == 47 {
            while this.peek() >= 0 && !(this.peek() in [10, 13]) { this.pos += 1; }
        } else if kind == 42 {
            loop {
                let byte = this.peek();
                if byte < 0 {
                    this.pos = start;
                    this.fail("块注释未结束 / unterminated block comment");
                }
                this.pos += 1;
                if byte == 42 && this.peek() == 47 { this.pos += 1; break; }
            }
        } else {
            this.pos = start;
            this.fail("无效注释 / expected // or /* comment");
        }
        let comment = "" + this.data.extract(start, this.pos - start);
        let multiline = comment.contains("\n") || comment.contains("\r");
        comment.replace("\r\n", "\n");
        comment.replace("\r", "\n");
        comment.replace("\n", this.eol);
        comments.push(comment);
        if inline_only && (kind == 47 || multiline) { return comments; }
    }
}

fn comment_lines(comments, pad, eol) {
    let output = "";
    for comment in comments {
        if output != "" { output += eol; }
        output += pad + comment;
    }
    output
}

fn comments_between(comments, pad, eol) {
    let output = "";
    for comment in comments {
        output += comment;
        // A line comment must end before the next key, colon or value is emitted.
        output += if comment.starts_with("//") { eol + pad } else { " " };
    }
    output
}

fn comments_after(comments, pad, eol) {
    let output = "";
    let new_line = false;
    for comment in comments {
        output += if new_line { eol + pad } else { " " };
        output += comment;
        new_line = comment.starts_with("//");
    }
    output
}

fn expect(value) {
    if this.peek() != value {
        let message = switch value {
            34 => "对象键名必须使用双引号 / expected a double-quoted object key",
            58 => "键名后缺少冒号 / expected ':' after object key",
            44 => "元素之间缺少逗号 / expected ',' between items",
            _ => "unexpected character or end of input"
        };
        this.fail(message);
    }
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
        let leading = this.read_comments(false);
        let child_pad = pad + this.indent;
        if this.peek() == close {
            this.pos += 1;
            if leading.len > 0 {
                return open_text + this.eol + comment_lines(leading, child_pad, this.eol)
                    + this.eol + pad + close_text;
            }
            return open_text + close_text;
        }
        let items = [];
        let keys = [];
        let members = #{};
        let footer = [];
        loop {
            let key = [];
            let body = "";
            if object {
                key = this.read_string();
                // Refuse duplicate decoded keys instead of silently discarding a member.
                if members.contains(key[1]) { this.fail("duplicate object key: " + key[0]); }
                let key_comments = this.read_comments(false);
                this.expect(58);
                let value_comments = this.read_comments(false);
                let value = this.read_value(depth + 1, child_pad);
                body = key[0];
                if key_comments.len > 0 { body += " " + comments_between(key_comments, child_pad, this.eol); }
                body += ": " + comments_between(value_comments, child_pad, this.eol) + value;
            } else {
                body = this.read_value(depth + 1, child_pad);
            }
            let tail = this.read_comments(true);
            let after = this.read_comments(false);
            let done = this.peek() == close;
            let next = [];
            if done {
                footer = after;
                after = [];
            } else {
                this.expect(44);
                tail += this.read_comments(true);
                next = this.read_comments(false);
                // Accept a single trailing comma and omit it when formatting the container.
                if this.peek() == close { footer = next; done = true; }
            }
            let item = #{ leading: leading, body: body, tail: tail, after: after };
            if object {
                members[key[1]] = item;
                keys.push(key[1]);
            } else { items.push(item); }
            if done { this.pos += 1; break; }
            leading = next;
        }
        if object {
            // Case-sensitive Unicode order: A-Z before a-z; escaped keys use their decoded value.
            keys.sort();
            for key in keys { items.push(members[key]); }
        }
        let output = open_text + this.eol;
        for i in 0..items.len {
            let item = items[i];
            if item.leading.len > 0 { output += comment_lines(item.leading, child_pad, this.eol) + this.eol; }
            output += child_pad + item.body;
            // Emit punctuation before line comments so sorting cannot hide a required comma.
            if i + 1 < items.len { output += ","; }
            output += comments_after(item.tail, child_pad, this.eol);
            if item.after.len > 0 { output += this.eol + comment_lines(item.after, child_pad, this.eol); }
            output += this.eol;
        }
        if footer.len > 0 { output += comment_lines(footer, child_pad, this.eol) + this.eol; }
        return output + pad + close_text;
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
let header = parser.read_comments(false);
let result = "";
if header.len > 0 { result += comment_lines(header, "", line_ending) + line_ending; }
result += parser.read_value(0, "");
result += comments_after(parser.read_comments(true), "", line_ending);
let footer = parser.read_comments(false);
if footer.len > 0 { result += line_ending + comment_lines(footer, "", line_ending); }
if parser.pos != parser.data.len { parser.fail("extra content after JSON value"); }
// Preserve whether the selected text/document ended with a newline, using its current EOL style.
if text.ends_with("\n") || text.ends_with("\r") { result += line_ending; }
result
