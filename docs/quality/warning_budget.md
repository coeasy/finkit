# 警告预算（Warning Budget）

> 门禁：`python scripts/check_warning_budget.py`
> 基线：`scripts/warning_budget_baseline.json`
> 范围：`cargo clippy -p finkit --all-targets`

## 为什么是棘轮，不是待办清单

V4 记录过 ~5.5k 条 clippy 警告。把这么大的存量写成一个" someday 修完"的清单，
结果是它在几十个提交里被忘掉，然后新警告继续累加——清单越旧越没人信。

真正能生效的形式是**棘轮**：把当前每个 lint 的条数提交进基线，门禁在**任何 lint
上升**时失败。修一个 lint 就 lowering 一条基线；没有任何东西允许变多。这样存量
自然会往下走，而且每一步都可度量。

## 当前基线

**14647 条警告 / 125 个 lint**（2026-10-07 建立）。

| 条数 | lint | 性质 |
| --- | --- | --- |
| 2790 | `clippy::cast_precision_loss` | 有意为之：金融库全线 `f64` |
| 2165 | `clippy::missing_errors_doc` | 文档债，可批量补 |
| 1999 | `clippy::must_use_candidate` | 机械可加属性 |
| 1039 | `clippy::cast_lossless` | 同第一类 |
| 915 | `clippy::doc_markdown` | 纯文档 |
| 644 | `clippy::semicolon_if_nothing_returned` | 纯风格 |
| 403 | `clippy::cast_possible_truncation` | 有意为之 |
| 399 | `clippy::uninlined_format_args` | 机械 |
| 350 | `clippy::missing_panics_doc` | 文档债 |
| 345 | `clippy::return_self_not_must_use` | 机械 |
| 308 | `clippy::similar_names` | 可读性 |
| 288 | `clippy::unreadable_literal` | 可读性 |

前三类合计约 7000 条，占总量近一半，且**大部分不该直接修**：`cast_precision_loss`
在一个以 `f64` 为契约的金融库里修掉，等于把 `f32` 悄悄引入数值路径。这类应该
用 crate 级 `allow` 显式豁免并在代码里写明理由——把"有意为之"从噪声里摘出来，
预算才看得出真实趋势。

## 门禁实现里两个非显然的决定

1. **必须用 `--message-format=json`。** `short` 格式不带 lint 代码，只有一个总数
   的基线不构成棘轮："少了 1200 条"和"总数没变但多了 300 个
   `needless_range_loop`"在总数上长得一样。
2. **只统计指向仓库源文件的诊断。** 本机（沙箱 Windows）上
   `error deleting lock file for incremental compilation session directory`
   单独贡献了 110 条与代码无关的 "rustc" 警告；不过滤的话，预算的结论会随
   文件系统的心情漂移。判据是"任一 span 指向可读的 `.rs`"——用 `is_primary`
   会在宏展开处误杀约 400 条真实的 `clippy::float_cmp`。

## 用法

```bash
python scripts/check_warning_budget.py           # 门禁（CI 跑这个）
python scripts/check_warning_budget.py --write   # 修完 lint 后重定基线
python scripts/check_warning_budget.py --json    # 机器可读明细
```

`--write` 需要一次有意的评审：它同时是"我修干净了"和"我决定接受这些新警告"
两个动作。CI 里应当只在非 PR 提交上允许它。
