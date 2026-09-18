param(
    [Parameter(Mandatory = $true)]
    [string]$Root,

    [Parameter(Mandatory = $true)]
    [string]$PublisherName
)

$ErrorActionPreference = "Stop"
$releaseRoot = (Resolve-Path $Root).Path
$unpacked = Get-ChildItem -Path $releaseRoot -Directory -Filter "win*-unpacked" | Select-Object -First 1
if ($null -eq $unpacked) {
    throw "No unpacked Windows application found under $releaseRoot"
}

$requiredExecutables = @(
    (Join-Path $unpacked.FullName "Simplex.exe"),
    (Join-Path $unpacked.FullName "resources\runtime\node.exe")
)
foreach ($file in $requiredExecutables) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
        throw "Missing signed executable: $file"
    }
}

$files = @(
    Get-ChildItem -Path $unpacked.FullName -File -Recurse -Filter "*.exe" |
        ForEach-Object { $_.FullName }
)
$installer = Get-ChildItem -Path $releaseRoot -File -Filter "Simplex-*-win-*.exe" | Select-Object -First 1
if ($null -eq $installer) {
    throw "No Simplex NSIS installer found under $releaseRoot"
}
$files += $installer.FullName
$files = @($files | Sort-Object -Unique)

foreach ($file in $files) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
        throw "Missing signed executable: $file"
    }
    $signature = Get-AuthenticodeSignature -LiteralPath $file
    if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::Valid) {
        throw "Invalid Authenticode signature for $file`: $($signature.Status) $($signature.StatusMessage)"
    }
    $actualPublisher = $signature.SignerCertificate.GetNameInfo(
        [System.Security.Cryptography.X509Certificates.X509NameType]::SimpleName,
        $false
    )
    if ($actualPublisher -cne $PublisherName) {
        throw "$file is signed by '$actualPublisher', expected '$PublisherName'"
    }
    if ($null -eq $signature.TimeStamperCertificate) {
        throw "$file has no trusted timestamp"
    }
}

Write-Output "Verified Authenticode publisher and timestamp on every packaged executable and the NSIS installer"
