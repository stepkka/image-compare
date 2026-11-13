param([Parameter(ValueFromRemainingArguments=$true)][string[]]$CargoArgs)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
if (Test-Path -LiteralPath "$root\.tools\cargo\bin\cargo.exe") {
    $env:CARGO_HOME = "$root\.tools\cargo"
    $env:RUSTUP_HOME = "$root\.tools\rustup"
    $env:PATH = "$env:CARGO_HOME\bin;$env:PATH"
}
$env:CARGO_NET_OFFLINE = 'false'
Push-Location $root
try {
    & cargo @CargoArgs
    if ($LASTEXITCODE -ne 0) { throw "Cargo failed with exit code $LASTEXITCODE" }
} finally { Pop-Location }
