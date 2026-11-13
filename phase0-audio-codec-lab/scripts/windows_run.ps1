<#
  U1 Windows x86_64 재현 스크립트 — 이번 실험에서는 **실행하지 못했다 (NOT_RUN)**.
  macOS 에서 만든 fixtures\generated 와 fixtures\manifest.csv 를 이 랩 폴더에 그대로 복사한 뒤 실행한다.
  (픽스처를 Windows 에서 다시 만들지 않는다: 손실 압축 파일은 FFmpeg 버전이 다르면 바이트가 달라진다.)

  사용 (PowerShell, 랩 루트에서):
    scripts\windows_run.ps1 -Ffmpeg C:\ffmpeg\bin\ffmpeg.exe -Ffprobe C:\ffmpeg\bin\ffprobe.exe

  전제: Rust(stable, edition 2024 지원 = 1.85+) + MSVC 빌드 도구, 인터넷(크레이트 내려받기) 또는 복사해 온 .cargo-home.
  결과: results\windows\ 아래 CSV/JSONL. 끝나면 `python scripts\assemble_results.py` 로 results\ 최상위 CSV 를 갱신한다.
#>
param(
  [string]$Ffmpeg = "ffmpeg.exe",
  [string]$Ffprobe = "ffprobe.exe",
  [switch]$SkipPerf,
  [switch]$SkipPathTest
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
New-Item -ItemType Directory -Force -Path results\windows | Out-Null

# --- 1. 환경 기록 (결과 해석에 필요한 사실만)
$os = Get-CimInstance Win32_OperatingSystem
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$env_info = [ordered]@{
  os          = $os.Caption
  os_build    = $os.BuildNumber
  cpu         = $cpu.Name
  ram_gb      = [math]::Round($os.TotalVisibleMemorySize / 1MB, 1)
  rustc       = (rustc --version)
  ffmpeg_path = $Ffmpeg
  ffmpeg_ver  = ((& $Ffmpeg -version) | Select-Object -First 1)
  ffmpeg_lic  = (((& $Ffmpeg -L) | Select-Object -First 4) -join " ")
  ffmpeg_conf = ((& $Ffmpeg -buildconf) -join " ")
}
$env_info | ConvertTo-Json | Out-File -Encoding utf8 results\windows\environment.json
Get-FileHash $Ffmpeg, $Ffprobe -Algorithm SHA256 | Format-Table Hash, Path | Out-File -Encoding utf8 results\windows\ffmpeg_sha256.txt

# --- 2. 빌드 (Cargo.lock 고정)
$env:CARGO_HOME = Join-Path $root ".cargo-home"
cargo build --release --locked
$exe = Join-Path $root "target\release\u1_audio_codec_lab.exe"

# --- 3. 픽스처 SHA-256 검증 (실패하면 중단: 같은 입력이 아니면 비교가 의미 없다)
& $exe verify-fixtures
if ($LASTEXITCODE -ne 0) { throw "fixture SHA-256 mismatch — macOS 의 fixtures\generated 를 그대로 복사했는지 확인" }

# --- 4. 호환성 매트릭스 (4모드 x 3회)
& $exe matrix --set compat --ffmpeg $Ffmpeg --ffprobe $Ffprobe --results results\windows
Move-Item -Force results\windows\performance.csv results\windows\performance_compat_small_files.csv

# --- 5. 성능 매트릭스 (1/10/61분)
if (-not $SkipPerf) {
  & $exe matrix --set perf --suffix _perf --ffmpeg $Ffmpeg --ffprobe $Ffprobe --results results\windows
  Move-Item -Force results\windows\performance_perf.csv results\windows\performance.csv
}

# --- 6. 공백·한글 경로 (입력/출력 모두)
if (-not $SkipPathTest) {
  $dir = Join-Path $env:TEMP "u1 경로 test"
  New-Item -ItemType Directory -Force -Path $dir | Out-Null
  $in = Join-Path $dir "입력 파일 (1).m4a"
  Copy-Item fixtures\generated\F07_m4a_aac_lc_stereo_44k1.m4a $in
  $lines = foreach ($m in "symphonia", "ffmpeg", "hybrid") {
    $out = Join-Path $dir "출력 $m.wav"
    & $exe run --mode $m --input $in --output $out --ffmpeg $Ffmpeg --ffprobe $Ffprobe --fixture-id PATH_TEST --json
  }
  $lines | Out-File -Encoding utf8 results\windows\path_test.jsonl
  Remove-Item -Recurse -Force $dir   # OS 임시 폴더에 만든 것은 여기서 정리한다
}

Write-Host "done. results\windows\ 를 확인하고 WINDOWS_CHECKLIST.md 를 채운 뒤 python scripts\assemble_results.py 를 실행한다."
