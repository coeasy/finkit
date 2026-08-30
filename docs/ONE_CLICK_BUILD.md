# One-Click Build · 一键构建全部语言

仓库为"一次跑完 7 种语言构建 + 安装 + 冒烟测试"提供了脚本封装，产物统一输出到 `dist/`。

## 1. 方式一：Docker（最省事，工具链最全）

`Dockerfile` 已预装 Rust + 各语言工具链：

```bash
docker build -t alpha_ta/builder:latest .
docker run --rm -v "$(pwd)/dist:/work/dist" alpha_ta/builder:latest
```

配合 Compose：

```bash
docker-compose up
```

产物落在宿主机 `dist/{python,java,node,go,c,dotnet}`。

## 2. 方式二：本机脚本

```bash
./build-usage.sh                 # bash / zsh / Git-Bash
pwsh ./build-usage.ps1           # PowerShell 7+
```

或 `Makefile`：

```bash
make                    # 全部
make python             # 仅 Python wheel
make java / make node / make go / make c / make dotnet   # 逐个
make bench-vs-talib     # AlphaTA vs TA-Lib C 对照
make install-and-test   # 本地安装所有产物 + 冒烟
make docker-compose-up  # 走 Docker/Compose
```

## 3. 会执行什么

1. 各绑定按 [BUILD_GUIDE](./BUILD_GUIDE.md) 的步骤编译原生库；
2. 把产物复制到 `dist/` 对应目录；
3. 生成该语言的包/工程骨架（如 NuGet 结构）；
4. 跑冒烟测试（每个语言至少一个指标调用）；
5. 汇总 `dist/`。

## 4. 注意

- Windows 下请优先用 **PowerShell 7（`pwsh`）** 脚本或 Docker；旧 `cmd` 下部分步骤可能受限。
- 若只改核心指标，无需全量一键；直接 `cargo test -p finkit` 更快。
- 产物目录为 gitignore 的 `/dist/`，不会进入提交。