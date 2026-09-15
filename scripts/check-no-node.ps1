$ErrorActionPreference = 'Stop'
$originalPath = $env:Path
try {
  $env:Path = (($env:Path -split ';') | Where-Object { $_ -and -not (Test-Path (Join-Path $_ 'node.exe')) }) -join ';'
  if (Get-Command node -ErrorAction SilentlyContinue) { throw 'Node.js still reachable' }
  & bun.exe run check:runtime
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  & bun.exe run build
  exit $LASTEXITCODE
} finally { $env:Path = $originalPath }
