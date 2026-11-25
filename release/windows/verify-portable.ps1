param(
    [Parameter(Mandatory)][string]$Archive,
    [switch]$IncludePlugins
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead((Resolve-Path -LiteralPath $Archive).Path)
try {
    $files = @{}
    foreach ($entry in $zip.Entries) {
        $name = $entry.FullName.Replace('\', '/')
        if ($name.EndsWith('/')) { continue }
        if ($files.ContainsKey($name)) { throw "duplicate archive entry: $name" }
        $files[$name] = $entry
    }
    $required = @('winc.exe', 'winc-pty.exe', 'conpty.dll', 'OpenConsole.exe', 'winc-watch',
        'winc-server-Linux-x86_64', 'winc-server-Linux-aarch64', 'bin/winc', 'bin/winc.cmd',
        'data/README.txt', 'LICENSE', 'NOTICE', 'winc.ico')
    if ($IncludePlugins) {
        $required += @('PLUGINS.md', 'README-PORTABLE.txt',
            'plugins/winc.remove-duplicate-lines/plugin.json',
            'plugins/winc.remove-duplicate-lines/main.rhai',
            'plugins/winc.sort-json-keys/plugin.json',
            'plugins/winc.sort-json-keys/main.rhai')
    }
    # Exact membership also excludes installer markers, user state and compiler output.
    foreach ($name in $required) {
        if (-not $files.ContainsKey($name) -or $files[$name].Length -eq 0) { throw "missing or empty: $name" }
    }
    foreach ($name in $files.Keys) {
        if ($name -notin $required) { throw "unexpected archive file: $name" }
    }
    function Read-EntryBytes([string]$Name) {
        $inputStream = $files[$Name].Open()
        $buffer = [IO.MemoryStream]::new()
        try {
            $inputStream.CopyTo($buffer)
            return ,$buffer.ToArray()
        } finally {
            $inputStream.Dispose()
            $buffer.Dispose()
        }
    }
    foreach ($name in @('winc.exe', 'winc-pty.exe', 'conpty.dll', 'OpenConsole.exe')) {
        $bytes = Read-EntryBytes $name
        if ($bytes.Length -lt 64 -or $bytes[0] -ne 0x4d -or $bytes[1] -ne 0x5a) { throw "invalid PE: $name" }
        $pe = [BitConverter]::ToInt32($bytes, 60)
        if ($pe -lt 0 -or $pe -gt $bytes.Length - 6 -or
            [BitConverter]::ToUInt32($bytes, $pe) -ne 0x00004550 -or
            [BitConverter]::ToUInt16($bytes, $pe + 4) -ne 0x8664) { throw "not a Windows x64 binary: $name" }
    }
    foreach ($pair in @(@('winc-watch', 62), @('winc-server-Linux-x86_64', 62), @('winc-server-Linux-aarch64', 183))) {
        $bytes = Read-EntryBytes $pair[0]
        if ($bytes.Length -lt 20 -or [BitConverter]::ToUInt32($bytes, 0) -ne 0x464c457f -or
            $bytes[4] -ne 2 -or $bytes[5] -ne 1 -or
            [BitConverter]::ToUInt16($bytes, 18) -ne $pair[1]) { throw "wrong Linux binary: $($pair[0])" }
    }
    if (([Text.Encoding]::UTF8.GetString((Read-EntryBytes 'bin/winc'))).Contains("`r")) {
        throw 'WSL launcher must use LF line endings'
    }
    if ($IncludePlugins) {
        foreach ($name in @('remove-duplicate-lines', 'sort-json-keys')) {
            $manifest = [Text.Encoding]::UTF8.GetString((Read-EntryBytes "plugins/winc.$name/plugin.json")) | ConvertFrom-Json
            if ($manifest.id -ne "winc.$name" -or $manifest.api_version -ne 1 -or
                $manifest.commands[0].script -ne 'main.rhai') { throw "invalid bundled plugin manifest: $name" }
        }
    }
    Write-Host "Verified $($files.Count) portable files, Windows x64 and both Linux architectures."
} finally {
    $zip.Dispose()
}
