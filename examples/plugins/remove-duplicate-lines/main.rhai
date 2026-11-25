// Preserve the first occurrence, its whitespace and the final newline for nonempty output.
let normalized = text;
normalized.replace("\r\n", "\n");
normalized.replace("\r", "\n");
let trailing = normalized.ends_with("\n");
let lines = normalized.split("\n");
if trailing { lines.pop(); }
let seen = #{};
let output = [];
for line in lines {
    let key = line;
    if options.trim_whitespace { key.trim(); }
    if options.ignore_case { key = key.to_lower(); }
    if !seen.contains(key) {
        seen[key] = true;
        output.push(line);
    }
}
let result = "";
let first = true;
for line in output {
    if !first { result += line_ending; }
    first = false;
    result += line;
}
// Empty text already displays as one blank line; a newline would create a second one.
if trailing && result != "" { result += line_ending; }
result
