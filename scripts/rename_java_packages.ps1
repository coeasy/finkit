$dir = 'p:\llm_code\alpha_ta\ffi\java-binding\java\src\main\java\com\alphata'
Get-ChildItem -Path $dir -Filter '*.java' | ForEach-Object {
    $content = Get-Content $_.FullName -Raw
    $content = $content -replace 'package com\.alpha_ta;', 'package com.alphata;'
    $content = $content -replace 'import com\.alpha_ta\.', 'import com.alphata.'
    Set-Content $_.FullName $content -NoNewline
    Write-Host "Updated: $($_.Name)"
}
