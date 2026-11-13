param([string]$ExePath = (Join-Path (Split-Path $PSScriptRoot -Parent) 'target\release\imagecompare.exe'))
$ErrorActionPreference = 'Stop'
$exe = (Resolve-Path -LiteralPath $ExePath).Path
$classes = 'HKCU:\Software\Classes'
$clsid = '{9A29AFC8-8445-49EE-B5D4-916E15CD4792}'
$extensions = @('.png','.jpg','.jpeg','.jfif','.gif','.webp','.bmp','.tif','.tiff','.ico')
# Register only our own CLSID and verb; default file associations stay intact.
$server = "$classes\CLSID\$clsid\LocalServer32"
New-Item -Path $server -Force | Out-Null
Set-Item -LiteralPath $server -Value ('"' + $exe + '" --shell-server')
foreach ($extension in $extensions) {
    $verb = "$classes\SystemFileAssociations\$extension\shell\ImageCompare"
    New-Item -Path "$verb\DropTarget" -Force | Out-Null
    Set-Item -LiteralPath $verb -Value 'Compare with ImageCompare'
    New-ItemProperty -LiteralPath $verb -Name 'Icon' -Value ('"' + $exe + '",0') -PropertyType String -Force | Out-Null
    New-ItemProperty -LiteralPath $verb -Name 'MultiSelectModel' -Value 'Player' -PropertyType String -Force | Out-Null
    New-ItemProperty -LiteralPath "$verb\DropTarget" -Name 'CLSID' -Value $clsid -PropertyType String -Force | Out-Null
}
Add-Type -Namespace ImageCompare -Name ShellNotify -MemberDefinition '[System.Runtime.InteropServices.DllImport("shell32.dll")] public static extern void SHChangeNotify(uint e, uint f, System.IntPtr a, System.IntPtr b);'
[ImageCompare.ShellNotify]::SHChangeNotify(0x08000000, 0, [IntPtr]::Zero, [IntPtr]::Zero)
Write-Host "Installed Explorer integration for: $exe"
Write-Host 'Select images, right-click, and choose Compare with ImageCompare.'
Write-Host 'On Windows 11, use Show more options if needed.'
