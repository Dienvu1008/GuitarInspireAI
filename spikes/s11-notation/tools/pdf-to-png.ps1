# Renders page 1 of a PDF to PNG with the Windows PDF renderer (Windows.Data.Pdf, WinRT),
# an engine independent of both PDF routes. Windows PowerShell 5.1.
#   powershell -File pdf-to-png.ps1 <in.pdf> <out.png> [scale]
param([string]$In, [string]$Out, [double]$Scale = 2.0)
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime]
$null = [Windows.Data.Pdf.PdfDocument, Windows.Data.Pdf, ContentType = WindowsRuntime]
$null = [Windows.Storage.Streams.InMemoryRandomAccessStream, Windows.Storage.Streams, ContentType = WindowsRuntime]
$asTask = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
} | Select-Object -First 1
$asAction = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncAction'
} | Select-Object -First 1
function Await($op, [type]$t) { $task = $asTask.MakeGenericMethod($t).Invoke($null, @($op)); $task.Wait(); $task.Result }
$file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync((Resolve-Path $In).Path)) ([Windows.Storage.StorageFile])
$doc = Await ([Windows.Data.Pdf.PdfDocument]::LoadFromFileAsync($file)) ([Windows.Data.Pdf.PdfDocument])
$page = $doc.GetPage(0)
$opts = New-Object Windows.Data.Pdf.PdfPageRenderOptions
$opts.DestinationWidth = [uint32]($page.Size.Width * $Scale)
$opts.DestinationHeight = [uint32]($page.Size.Height * $Scale)
$stream = New-Object Windows.Storage.Streams.InMemoryRandomAccessStream
$asAction.Invoke($null, @($page.RenderToStreamAsync($stream, $opts))).Wait()
$net = [System.IO.WindowsRuntimeStreamExtensions]::AsStreamForRead($stream.GetInputStreamAt(0))
$fs = [System.IO.File]::Create($Out); $net.CopyTo($fs); $fs.Close()
"{0}: {1} page(s), page 1 {2:N0} x {3:N0} pt -> {4}" -f (Split-Path $In -Leaf), $doc.PageCount, $page.Size.Width, $page.Size.Height, $Out
