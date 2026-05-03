param(
    [Parameter(Mandatory = $true)]
    [string]$InputFile
)

if (-not (Test-Path -LiteralPath $InputFile)) {
    Write-Error "File not found: $InputFile"
    exit 1
}

$invalidLines = [System.Collections.Generic.List[string]]::new()
$looseLines   = [System.Collections.Generic.List[string]]::new()

$validCount = 0
$lineNo = 0

$reader = [System.IO.StreamReader]::new($InputFile, [System.Text.Encoding]::UTF8)

try {
    while ($null -ne ($line = $reader.ReadLine())) {
        $lineNo++

        $trimmed = $line.Trim()
        if ($trimmed.Length -eq 0) {
            continue
        }

        $parts = $trimmed -split '\s+'

        if ($parts.Count -ne 2 -and $parts.Count -ne 3) {
            $invalidLines.Add("line $lineNo invalid format: expected `word freq [tag]` -> $line")
            continue
        }

        $freq = 0
        if (-not [int]::TryParse($parts[1], [ref]$freq) -or $freq -lt 0) {
            $invalidLines.Add("line $lineNo invalid frequency `"$($parts[1])`" -> $line")
            continue
        }

        if ($parts.Count -eq 2) {
            $looseLines.Add("line $lineNo loose format: missing optional tag -> $line")
        }

        $validCount++
    }
}
finally {
    $reader.Close()
}

Write-Host ""
Write-Host "Validation result"
Write-Host "================="
Write-Host "Valid lines   : $validCount"
Write-Host "Loose lines   : $($looseLines.Count)"
Write-Host "Invalid lines : $($invalidLines.Count)"
Write-Host ""

Write-Host "Invalid format"
Write-Host "--------------"
if ($invalidLines.Count -eq 0) {
    Write-Host "None"
} else {
    $invalidLines | ForEach-Object { Write-Host $_ }
}

Write-Host ""
Write-Host "Loose format"
Write-Host "------------"
if ($looseLines.Count -eq 0) {
    Write-Host "None"
} else {
    $looseLines | ForEach-Object { Write-Host $_ }
}

if ($invalidLines.Count -gt 0) {
    exit 1
}

exit 0