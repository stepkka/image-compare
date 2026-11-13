$ErrorActionPreference = 'Stop'
$classes = 'HKCU:\Software\Classes'
$extensions = @('.png','.jpg','.jpeg','.jfif','.gif','.webp','.bmp','.tif','.tiff','.ico')
# Explicit, app-owned registry keys only. Never remove extension or shell parents.
$keys = @("$classes\CLSID\{9A29AFC8-8445-49EE-B5D4-916E15CD4792}")
foreach ($extension in $extensions) {
    $keys += "$classes\SystemFileAssociations\$extension\shell\ImageCompare"
}
foreach ($key in $keys) {
    if (Test-Path -LiteralPath $key) { Remove-Item -LiteralPath $key -Recurse -Force }
}
Add-Type -Namespace ImageCompare -Name ShellNotify -MemberDefinition '[System.Runtime.InteropServices.DllImport("shell32.dll")] public static extern void SHChangeNotify(uint e, uint f, System.IntPtr a, System.IntPtr b);'
[ImageCompare.ShellNotify]::SHChangeNotify(0x08000000, 0, [IntPtr]::Zero, [IntPtr]::Zero)
Write-Host 'ImageCompare Explorer integration removed.'
