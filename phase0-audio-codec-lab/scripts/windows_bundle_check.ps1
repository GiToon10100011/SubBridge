<#
  U1 Windows FFmpeg 번들 점검 (W9 크기/의존성/라이선스, W10 첫 실행·백신).
  이 파일은 UTF-8 **BOM 포함**으로 저장돼 있다. Windows PowerShell 5.1 은 BOM 이 없으면 CP949 로 읽어 한글이 깨진다.

  이 스크립트는 사실을 기록할 뿐 PASS/FAIL 을 판정하지 않는다. 판정은 결과를 받은 뒤 보고서에서 한다.
  ffmpeg.exe 를 내려받아 **풀자마자** 실행해야 "첫 실행" 시간이 의미가 있다. Unblock-File 은 쓰지 않는다 (Zone.Identifier 를 지우면 W10 시험이 달라진다).
  SmartScreen 팝업은 GUI 에서만 뜨므로 이 스크립트가 잡지 못한다. 탐색기에서 exe 를 더블클릭해 본 결과는 사람이 따로 적는다.

  사용 (PowerShell, 랩 루트에서):
    scripts\windows_bundle_check.ps1 -Ffmpeg <압축 푼 폴더>\bin\ffmpeg.exe -Ffprobe <압축 푼 폴더>\bin\ffprobe.exe -SourceUrl <내려받은 URL>
  (<압축 푼 폴더> 는 이미 있는 폴더의 실제 경로다. 새로 만들 이름이 아니다 — 예: 탐색기에서 압축을 푼 위치)
  결과: -OutDir (기본 results\windows_lgpl\bundle_check) 아래 bundle_check.json 과 텍스트 파일들.
  의존 DLL: dumpbin.exe 가 PATH 에 있으면(VS 개발자 명령 프롬프트) 그것을 쓰고, 없으면 파일 안의 "*.dll" 문자열을 찾는다(근사치).
#>
param(
  [Parameter(Mandatory = $true)][string]$Ffmpeg,
  [Parameter(Mandatory = $true)][string]$Ffprobe,
  [string]$OutDir = "results\windows_lgpl\bundle_check",
  [string]$SourceUrl = "",
  [string]$Note = "",
  [string]$Fixture = "fixtures\generated\F07_m4a_aac_lc_stereo_44k1.m4a",
  [ValidateRange(1, 50)][int]$WarmRuns = 5
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
$out = Join-Path $root $OutDir
New-Item -ItemType Directory -Force -Path $out | Out-Null
$checked_at = Get-Date -Format "yyyy-MM-ddTHH:mm:sszzz"

function Redact([string]$s) {
  if ($s -and $env:USERPROFILE) { return $s.Replace($env:USERPROFILE, "%USERPROFILE%") }
  return $s
}
function Resolve-Exe([string]$p) {
  if (Test-Path -LiteralPath $p) { return (Get-Item -LiteralPath $p).FullName }
  return (Get-Command $p -ErrorAction Stop).Source
}
# 네이티브 stderr 가 오류로 취급되어 스크립트가 멈추지 않도록 잠시 Continue 로 바꾼다 (Windows PowerShell 5.1)
function Invoke-Native([string]$Exe, [string[]]$ArgList) {
  $old = $ErrorActionPreference
  $ErrorActionPreference = "Continue"
  try {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $lines = @(& $Exe @ArgList 2>&1 | ForEach-Object { "$_" })
    $code = $LASTEXITCODE
    $sw.Stop()
  }
  finally { $ErrorActionPreference = $old }
  return [pscustomobject]@{ Code = $code; Ms = [math]::Round($sw.Elapsed.TotalMilliseconds, 1); Lines = $lines }
}
function Write-Utf8([string]$path, [string]$text) {
  [System.IO.File]::WriteAllText($path, $text, (New-Object System.Text.UTF8Encoding($false)))
}
function Median([double[]]$v) {
  if ($v.Count -eq 0) { return $null }
  $s = $v | Sort-Object
  return $s[[int][math]::Floor(($s.Count - 1) / 2)]
}

$ff = @{ ffmpeg = (Resolve-Exe $Ffmpeg); ffprobe = (Resolve-Exe $Ffprobe) }

# --- 1. 백신 환경 (실행 전에 기록. 첫 실행 시간을 해석하는 데 필요하다)
$av = [ordered]@{ products = @(); defender_realtime = $null; defender_signature = $null; defender_error = $null }
try { $av.products = @(Get-CimInstance -Namespace root/SecurityCenter2 -ClassName AntiVirusProduct -ErrorAction Stop | ForEach-Object { $_.displayName }) } catch { $av.defender_error = "SecurityCenter2: $($_.Exception.Message)" }
try {
  $mp = Get-MpComputerStatus -ErrorAction Stop
  $av.defender_realtime = $mp.RealTimeProtectionEnabled
  $av.defender_signature = $mp.AntivirusSignatureVersion
}
catch { $av.defender_error = "Get-MpComputerStatus: $($_.Exception.Message)" }
$run_started = Get-Date

# --- 2. 첫 실행 시간: 이 파일을 이 세션에서 처음 실행하는 순간 (-version)
$first_ms = [ordered]@{}
foreach ($k in "ffmpeg", "ffprobe") { $first_ms[$k] = (Invoke-Native $ff[$k] @("-version")).Ms }

# --- 3. 파일별 정적 사실
$bins = @()
foreach ($k in "ffmpeg", "ffprobe") {
  $p = $ff[$k]
  $ver = Invoke-Native $p @("-version")
  # -hide_banner: Invoke-Native 가 stderr 를 합치므로 없으면 license_lines 에 버전 배너가 들어간다 (W9 첫 실행에서 실제로 그랬다)
  $lic = Invoke-Native $p @("-hide_banner", "-L")
  $conf = Invoke-Native $p @("-buildconf")
  $conf_text = $conf.Lines -join " "
  $sig = Get-AuthenticodeSignature -LiteralPath $p
  $zone = "none"
  try {
    $z = @(Get-Content -LiteralPath $p -Stream Zone.Identifier -ErrorAction Stop | Where-Object { $_ -like "ZoneId=*" })
    if ($z.Count -gt 0) { $zone = $z[0].Substring(7) }
  }
  catch { }
  $deps = @()
  $method = "string-scan"
  $dumpbin = Get-Command dumpbin.exe -ErrorAction SilentlyContinue
  if ($dumpbin) {
    $d = Invoke-Native $dumpbin.Source @("/dependents", $p)
    $deps = @($d.Lines | Where-Object { $_ -match "^\s+\S+\.dll\s*$" } | ForEach-Object { $_.Trim() })
    $method = "dumpbin"
  }
  else {
    $text = [System.Text.Encoding]::GetEncoding(28591).GetString([System.IO.File]::ReadAllBytes($p))
    $deps = @([regex]::Matches($text, "[A-Za-z0-9_\.\-]{1,64}\.dll", "IgnoreCase") | ForEach-Object { $_.Value.ToLower() } | Sort-Object -Unique)
  }
  Write-Utf8 (Join-Path $out "dependents_$k.txt") ("# method: $method`r`n" + ($deps -join "`r`n") + "`r`n")
  if ($k -eq "ffmpeg") {
    Write-Utf8 (Join-Path $out "ffmpeg_license.txt") (Redact ($lic.Lines -join "`r`n"))
    Write-Utf8 (Join-Path $out "ffmpeg_buildconf.txt") (Redact ($conf.Lines -join "`r`n"))
  }
  $bins += [ordered]@{
    name              = $k
    path              = Redact $p
    bytes             = (Get-Item -LiteralPath $p).Length
    sha256            = (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash
    zone_id           = $zone
    signature_status  = "$($sig.Status)"
    signature_subject = if ($sig.SignerCertificate) { $sig.SignerCertificate.Subject } else { $null }
    version_line      = ($ver.Lines | Select-Object -First 1)
    license_lines     = (($lic.Lines | Select-Object -First 4) -join " ")
    enable_gpl        = ($conf_text -match "--enable-gpl")
    enable_version3   = ($conf_text -match "--enable-version3")
    enable_nonfree    = ($conf_text -match "--enable-nonfree")
    enable_shared     = ($conf_text -match "--enable-shared")
    external_libs     = @([regex]::Matches($conf_text, "--enable-(lib[a-z0-9_\-]+)") | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique)
    dependents_method = $method
    dependents        = $deps
  }
}

# --- 4. 변환 시간: 첫 변환 1회 + 이어서 WarmRuns 회 (같은 입력, 임시 출력은 끝에 지운다)
$timing = [ordered]@{ fixture = $Fixture; skipped = $null }
$fix = Join-Path $root $Fixture
if (-not (Test-Path -LiteralPath $fix)) {
  $timing.skipped = "fixture 없음: $Fixture"
  Write-Warning $timing.skipped
}
else {
  $tmp = Join-Path $env:TEMP ("u1_bundle_check_" + [guid]::NewGuid().ToString("N") + ".wav")
  try {
    $convert = @("-hide_banner", "-loglevel", "error", "-y", "-i", $fix, "-vn", "-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le", "-f", "wav", $tmp)
    $first = Invoke-Native $ff.ffmpeg $convert
    $warm = @(1..$WarmRuns | ForEach-Object { Invoke-Native $ff.ffmpeg $convert })
    $timing.convert_first_ms = $first.Ms
    $timing.convert_warm_ms = @($warm | ForEach-Object { $_.Ms })
    $timing.convert_warm_median_ms = (Median ([double[]]@($warm | ForEach-Object { $_.Ms })))
    $timing.convert_exit_codes = @(@($first) + $warm | ForEach-Object { $_.Code })
    $timing.output_bytes = if (Test-Path -LiteralPath $tmp) { (Get-Item -LiteralPath $tmp).Length } else { 0 }
    $timing.stderr_first_lines = @(@($first) + $warm | ForEach-Object { $_.Lines } | Select-Object -First 3 | ForEach-Object { Redact $_ })
  }
  finally { if (Test-Path -LiteralPath $tmp) { Remove-Item -Force -LiteralPath $tmp } }
}

# --- 5. 실행 뒤 Defender 탐지 기록 (관리자 권한이 없으면 조회가 안 될 수 있다)
$det = [ordered]@{ available = $false; count = $null; error = $null }
try {
  $hits = @(Get-MpThreatDetection -ErrorAction Stop | Where-Object { $_.InitialDetectionTime -ge $run_started })
  $det.available = $true
  $det.count = $hits.Count
}
catch { $det.error = $_.Exception.Message }

$os = Get-CimInstance Win32_OperatingSystem
$report = [ordered]@{
  checked_at                    = $checked_at
  source_url                    = $SourceUrl
  note                          = $Note
  os                            = $os.Caption
  os_build                      = $os.BuildNumber
  av                            = $av
  first_launch_version_ms       = $first_ms
  binaries                      = $bins
  timing                        = $timing
  defender_detections_after_run = $det
}
Write-Utf8 (Join-Path $out "bundle_check.json") (Redact ($report | ConvertTo-Json -Depth 6))

Write-Host "done. $OutDir 를 확인한다."
Write-Host "ffmpeg $($bins[0].bytes) bytes, -L = $($bins[0].license_lines)"
Write-Host "첫 -version $($first_ms.ffmpeg) ms / 첫 변환 $($timing.convert_first_ms) ms / 이후 중앙값 $($timing.convert_warm_median_ms) ms"
Write-Host "이 값은 기록일 뿐 판정이 아니다. 탐색기 더블클릭 시 SmartScreen/Defender 팝업이 떴는지는 사람이 따로 적는다."
