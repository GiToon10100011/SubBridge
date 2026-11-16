<#
  U1 Windows x86_64 재현 스크립트.
  이 파일은 UTF-8 **BOM 포함**으로 저장돼 있다. Windows PowerShell 5.1 은 BOM 이 없으면 CP949 로 읽어 한글 리터럴이 깨진다.
  편집기로 고칠 때 BOM 을 지우지 않는다.

  macOS 에서 만든 fixtures\generated 와 fixtures\manifest.csv 를 이 랩 폴더에 그대로 복사한 뒤 실행한다.
  (픽스처를 Windows 에서 다시 만들지 않는다: 손실 압축 파일은 FFmpeg 버전이 다르면 바이트가 달라진다.)

  사용 (PowerShell, 랩 루트에서):
    # 호환성 세트 + 공백·한글 경로 테스트 (성능 세트 제외)
    scripts\windows_run.ps1 -Ffmpeg C:\ffmpeg\bin\ffmpeg.exe -Ffprobe C:\ffmpeg\bin\ffprobe.exe -SkipPerf
    # 성능 세트(1/10/61분)만. 앞선 호환성 실행의 결과는 덮어쓰지 않고 *_perf 파일로 쓴다 (W8)
    scripts\windows_run.ps1 -Ffmpeg ... -Ffprobe ... -PerfOnly
    # 다른 FFmpeg(예: LGPL 번들 후보)로 호환성 세트만, 결과는 다른 폴더에 (W9)
    scripts\windows_run.ps1 -Ffmpeg ... -Ffprobe ... -SkipPerf -SkipPathTest -ResultsDir results\windows_lgpl

  전제: Rust(stable, edition 2024 지원 = 1.85+) + MSVC 빌드 도구, 인터넷(크레이트 내려받기) 또는 복사해 온 .cargo-home.
  -ResultsDir 는 랩 루트 기준 상대 경로다.
  결과: -ResultsDir 아래 CSV/JSONL. results\windows 를 쓴 경우 끝나면 `python scripts\assemble_results.py` 로 results\ 최상위 CSV 를 갱신한다.
#>
param(
  [string]$Ffmpeg = "ffmpeg.exe",
  [string]$Ffprobe = "ffprobe.exe",
  [string]$ResultsDir = "results\windows",
  [switch]$SkipPerf,
  [switch]$PerfOnly,
  [switch]$SkipPathTest
)
$ErrorActionPreference = "Stop"
if ($SkipPerf -and $PerfOnly) { throw "-SkipPerf 와 -PerfOnly 는 함께 쓸 수 없다" }
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
New-Item -ItemType Directory -Force -Path $ResultsDir | Out-Null
# -PerfOnly 는 앞선 실행의 환경 기록을 덮어쓰지 않도록 다른 이름으로 쓴다
$sfx = if ($PerfOnly) { "_perf" } else { "" }

# --- 1. 환경 기록 (결과 해석에 필요한 사실만)
$os = Get-CimInstance Win32_OperatingSystem
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$ffmpeg_file = Get-Item (Get-Command $Ffmpeg).Source
$env_info = [ordered]@{
  os           = $os.Caption
  os_build     = $os.BuildNumber
  cpu          = $cpu.Name
  ram_gb       = [math]::Round($os.TotalVisibleMemorySize / 1MB, 1)
  rustc        = (rustc --version)
  ffmpeg_path  = $Ffmpeg
  ffmpeg_bytes = $ffmpeg_file.Length
  ffmpeg_ver   = ((& $Ffmpeg -version) | Select-Object -First 1)
  ffmpeg_lic   = (((& $Ffmpeg -L) | Select-Object -First 4) -join " ")
  ffmpeg_conf  = ((& $Ffmpeg -buildconf) -join " ")
}
$env_info | ConvertTo-Json | Out-File -Encoding utf8 "$ResultsDir\environment$sfx.json"
# 경로 칸이 잘리지 않도록 표 대신 한 줄씩 쓴다 ("SHA-256  경로")
Get-FileHash $ffmpeg_file.FullName, (Get-Command $Ffprobe).Source -Algorithm SHA256 | ForEach-Object { "$($_.Hash)  $($_.Path)" } | Out-File -Encoding utf8 "$ResultsDir\ffmpeg_sha256$sfx.txt"

# --- 2. 빌드 (Cargo.lock 고정)
$env:CARGO_HOME = Join-Path $root ".cargo-home"
cargo build --release --locked
if ($LASTEXITCODE -ne 0) { throw "cargo build 실패 (종료 코드 $LASTEXITCODE) — 위 오류를 확인" }
$exe = Join-Path $root "target\release\u1_audio_codec_lab.exe"
if (-not (Test-Path -LiteralPath $exe)) { throw "빌드 산출물이 없다: $exe" }

# --- 3. 픽스처 SHA-256 검증 (실패하면 중단: 같은 입력이 아니면 비교가 의미 없다)
& $exe verify-fixtures
if ($LASTEXITCODE -ne 0) { throw "fixture SHA-256 mismatch — macOS 의 fixtures\generated 를 그대로 복사했는지 확인" }

# --- 4. 호환성 매트릭스 (4모드 x 3회)
if (-not $PerfOnly) {
  & $exe matrix --set compat --ffmpeg $Ffmpeg --ffprobe $Ffprobe --results $ResultsDir
  if ($LASTEXITCODE -ne 0) { throw "호환성 matrix 비정상 종료 (종료 코드 $LASTEXITCODE)" }
  Move-Item -Force "$ResultsDir\performance.csv" "$ResultsDir\performance_compat_small_files.csv"
}

# --- 5. 성능 매트릭스 (1/10/61분)
if (-not $SkipPerf) {
  & $exe matrix --set perf --suffix _perf --ffmpeg $Ffmpeg --ffprobe $Ffprobe --results $ResultsDir
  if ($LASTEXITCODE -ne 0) { throw "성능 matrix 비정상 종료 (종료 코드 $LASTEXITCODE)" }
  Move-Item -Force "$ResultsDir\performance_perf.csv" "$ResultsDir\performance.csv"
}

# --- 6. 공백·한글 경로 (입력/출력 모두)
if (-not $PerfOnly -and -not $SkipPathTest) {
  $dir = Join-Path $env:TEMP "u1 경로 test"
  $prev_enc = [Console]::OutputEncoding
  try {
    # 프로그램이 UTF-8 로 쓰는 JSON 을 CP949 로 읽어 경로 문자열이 깨지지 않게 한다
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    $in = Join-Path $dir "입력 파일 (1).m4a"
    Copy-Item fixtures\generated\F07_m4a_aac_lc_stereo_44k1.m4a $in
    $lines = foreach ($m in "symphonia", "ffmpeg", "hybrid") {
      $out = Join-Path $dir "출력 $m.wav"
      & $exe run --mode $m --input $in --output $out --ffmpeg $Ffmpeg --ffprobe $Ffprobe --fixture-id PATH_TEST --json
      if ($LASTEXITCODE -ne 0) { Write-Warning "경로 테스트 $m 실패 (종료 코드 $LASTEXITCODE)" }
    }
    # BOM 없는 UTF-8 로 쓴다 (.NET 은 PowerShell 위치가 아니라 프로세스 현재 폴더를 쓰므로 절대 경로로 준다)
    [System.IO.File]::WriteAllLines((Join-Path $root "$ResultsDir\path_test.jsonl"), [string[]]@($lines), (New-Object System.Text.UTF8Encoding($false)))
  }
  finally {
    [Console]::OutputEncoding = $prev_enc
    # 이 시험이 만든 폴더 하나만 지운다. 도중에 실패해도 정리한다
    if (Test-Path -LiteralPath $dir) { Remove-Item -Recurse -Force -LiteralPath $dir }
  }
}

Write-Host "done. $ResultsDir 를 확인한다. results\windows 를 쓴 경우 WINDOWS_CHECKLIST.md 를 채운 뒤 python scripts\assemble_results.py 를 실행한다."
