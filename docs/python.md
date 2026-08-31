# Python 安装与发布指南

Finkit 的 Python 绑定使用 PyO3 和 maturin 构建。发布包是带原生 Rust 扩展的 wheel，安装时不需要本地 Rust 工具链。

## 支持矩阵

### Python 版本

| Python | wheel | 说明 |
| --- | --- | --- |
| CPython 3.8–3.14 | ✅ | CI 为每个可用平台组合构建并执行测试 |
| PyPy | 未承诺 | 当前发布流程只构建 CPython wheel |
| CPython free-threaded（`python3.14t`） | 未承诺 | 不属于 v0.1.0 的发布矩阵 |

### 操作系统与架构

| 平台 | wheel 标签 |
| --- | --- |
| Linux x86_64 | `manylinux_2_17_x86_64` |
| macOS Intel | `macosx_*_x86_64` |
| macOS Apple Silicon | `macosx_*_arm64` |
| Windows x86_64 | `win_amd64` |

当前矩阵不包含 Linux ARM64、32 位 Windows 和 musllinux；macOS arm64 也不提供 CPython 3.8 wheel（该版本没有可用的官方 arm64 解释器）。使用这些平台或组合时请按下面的源码方式构建。

## 安装已构建 wheel

1. 在 [GitHub Releases](https://github.com/coeasy/finkit/releases) 下载与本机 Python、系统和架构匹配的 `finkit-0.1.0-*.whl`。
2. 在目标虚拟环境中安装：

```bash
python -m pip install --upgrade pip
python -m pip install ./finkit-0.1.0-<匹配本机的 wheel>.whl
```

如果对应 Release 尚未附带 wheel，可在仓库的 [Python wheels workflow](https://github.com/coeasy/finkit/actions/workflows/python-wheels.yml) 中下载同名 workflow artifact。安装本地 wheel 时，pip 会自动安装运行时依赖 NumPy。

验证安装：

```bash
python - <<'PY'
import finkit as ta
import numpy as np

close = np.arange(1.0, 101.0)
rsi = ta.rsi(close, timeperiod=14)
assert len(rsi) == len(close)
print(f"finkit loaded; RSI length={len(rsi)}")
PY
```

### 如何选择 wheel

wheel 文件名中的标签对应以下信息：

- `cp311`：CPython 3.11；`cp314`：CPython 3.14。
- `manylinux_2_17_x86_64`：Linux x86_64，glibc 2.17 或更高。
- `macosx_*_arm64`：Apple Silicon；`macosx_*_x86_64`：Intel Mac。
- `win_amd64`：64 位 Windows。

不要只按文件名中的 Python 版本选择；系统和 CPU 架构标签也必须匹配。通常让 pip 直接安装目录中的 wheel 最安全：

```bash
python -m pip install ./dist/finkit-0.1.0-*.whl
```

## 从源码安装

源码安装适合尚未提供 wheel 的平台、开发工作和需要修改 Rust 核心的场景。

### 前置条件

- CPython 3.8–3.14（推荐使用仍受上游支持的版本）
- Rust stable，且能满足工作区的 MSRV：Rust 1.85+
- Python、pip 和虚拟环境
- Linux 需要 C 编译器；macOS 需要 Xcode Command Line Tools；Windows 需要 Visual Studio C++ Build Tools
- NumPy（绑定函数使用 NumPy 一维数组作为输入）

### Linux / macOS

```bash
git clone https://github.com/coeasy/finkit.git
cd finkit

python3 -m venv .venv
source .venv/bin/activate
python -m pip install --upgrade pip
python -m pip install "maturin>=1.5,<2.0" "numpy>=1.24" pytest

cd ffi/python-binding
maturin develop --release

cd ../..
python -m pytest ffi/python-binding/tests -q
```

### Windows PowerShell

```powershell
git clone https://github.com/coeasy/finkit.git
Set-Location finkit

py -3.11 -m venv .venv
.\.venv\Scripts\Activate.ps1
python -m pip install --upgrade pip
python -m pip install "maturin>=1.5,<2.0" "numpy>=1.24" pytest

Set-Location ffi/python-binding
maturin develop --release

Set-Location ../..
python -m pytest ffi/python-binding/tests -q
```

## 构建单个 wheel

在 `ffi/python-binding` 目录执行：

```bash
python -m pip install "maturin>=1.5,<2.0"
maturin build --release --locked --out dist
python -m pip install ./dist/finkit-0.1.0-*.whl
```

使用多个本地 Python 解释器时，对每个解释器分别构建：

```bash
for py in python3.8 python3.9 python3.10 python3.11 python3.12 python3.13 python3.14; do
  "$py" -m pip install "maturin>=1.5,<2.0"
  maturin build --release --locked --interpreter "$py" --out dist
done
```

上面的循环要求这些命令都在 PATH 中；Windows 可在 PowerShell 中对 `py -3.8`、`py -3.9` 等解释器逐个执行同一条 `maturin build` 命令。CI 使用同样的逐解释器策略，并额外构建 Linux manylinux wheel。

## NumPy 和 Pandas 用法

指标函数接收一维、`float64` NumPy 数组。Pandas 不是运行时必需依赖；使用 DataFrame 时显式转换列：

```python
import finkit as ta
import numpy as np
import pandas as pd

df = pd.DataFrame({"close": np.arange(1.0, 101.0)})
close = df["close"].to_numpy(dtype=np.float64, copy=False)

df["rsi"] = ta.rsi(close, timeperiod=14)
macd, signal, hist = ta.macd(close, fastperiod=12, slowperiod=26, signalperiod=9)
df["macd"] = macd
df["signal"] = signal
df["hist"] = hist
```

带有 `df.ta` accessor 的 pandas 集成属于可选能力。要运行对应测试：

```bash
python -m pip install pandas
python -m pytest ffi/python-binding/tests/test_accessor.py -q
```

## 开发与发布检查

```bash
cd ffi/python-binding

# 构建源码包
maturin sdist --out dist

# 构建 wheel
maturin build --release --locked --out dist

# 安装 wheel 后运行完整 Python 测试
python -m pip install ./dist/finkit-0.1.0-*.whl
cd ../..
python -m pytest ffi/python-binding/tests -q
```

每次推送到 `main`、创建 pull request 或推送 `v*` tag 时，GitHub Actions 的 Python wheels workflow 会为 4 个平台和 7 个 CPython 小版本构建 wheel，并将每个 wheel 作为独立 artifact 上传。发布流程还应在创建 Release 时把这些 artifacts 附加到 Release，避免用户直接下载源码包。

## 常见问题

### `No matching distribution found` 或 `is not a supported wheel`

当前解释器、系统或架构与 wheel 标签不匹配。先查看：

```bash
python -VV
python -c "import platform; print(platform.system(), platform.machine())"
```

然后选择对应的 `cpXY`、系统和架构 wheel。注意 32 位 Python 不能安装 `win_amd64`。

### `ModuleNotFoundError: No module named 'finkit'`

确认 pip 和 python 属于同一个虚拟环境：

```bash
python -m pip show finkit
python -c "import sys; print(sys.executable)"
```

不要在源码目录中直接验证已安装 wheel；当前目录下的 `finkit/` 可能遮蔽 site-packages。切换到临时目录后再执行 import。

### `ImportError: numpy.core.multiarray failed to import`

先升级 pip 和 NumPy，再重新安装 wheel：

```bash
python -m pip install --upgrade pip numpy
python -m pip install --force-reinstall ./finkit-0.1.0-*.whl
```

### 源码构建找不到 Rust 或链接器

确认 `rustc --version` 满足 MSRV 1.85+，并安装对应平台的 C/C++ 编译工具链。Windows 还要使用 64 位 Python 与 MSVC 工具链。

## 相关文档

- [Python binding README](../ffi/python-binding/README.md)
- [总安装指南](installation.md)
- [开发指南](development.md)
- [发布页](https://github.com/coeasy/finkit/releases)

## 自动发布 Release wheel

当向仓库推送符合 `vX.Y.Z` 格式的版本 tag 时，`Python wheels` workflow 会：

1. 为支持的 Python 与操作系统组合构建 wheel；
2. 在每个目标环境中安装 wheel 并运行 Python 绑定测试；
3. 汇总 27 个 wheel，使用 `twine check` 校验元数据与兼容性标签；
4. 将全部 wheel 自动上传到对应的 GitHub Release。

如果需要为已经存在的 Release 补发 wheel，可在 Actions 页面手动运行该 workflow，并填写 `release_tag`，例如 `v0.1.0`。手动补发使用当前 `main` 的源码，因此应先确认源码版本与目标 tag 一致。

构建失败时不会执行 Release 上传步骤；只有全部 wheel 构建、安装测试和汇总校验通过后才会发布。