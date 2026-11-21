param(
    [string]$ImagePath = ".\artifacts\live\03_download_progress.png"
)

Add-Type -AssemblyName System.Runtime.WindowsRuntime
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]

function Await($asyncOp, $type) {
    $asTask = $asTaskGeneric.MakeGenericMethod($type)
    $netTask = $asTask.Invoke($null, @($asyncOp))
    $netTask.Wait(-1) | Out-Null
    $netTask.Result
}

[Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime] | Out-Null
[Windows.Graphics.Imaging.BitmapDecoder, Windows.Graphics.Imaging, ContentType = WindowsRuntime] | Out-Null
[Windows.Media.Ocr.OcrEngine, Windows.Media.Ocr, ContentType = WindowsRuntime] | Out-Null

$fullPath = (Resolve-Path $ImagePath).Path
$fileOp = [Windows.Storage.StorageFile]::GetFileFromPathAsync($fullPath)
$file = Await $fileOp ([Windows.Storage.StorageFile])

$streamOp = $file.OpenAsync([Windows.Storage.FileAccessMode]::Read)
$stream = Await $streamOp ([Windows.Storage.Streams.IRandomAccessStream])

$decoderOp = [Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)
$decoder = Await $decoderOp ([Windows.Graphics.Imaging.BitmapDecoder])

$bmpOp = $decoder.GetSoftwareBitmapAsync()
$bitmap = Await $bmpOp ([Windows.Graphics.Imaging.SoftwareBitmap])

$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
if ($null -eq $engine) {
    $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage([Windows.Globalization.Language]::new("en-US"))
}
$ocrOp = $engine.RecognizeAsync($bitmap)
$result = Await $ocrOp ([Windows.Media.Ocr.OcrResult])

Write-Output "=== OCR RESULT FOR: $ImagePath ==="
Write-Output "FULL_TEXT: $($result.Text)"
foreach ($l in $result.Lines) {
    Write-Output "LINE: $($l.Text)"
    foreach ($w in $l.Words) {
        $r = $w.BoundingRect
        Write-Output "  WORD: '$($w.Text)' [$($r.X),$($r.Y),$($r.Width),$($r.Height)]"
    }
}
