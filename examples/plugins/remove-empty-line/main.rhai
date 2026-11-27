// Normalize line endings only; retain the original content of nonblank lines.
let normalized = text;
normalized.replace("\r\n", "\n");
normalized.replace("\r", "\n");
let trailing = normalized.ends_with("\n");
let result = "";
for line in normalized.split("\n") {
    // Trim a copy to detect whitespace-only lines without changing indentation.
    let content = line;
    content.trim();
    if content == "" { continue; }
    if result != "" { result += line_ending; }
    result += line;
}
// An empty document already displays one blank line; never add a newline to it.
if trailing && result != "" { result += line_ending; }
result
