[Windows.Security.Cryptography.CryptographicBuffer, Windows.Security.Cryptography, ContentType = WindowsRuntime] | Out-Null
[Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime] | Out-Null
[Windows.Storage.Streams.IRandomAccessStream, Windows.Storage.Streams, ContentType = WindowsRuntime] | Out-Null
[Windows.Graphics.Imaging.BitmapDecoder, Windows.Graphics.Imaging, ContentType = WindowsRuntime] | Out-Null
[Windows.Media.Ocr.OcrEngine, Windows.Media.Ocr, ContentType = WindowsRuntime] | Out-Null

$path = (Resolve-Path ".\artifacts\live\02_game_loaded.png").Path
$op = [Windows.Storage.StorageFile]::GetFileFromPathAsync($path)
while ($op.Status -eq 'Started') { Start-Sleep -Milliseconds 20 }
$file = $op.GetResults()

$streamOp = $file.OpenAsync([Windows.Storage.FileAccessMode]::Read)
while ($streamOp.Status -eq 'Started') { Start-Sleep -Milliseconds 20 }
$stream = $streamOp.GetResults()

$decOp = [Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)
while ($decOp.Status -eq 'Started') { Start-Sleep -Milliseconds 20 }
$decoder = $decOp.GetResults()

$bmpOp = $decoder.GetSoftwareBitmapAsync()
while ($bmpOp.Status -eq 'Started') { Start-Sleep -Milliseconds 20 }
$bitmap = $bmpOp.GetResults()

$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
if ($engine -eq $null) {
    $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage([Windows.Globalization.Language]::new("en-US"))
}
$ocrOp = $engine.RecognizeAsync($bitmap)
while ($ocrOp.Status -eq 'Started') { Start-Sleep -Milliseconds 50 }
$result = $ocrOp.GetResults()

Write-Output "=== FULL OCR TEXT ==="
Write-Output $result.Text
Write-Output "=== DETECTED LINES & WORDS ==="
foreach ($line in $result.Lines) {
    Write-Output "Line: $($line.Text)"
    foreach ($w in $line.Words) {
        $r = $w.BoundingRect
        Write-Output "   Word '$($w.Text)' at ($($r.X), $($r.Y), $($r.Width), $($r.Height))"
    }
}
