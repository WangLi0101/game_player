$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/prepare-update-release.ps1"
$project = Join-Path $PSScriptRoot '../..'
$fixture = Get-Content -LiteralPath (Join-Path $project 'src-tauri/src/app_update/fixtures/signed.json') -Raw | ConvertFrom-Json
$repository = 'WangLi0101/game_player'
function Must-Fail([scriptblock]$Task) {
  $failed = $false
  try { & $Task | Out-Null } catch { $failed = $true }
  if (!$failed) { throw '预期拒绝无效发布数据。' }
}
$valid = New-UpdateManifest '1.1.0' '中文更新说明' $fixture.signature $repository 'youji_1.1.0_x64-setup.exe'
if ($valid.platforms.'windows-x86_64'.signature -ne $fixture.signature) { throw '签名未保留。' }
if ($valid.platforms.'windows-x86_64'.url -cne 'https://github.com/WangLi0101/game_player/releases/download/v1.1.0/youji_1.1.0_x64-setup.exe') {
  throw '更新清单的下载地址与发布仓库不一致。'
}
Must-Fail { New-UpdateManifest '1.2.0' '中文说明' $fixture.signature $repository 'youji_1.2.0_x64-setup.exe' }
Must-Fail { New-UpdateManifest '1.1.0' '' $fixture.signature $repository 'youji_1.1.0_x64-setup.exe' }
Must-Fail { New-UpdateManifest '1.1.0' '中文说明' '' $repository 'youji_1.1.0_x64-setup.exe' }
Must-Fail { New-UpdateManifest '1.1.0' '中文说明' $fixture.signature $repository 'youji_1.2.0_x64-setup.exe' }
$temp = Join-Path ([IO.Path]::GetTempPath()) ('youji-release-' + [guid]::NewGuid())
try {
  [IO.Directory]::CreateDirectory((Join-Path $temp 'release-notes')) | Out-Null
  [IO.Directory]::CreateDirectory((Join-Path $temp 'src-tauri')) | Out-Null
  [IO.Directory]::CreateDirectory((Join-Path $temp 'bundle')) | Out-Null
  [IO.File]::WriteAllText((Join-Path $temp 'src-tauri/tauri.conf.json'), (@{version='1.1.0';plugins=@{updater=@{pubkey=$fixture.publicKey}}} | ConvertTo-Json -Depth 5))
  Must-Fail { Read-ReleaseNotes $temp '1.1.0' }
  [IO.File]::WriteAllText((Join-Path $temp 'release-notes/1.1.0.md'), '中文说明')
  [IO.File]::WriteAllBytes((Join-Path $temp 'bundle/test.exe'), [Convert]::FromBase64String($fixture.payload))
  Must-Fail { Prepare-UpdateRelease $temp (Join-Path $temp 'bundle') (Join-Path $temp 'output') $repository }
  [IO.File]::WriteAllText((Join-Path $temp 'bundle/test.exe.sig'), $fixture.signature)
  Prepare-UpdateRelease $temp (Join-Path $temp 'bundle') (Join-Path $temp 'output') $repository
  foreach ($name in @('youji_1.1.0_x64-setup.exe','youji_1.1.0_x64-setup.exe.sig','latest.json','release-notes.md')) {
    if (!(Test-Path -LiteralPath (Join-Path $temp "output/$name"))) { throw "缺少发布产物 $name。" }
  }
} finally { if (Test-Path -LiteralPath $temp) { Remove-Item -LiteralPath $temp -Recurse -Force } }
Write-Output '更新发布规则用例通过。'
