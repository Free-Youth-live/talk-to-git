$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root 'posters/1024_freeyouth'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$W = 1080
$H = 1440

function RGB([int]$r, [int]$g, [int]$b) {
    return [System.Drawing.Color]::FromArgb($r, $g, $b)
}
function New-Canvas([System.Drawing.Color]$color) {
    $bmp = New-Object System.Drawing.Bitmap $W, $H
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $g.Clear($color)
    return @($bmp, $g)
}
function Box($g, $color, [float]$x, [float]$y, [float]$w, [float]$h) {
    $brush = New-Object System.Drawing.SolidBrush $color
    $g.FillRectangle($brush, $x, $y, $w, $h)
    $brush.Dispose()
}
function Ln($g, $color, [float]$x1, [float]$y1, [float]$x2, [float]$y2, [float]$width = 1) {
    $pen = New-Object System.Drawing.Pen $color, $width
    $g.DrawLine($pen, $x1, $y1, $x2, $y2)
    $pen.Dispose()
}
function Dot($g, $color, [float]$x, [float]$y, [float]$d) {
    $brush = New-Object System.Drawing.SolidBrush $color
    $g.FillEllipse($brush, $x, $y, $d, $d)
    $brush.Dispose()
}
function Txt($g, [string]$text, [string]$family, [float]$size, $color, [float]$x, [float]$y, [switch]$bold) {
    $style = if ($bold) { [System.Drawing.FontStyle]::Bold } else { [System.Drawing.FontStyle]::Regular }
    $font = [System.Drawing.Font]::new($family, $size, $style, [System.Drawing.GraphicsUnit]::Pixel)
    $brush = New-Object System.Drawing.SolidBrush $color
    $g.DrawString($text, $font, $brush, $x, $y)
    $font.Dispose()
    $brush.Dispose()
}
function Save-Poster($bmp, $g, [string]$name) {
    $path = Join-Path $out $name
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose()
    $bmp.Dispose()
}

function Blue-Poster {
    $bmp, $g = New-Canvas (RGB 7 17 31)
    $white = RGB 236 244 255
    $cyan = RGB 80 224 238
    $muted = RGB 129 164 190
    $purple = RGB 154 112 239
    Box $g (RGB 14 39 63) 650 0 430 1440
    Box $g $cyan 60 72 8 160
    Txt $g '1024' 'Cascadia Mono' 164 $white 86 74 $true
    Txt $g '程序员节' 'Noto Sans SC' 48 $cyan 94 300 $true
    Txt $g '10.24 / DEVELOPER DAY' 'Cascadia Mono' 22 $muted 94 374
    for ($i = 0; $i -lt 11; $i++) {
        $bits = ('01001000','01100101','01101100','01101100','01101111','00100000','01010111','01101111','01110010','01101100','01100100')[$i]
        Txt $g $bits 'Cascadia Mono' 20 (RGB 44 94 123) 680 (70 + $i * 45)
    }
    Ln $g (RGB 38 84 116) 86 540 996 540 2
    Ln $g $cyan 160 780 350 650 4
    Ln $g $cyan 350 650 540 780 4
    Ln $g $purple 540 780 730 650 4
    Ln $g $purple 730 650 900 780 4
    Dot $g $cyan 132 752 56
    Dot $g $cyan 322 622 56
    Dot $g $purple 512 752 56
    Dot $g $purple 702 622 56
    Dot $g $cyan 872 752 56
    Txt $g 'commit' 'Cascadia Mono' 22 $muted 112 850
    Txt $g 'review' 'Cascadia Mono' 22 $muted 490 850
    Txt $g 'merge' 'Cascadia Mono' 22 $muted 830 850
    Txt $g '不羁的青春' 'Noto Sans SC' 76 $white 76 1020 $true
    Txt $g 'FreeYouth' 'Cascadia Mono' 40 $cyan 84 1145
    Txt $g '把自由写进代码，把热爱留在现场' 'Noto Sans SC' 28 $white 82 1240
    Txt $g '10.24  程序员节快乐' 'Noto Sans SC' 24 $muted 84 1322
    Save-Poster $bmp $g '01_blue_binary.png'
}

function Terminal-Poster {
    $bmp, $g = New-Canvas (RGB 5 7 7)
    $green = RGB 114 239 132
    $dim = RGB 93 142 100
    $white = RGB 224 239 225
    $gray = RGB 39 61 45
    Box $g $gray 58 84 964 770
    Box $g (RGB 12 20 14) 60 128 960 724
    Dot $g (RGB 232 96 89) 84 105 16
    Dot $g (RGB 230 192 78) 110 105 16
    Dot $g (RGB 105 197 93) 136 105 16
    Txt $g 'Terminal — FreeYouth' 'Cascadia Mono' 20 $dim 178 101
    Txt $g 'PS D:\ttg> git status' 'Cascadia Mono' 25 $green 94 184
    Txt $g 'On branch main' 'Cascadia Mono' 23 $dim 94 240
    Txt $g 'nothing to commit, working tree clean' 'Cascadia Mono' 23 $dim 94 282
    Txt $g 'PS D:\ttg> git log --oneline -3' 'Cascadia Mono' 25 $green 94 362
    Txt $g 'a1024f3 keep learning' 'Cascadia Mono' 23 $dim 94 420
    Txt $g 'b2048af make it useful' 'Cascadia Mono' 23 $dim 94 462
    Txt $g 'c4096be stay free' 'Cascadia Mono' 23 $dim 94 504
    Txt $g 'PS D:\ttg> git commit -m "不羁的青春"' 'Noto Sans SC' 27 $green 94 594
    Txt $g 'write code. stay free.' 'Cascadia Mono' 25 $white 94 690
    Txt $g 'PS D:\ttg> _' 'Cascadia Mono' 25 $green 94 748
    Txt $g '1024' 'Cascadia Mono' 168 $green 62 930 $true
    Txt $g '不羁的青春' 'Noto Sans SC' 72 $white 68 1130 $true
    Txt $g '今天，给每一段代码一点自由' 'Noto Sans SC' 28 $green 72 1250
    Txt $g 'FreeYouth  /  10.24' 'Cascadia Mono' 24 $dim 72 1330
    Save-Poster $bmp $g '02_terminal_green.png'
}

function Magazine-Poster {
    $bmp, $g = New-Canvas (RGB 247 242 234)
    $black = RGB 33 34 39
    $orange = RGB 235 91 62
    $purple = RGB 94 67 155
    $cream = RGB 247 242 234
    Box $g $purple 820 0 260 510
    Box $g $orange 0 550 1080 24
    Txt $g 'FREEYOUTH / FIELD NOTES' 'Cascadia Mono' 22 $black 68 62 $true
    Txt $g '1024' 'Cascadia Mono' 250 $orange 52 156 $true
    Txt $g '10.24' 'Cascadia Mono' 34 $cream 864 66 $true
    Txt $g '程序员节' 'Noto Sans SC' 52 $black 70 620 $true
    Txt $g '不羁的青春' 'Noto Sans SC' 82 $black 62 740 $true
    Txt $g '写代码的人，也值得被看见' 'Noto Sans SC' 32 $purple 70 900
    Ln $g $black 70 1000 1000 1000 3
    Txt $g '把每一次提交，都写成热爱' 'Noto Sans SC' 36 $orange 70 1080 $true
    Txt $g 'FreeYouth' 'Cascadia Mono' 36 $black 72 1250
    Txt $g 'CODE / CREATE / STAY FREE' 'Cascadia Mono' 20 $purple 72 1320
    Dot $g $orange 870 1120 120
    Dot $g $purple 910 1190 48
    Save-Poster $bmp $g '03_magazine_orange_purple.png'
}

function Minimal-Poster {
    $bmp, $g = New-Canvas (RGB 245 247 250)
    $black = RGB 29 39 52
    $blue = RGB 48 110 210
    $muted = RGB 100 116 134
    Box $g $blue 42 42 996 2
    Box $g $blue 42 42 2 1356
    Box $g $blue 1036 42 2 1356
    Box $g $blue 42 1396 996 2
    Txt $g '10 / 24' 'Cascadia Mono' 56 $blue 82 104 $true
    Txt $g '程序员节' 'Noto Sans SC' 68 $black 80 240 $true
    Ln $g $blue 84 374 300 374 5
    Txt $g '不羁的青春' 'Noto Sans SC' 76 $black 80 500 $true
    Txt $g 'FreeYouth' 'Cascadia Mono' 42 $blue 86 640
    Txt $g '今天，向写代码的人致敬' 'Noto Sans SC' 30 $black 82 820
    Txt $g '代码有边界，青春不设限' 'Noto Sans SC' 28 $blue 82 930
    Ln $g $blue 84 1070 360 1070 2
    Txt $g 'FREEYOUTH / 2026.10.24' 'Cascadia Mono' 20 $muted 84 1140
    Txt $g 'KEEP BUILDING. KEEP FREE.' 'Cascadia Mono' 22 $black 84 1230 $true
    Save-Poster $bmp $g '04_minimal_white.png'
}

Blue-Poster
Terminal-Poster
Magazine-Poster
Minimal-Poster
Write-Output "created 4 posters in $out"
