// Read only declared, bundled dictionaries. No network or external runtime is required.
// Dictionary data: OpenCC 1.1.9 (Apache-2.0), https://github.com/BYVoid/OpenCC/tree/ver.1.1.9/data/dictionary
// See dictionaries/LICENSE. The longest-match conversion below is independently implemented.
let prefix = if command == "to-traditional" { "ST" }
    else if command == "to-simplified" { "TS" }
    else {
    throw "未知的简繁转换命令 / Unknown Chinese conversion command";
};
if text == "" { return text; }
let entries = #{};
let lengths = #{};
let max_length = 1;
for suffix in ["Characters", "Phrases"] {
    for row in read_resource("dictionaries/" + prefix + suffix + ".txt").split("\n") {
        row.trim();
        if row == "" || row.starts_with("#") { continue; }
        let columns = row.split("\t");
        if columns.len != 2 {
            throw "简繁字典格式错误 / Invalid Chinese dictionary";
        }
        let key = columns[0];
        let candidates = columns[1].split(" ");
        if key == "" || candidates[0] == "" {
            throw "简繁字典包含空词条 / Empty Chinese dictionary entry";
        }
        entries[key] = candidates[0];
        let first = key.sub_string(0, 1);
        let length = key.len;
        let previous = lengths[first];
        if previous == () || previous < length {
            lengths[first] = length;
        }
        if max_length < length { max_length = length; }
    }
}

// A small lookahead buffer avoids repeatedly scanning a long UTF-8 string by character index.
// NUL padding flushes the buffer; consumed/total ensures padding never enters the result.
let result = "";
let buffer = "";
let consumed = 0;
let total = text.len;
let padding = "";
padding.pad(max_length, '\u0000');
for chunk in [text, padding] {
    for character in chunk {
        if consumed >= total { break; }
        buffer += character;
        if buffer.len < max_length { continue; }
        let first = buffer.sub_string(0, 1);
        let limit = lengths[first];
        let count = if limit == () { 1 } else { limit };
        let replacement = first;
        // Longest phrase wins, then a single-character mapping, then unchanged text.
        loop {
            let key = buffer.sub_string(0, count);
            let value = entries[key];
            if value != () {
                replacement = value;
                break;
            }
            if count == 1 { break; }
            count -= 1;
        }
        result += replacement;
        buffer = buffer.sub_string(count);
        consumed += count;
    }
}
result
