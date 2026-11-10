# ADR-033 E8 to E10: the MuseScore procedure (local, Windows, reference machine).
#
# Needs MuseScore 4 (default install path below), Python 3 and a release build of
# mc-cli. Run from the repository root:
#   cargo build --release -p mc-cli --locked
#   powershell -File tools/musescore-check/run.ps1
# Output goes to target/musescore-check/; the summary is printed and also written to
# target/musescore-check/summary.txt for the result file in benchmarks/.
param(
    [string]$MuseScore = "C:\Program Files\MuseScore 4\bin\MuseScore4.exe"
)
# Native tools write progress to stderr, which Windows PowerShell 5 turns into errors;
# every step checks its own exit code and output instead.
$ErrorActionPreference = "Continue"
$out = "target/musescore-check"
New-Item -ItemType Directory $out -Force | Out-Null
$cli = "target/release/mc-cli.exe"
$summary = "$out/summary.txt"
"MuseScore check $(Get-Date -Format s)" | Set-Content $summary -Encoding UTF8
"MuseScore: $(& $MuseScore --version 2>&1 | Select-Object -First 1)" | Add-Content $summary -Encoding UTF8

function Say($line) { Write-Output $line; Add-Content -Path $summary -Value $line -Encoding UTF8 }

# The E1 inputs.
foreach ($f in 'feature-rich', 'small', 'typical', 'empty') {
    & $cli export-musicxml "fixture:$f" "$out/$f.musicxml" | Out-Null
}
& $cli export-midi fixture:feature-rich "$out/feature-rich.ours.mid" | Out-Null
& $cli export-midi fixture:typical "$out/typical.ours.mid" | Out-Null
& $cli export-musicxml "midi:$out/feature-rich.ours.mid" "$out/from-midi.musicxml" 2>$null | Out-Null

# E8: MuseScore opens each file headless and converts it.
foreach ($f in 'feature-rich', 'small', 'typical', 'empty', 'from-midi') {
    foreach ($ext in 'mid', 'musicxml') {
        $target = "$out/$f.musescore.$ext"
        Remove-Item $target -ErrorAction SilentlyContinue
        $p = Start-Process -FilePath $MuseScore -ArgumentList @('-o', $target, "$out/$f.musicxml") `
            -NoNewWindow -Wait -PassThru -RedirectStandardError "$out/$f.$ext.stderr.txt"
        $ok = ($p.ExitCode -eq 0) -and (Test-Path $target)
        Say ("E8 open+convert {0} -> {1}: exit {2}, output {3}" -f $f, $ext, $p.ExitCode, $(if ($ok) { 'written' } else { 'MISSING' }))
    }
}

# E8: same notes as our own MIDI export.
foreach ($f in 'feature-rich', 'typical') {
    Say "--- E8 notes: $f (ours vs MuseScore)"
    python tools/musescore-check/compare.py midi "$out/$f.ours.mid" "$out/$f.musescore.mid" 2>&1 | ForEach-Object { Say $_ }
}

# E9: string and fret survive MuseScore's own MusicXML export.
foreach ($f in 'feature-rich', 'typical', 'from-midi') {
    Say "--- E9 strings: $f (ours vs MuseScore re-export)"
    python tools/musescore-check/compare.py strings "$out/$f.musicxml" "$out/$f.musescore.musicxml" 2>&1 | ForEach-Object { Say $_ }
}

# ADR-035 TQ-8: the techniques fixture through MuseScore, technique by technique.
& $cli export-musicxml fixture:techniques "$out/techniques.musicxml" | Out-Null
$target = "$out/techniques.musescore.musicxml"
Remove-Item $target -ErrorAction SilentlyContinue
$p = Start-Process -FilePath $MuseScore -ArgumentList @('-o', $target, "$out/techniques.musicxml") `
    -NoNewWindow -Wait -PassThru -RedirectStandardError "$out/techniques.musicxml.stderr.txt"
Say ("TQ-8 open+convert techniques -> musicxml: exit {0}, output {1}" -f $p.ExitCode, $(if (Test-Path $target) { 'written' } else { 'MISSING' }))
Say "--- TQ-8 techniques (ours vs MuseScore re-export)"
python tools/musescore-check/compare.py techniques "$out/techniques.musicxml" $target 2>&1 | ForEach-Object { Say $_ }
# TQ-8, import side: what MuseScore keeps in its own file, before its MusicXML exporter.
$z = "$out/techniques.musescore.mscz"
Remove-Item $z -ErrorAction SilentlyContinue
$p = Start-Process -FilePath $MuseScore -ArgumentList @('-o', $z, "$out/techniques.musicxml") `
    -NoNewWindow -Wait -PassThru -RedirectStandardError "$out/techniques.mscz.stderr.txt"
if (Test-Path $z) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead((Resolve-Path $z).Path)
    $entry = $zip.Entries | Where-Object { $_.FullName -like '*.mscx' } | Select-Object -First 1
    $reader = New-Object IO.StreamReader($entry.Open()); $mscx = $reader.ReadToEnd(); $reader.Close(); $zip.Dispose()
    $n = { param($pattern) ([regex]::Matches($mscx, $pattern)).Count }
    Say ("TQ-8 MuseScore file (import): exit {0}; accent articulations {1}, down-bow {2}, up-bow {3}, cross noteheads {4}, strum symbols {5}, PalmMute spanners {6}" -f `
        $p.ExitCode, (& $n 'articAccent'), (& $n 'stringsDownBow'), (& $n 'stringsUpBow'), (& $n '<head>cross'), (& $n '<name>guitarStrum'), (& $n 'type="PalmMute"'))
} else {
    Say "TQ-8 MuseScore file (import): MISSING"
}
Remove-Item "$out/techniques*.png" -ErrorAction SilentlyContinue
$p = Start-Process -FilePath $MuseScore -ArgumentList @('-o', "$out/techniques.png", "$out/techniques.musicxml") `
    -NoNewWindow -Wait -PassThru
Say ("TQ-8 PNG: exit {0}, files {1}" -f $p.ExitCode, ((Get-ChildItem "$out/techniques*.png" | ForEach-Object Name) -join ', '))
# E10: a PNG rendering of feature_rich for review.
Remove-Item "$out/feature-rich*.png" -ErrorAction SilentlyContinue
$p = Start-Process -FilePath $MuseScore -ArgumentList @('-o', "$out/feature-rich.png", "$out/feature-rich.musicxml") `
    -NoNewWindow -Wait -PassThru
Say ("E10 PNG: exit {0}, files {1}" -f $p.ExitCode, ((Get-ChildItem "$out/feature-rich*.png" | ForEach-Object Name) -join ', '))

# E10 review copy: the first page scaled to 1700 px wide, the music part only (the
# page is 10200 px wide at MuseScore's default resolution).
Add-Type -AssemblyName System.Drawing
$page = "$out/feature-rich-1.png"
if (Test-Path $page) {
    $src = [System.Drawing.Image]::FromFile((Resolve-Path $page).Path)
    $w = 1700; $h = [int]($src.Height * $w / $src.Width)
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.InterpolationMode = 'HighQualityBicubic'
    $g.Clear([System.Drawing.Color]::White)
    $g.DrawImage($src, 0, 0, $w, $h)
    $view = $bmp.Clone((New-Object System.Drawing.Rectangle 0, 250, $w, 520), $bmp.PixelFormat)
    $view.Save((Join-Path (Resolve-Path $out).Path 'preview.png'), [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose(); $view.Dispose(); $src.Dispose()
    Say "E10 preview: $out/preview.png"
}
