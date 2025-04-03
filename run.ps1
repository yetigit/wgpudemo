$env:RUSTFLAGS = "--cfg=web_sys_unstable_apis"
$buildOutput = @()

$buildGood = 0
$bindingGood = 0

$rustBuildOutput = cargo build --target wasm32-unknown-unknown --release 2>&1
$exitCode = $LASTEXITCODE

if ($exitCode -eq 0)
{
  Write-Host "Build good"
  $buildGood = 1
} else
{
  Write-Host "Build BAD"
}
$buildOutput += $rustBuildOutput

if ($buildGood -eq 1)
{
  $bindingGen = wasm-bindgen --target web --no-typescript --out-dir ./www/pkg ./target/wasm32-unknown-unknown/release/gpudemo.wasm
  $exitCode = $LASTEXITCODE

  if ($exitCode -eq 0)
  {
    Write-Host "Binding good"
    $bindingGood = 1
  } else
  {
    Write-Host "Binding BAD"
  }
  $buildOutput += $bindingGen
}

$buildOutput -join "`r`n" | Set-Clipboard
if ($buildGood -eq 1 -and $bindingGood -eq 1)
{
  basic-http-server ./www
}
