$ErrorActionPreference = 'SilentlyContinue'
$root = 'p:\llm_code\fta'
$excludes = @('\.git', '\.emsdk', '\.fakehome', '\.test_venv', '\.github', 'target', 'obj', 'bin', 'node_modules', 'dist', '\.cargo')
$rx = [regex]('(' + ($excludes -join ')|(') + ')')
Get-ChildItem -Path $root -Recurse -File -Force -ErrorAction SilentlyContinue |
  Where-Object { -not $rx.IsMatch($_.FullName) } |
  Select-String -SimpleMatch -Pattern 'AlphaTA' -List |
  Select-Object -ExpandProperty Path |
  Sort-Object -Unique
