# 公式文法规范

本文档描述 Finkit（Finkit）公式引擎的完整文法，基于 `core/src/formula/grammar.pest` 导出为 **EBNF** 格式，并标注通达信（TDX）、同花顺（THS）、大智慧（DZH）各平台方言差异。

实现文件：`core/src/formula/grammar.pest`

---

## 概述

Finkit 公式语言兼容国内主流行情软件的指标语法，支持：

- 变量赋值（`:=` / `=`）与输出（`:`）
- 算术、逻辑、比较运算符
- 内置函数调用与数组下标访问
- 条件分支（`IF-THEN-ELSE`）、循环（`FOR` / `WHILE`）
- 绘图指令（`DRAWTEXT`、`STICKLINE` 等）
- 参数声明（`PARAMS:`）
- 多平台注释风格

---

## EBNF 文法

以下 EBNF 由 `grammar.pest` 规则直接映射。符号约定：

| 符号 | 含义 |
|------|------|
| `?` | 可选（0 或 1 次） |
| `*` | 重复（0 或更多） |
| `+` | 重复（1 或更多） |
| `\|` | 选择 |
| `"..."` | 字面量 |

### 顶层结构

```ebnf
(* 程序入口 *)
program          = [ param_decl ";" ] statement { ";" statement }

(* 参数声明 *)
param_decl       = "PARAMS" ":" param_item { "," param_item }
param_item       = identifier "(" number "," number "," number ")"

(* 语句 *)
statement        = compound_assignment
                 | assignment
                 | output
                 | draw_line | draw_band | draw_kline | draw_rect
                 | fill_rgn | part_line | poly_line | draw_gbk
                 | draw_text | draw_icon | stick_line | draw_sl
                 | draw_text_fix | draw_number | vert_line
                 | if_then_else_stmt | for_stmt | while_stmt
                 | expression
```

**示例 — 参数声明：**

```
PARAMS: N1(5,100,12), N2(5,100,26);
```

**示例 — 多语句程序：**

```
DIFF:=EMA(CLOSE,12)-EMA(CLOSE,26);DEA:=EMA(DIFF,9);MACD:=2*(DIFF-DEA);
```

---

### 赋值与输出

```ebnf
assignment       = identifier ( ":=" | "=" ) expression
compound_assignment = identifier compound_op expression
compound_op      = "+=" | "-=" | "*=" | "/="

output           = identifier ":" expression { "," output_attr }
output_attr      = color_spec | line_style | draw_modifier | point_style
```

| 平台 | 赋值运算符 | 说明 |
|------|-----------|------|
| TDX | `:=` | 标准赋值 |
| DZH | `=` | 大智慧兼容单等号赋值 |
| TDX/THS | `:` | 输出语句（绘图输出） |

**示例 — 赋值（TDX `:=`）：**

```
RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100;
```

**示例 — 赋值（DZH `=`）：**

```
涨幅=(C-REF(C,1))/REF(C,1)*100
```

**示例 — 输出带样式：**

```
MA5:MA(C,5),COLORRED,LINETHICK2
```

**示例 — 复合赋值：**

```
SUM_VAL += CLOSE;
```

---

### 控制流

```ebnf
if_then_else_stmt = "IF" expression "THEN" expression "ELSE" expression

for_stmt         = "FOR" identifier for_assign_op expression "TO" expression "DO" statement+ "END"
for_assign_op    = ":=" | "="

while_stmt       = "WHILE" expression "DO" statement+ "END"
```

**示例 — IF-THEN-ELSE：**

```
信号:IF(CLOSE>MA(CLOSE,20),1,0)
```

**示例 — FOR 循环：**

```
FOR I:=1 TO 10 DO SUM:=SUM+CLOSE; END
```

**示例 — WHILE 循环：**

```
WHILE I<10 DO I:=I+1; END
```

---

### 表达式

```ebnf
expression       = logical_or

logical_or       = logical_xor { "OR" logical_xor }
logical_xor      = logical_and { "XOR" logical_and }
logical_and      = comparison { "AND" comparison }
comparison       = addition [ comp_op addition ]
comp_op          = ">=" | "<=" | "==" | "!=" | "<>" | ">" | "<"

addition         = multiplication { add_op multiplication }
add_op           = "+" | "-" | "&"

multiplication   = unary { mul_op unary }
mul_op           = "*" | "/" | "%"

unary            = [ "NOT" ] unary | [ "-" ] power
power            = postfix [ "^" postfix ]

postfix          = primary { index_access }
index_access     = "[" expression "]"

primary          = function_call | number | string | variable | "(" expression ")"
```

**示例 — 逻辑与比较：**

```
CLOSE>MA(CLOSE,20) AND VOL>REF(VOL,1)
```

**示例 — 算术与幂运算：**

```
(HIGH-LOW)/CLOSE*100
```

**示例 — 数组下标：**

```
CLOSE[5]
```

**示例 — 逻辑非：**

```
NOT CROSS(MA5,MA10)
```

---

### 函数与变量

```ebnf
function_call    = identifier "(" [ expression { "," expression } ] ")"
variable         = identifier
identifier       = ALPHA { ALPHANUM | "_" }   (* 排除内置关键字 *)
number           = DIGIT+ [ "." DIGIT+ ]
string           = '"' { CHAR } '"' | "'" { CHAR } "'"
bool_val         = "TRUE" | "FALSE"
```

**示例 — 函数调用：**

```
EMA(CLOSE,12)
```

**示例 — 多参数函数：**

```
PLUS_DI(HIGH,LOW,CLOSE,14)
```

**示例 — 字符串参数（绘图）：**

```
DRAWTEXT(CLOSE>OPEN,HIGH,"买入")
```

---

### 注释

```ebnf
COMMENT          = line_comment | hash_comment | block_comment | brace_comment
line_comment     = "//" { !"\n" ANY }
hash_comment     = "#" { !"\n" ANY }
block_comment    = "/*" { !"*/" ANY } "*/"
brace_comment    = "{" { !"}" ANY } "}"
```

| 风格 | 平台 | 示例 |
|------|------|------|
| `//` | TDX / THS / DZH | `// 计算MACD` |
| `#` | Pine Script 兼容 | `# 注释` |
| `/* */` | 通用 C 风格 | `/* 块注释 */` |
| `{ }` | TDX 花括号 | `{布林带三线}` |

**示例 — TDX 花括号注释：**

```
{KDJ三线}
RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100;
```

---

### 颜色与绘图修饰

```ebnf
color_spec       = color_name | color_rgb | color_hex
color_name       = "COLORRED" | "COLORGREEN" | "COLORBLUE" | "COLORYELLOW"
                 | "COLORWHITE" | "COLORBLACK" | "COLORCYAN" | "COLORMAGENTA" | "COLORGRAY"
color_rgb        = "COLOR(" DIGIT+ "," DIGIT+ "," DIGIT+ ")"
color_hex        = "COLORHEX(" HEX_DIGIT+ ")"

line_style       = "LINETHICK" DIGIT
draw_modifier    = "NODRAW" | "NOTEXT" | "NOAXIS" | "COLORAUTO"
point_style      = "POINTDOT" | "CIRCLEDOT" | "CROSSDOT" | "STICK"
                 | "VOLSTICK" | "LINESTICK" | "COLORSTICK"
```

**示例 — 绘图指令：**

```
STICKLINE(CLOSE>OPEN,CLOSE,OPEN,0.8,0),COLORRED
```

**示例 — 画线：**

```
DRAWLINE(HIGH=HHV(HIGH,20),HIGH,LOW=LLV(LOW,20),LOW,1)
```

**示例 — 文字标注：**

```
DRAWTEXT(CROSS(MA5,MA10),LOW,"金叉"),COLORRED
```

---

### 绘图指令完整列表

```ebnf
draw_text        = "DRAWTEXT" "(" expression "," expression "," string ")" [ "," color_spec ]
draw_icon        = "DRAWICON" "(" expression "," expression "," expression ")" [ "," color_spec ]
stick_line       = "STICKLINE" "(" expression "," expression "," expression "," expression "," bool_val ")" [ "," color_spec ]
draw_line        = "DRAWLINE" "(" expression "," expression "," expression "," expression "," expression ")" [ "," color_spec ]
draw_band        = "DRAWBAND" "(" expression "," color_spec "," expression "," color_spec ")"
draw_kline       = "DRAWKLINE" "(" expression "," expression "," expression "," expression ")"
draw_rect        = "DRAWRECTREL" "(" expression "," expression "," expression "," expression "," color_spec ")"
fill_rgn         = "FILLRGN" "(" expression "," expression "," expression ")" [ "," color_spec ]
part_line        = "PARTLINE" "(" expression "," expression ")" [ "," color_spec ]
poly_line        = "POLYLINE" "(" expression "," expression ")" [ "," color_spec ]
draw_gbk         = "DRAWGBK" "(" expression "," color_spec ")"
draw_sl          = "DRAWSL" "(" expression "," expression "," expression "," expression ")" [ "," color_spec ]
draw_text_fix    = "DRAWTEXT_FIX" "(" expression "," expression "," string ")" [ "," color_spec ]
draw_number      = "DRAWNUMBER" "(" expression "," expression "," expression "," expression ")" [ "," color_spec ]
vert_line        = "VERTLINE" "(" expression ")" [ "," color_spec ]
```

---

## 平台方言差异

### OHLCV 变量别名

| 标准名 | TDX | THS | DZH | 说明 |
|--------|-----|-----|-----|------|
| `CLOSE` | `C` | `C` | `C` | 收盘价 |
| `OPEN` | `O` | `O` | `O` | 开盘价 |
| `HIGH` | `H` | `H` | `H` | 最高价 |
| `LOW` | `L` | `L` | `L` | 最低价 |
| `VOLUME` | `V` / `VOL` | `V` / `VOL` | `V` / `VOL` | 成交量 |
| `AMOUNT` | `A` | `A` | `A` | 成交额 |

**示例 — 别名等价：**

```
MA(CLOSE,5)    (* TDX 标准 *)
MA(C,5)        (* 三平台通用短别名 *)
```

### 同花顺（THS）历史引用别名

| THS 别名 | 等价表达式 | 说明 |
|----------|-----------|------|
| `CLOSE1` | `REF(CLOSE,1)` | 昨收 |
| `OPEN1` | `REF(OPEN,1)` | 昨开 |
| `HIGH1` | `REF(HIGH,1)` | 昨高 |
| `LOW1` | `REF(LOW,1)` | 昨低 |
| `VOL1` | `REF(VOL,1)` | 昨量 |

**示例：**

```
昨收:=CLOSE1;
涨幅:=(CLOSE-CLOSE1)/CLOSE1*100;
```

### 大智慧（DZH）特有语法

| 特性 | 说明 | 示例 |
|------|------|------|
| 单等号赋值 | `=` 代替 `:=` | `涨幅=(C-REF(C,1))/REF(C,1)*100` |
| 板块引用 | `BLOCKINDEX` / `BLOCKAVG` | `BLOCKINDEX('科技板块')` |
| 板块数据 | `BLOCKDATA` | `BLOCKDATA('科技板块','PCT')` |
| 动态行情 | `DYNAINFO(N)` | `DYNAINFO(3)` 最新价 |

**示例 — DZH 涨跌幅选股：**

```
涨幅:=(C-REF(C,1))/REF(C,1)*100;
选股:涨幅>REF(涨幅,1) AND 涨幅>3;
```

### 跨周期引用

Finkit 通过 `FormulaContext::with_period_data()` 提供跨周期数据，语法函数：

| 函数 | 说明 | 返回值 |
|------|------|--------|
| `PERIODTYPE()` | 当前周期类型 | 1=日 / 2=周 / 3=月 |
| `REFDATE(X, IDX)` | 引用指定 Bar 索引的值 | 数组 |

设置跨周期数据时使用键名 `WEEK` / `MONTH`（对应 `#WEEK` / `#MONTH` 语义）。

**示例 — 日线引用昨收：**

```
昨收:=REFDATE(CLOSE,1);
涨跌幅:=(CLOSE-昨收)/昨收*100;
```

**示例 — 周期类型判断：**

```
日线信号:IF(PERIODTYPE()=1,CLOSE>MA(CLOSE,20),0);
```

### 通达信（TDX）特有

| 特性 | 说明 | 示例 |
|------|------|------|
| 动态行情 | `DYNAINFO(N)` | `DYNAINFO(4)` 最高价 |
| 财务数据 | `FINANCE(N)` | `FINANCE(40)` 流通股本 |
| 筹码函数 | `WINNER` / `COST` | `WINNER(CLOSE)` |
| 花括号注释 | `{注释}` | `{MACD指标}` |

**示例 — TDX 筹码：**

```
获利比例:=WINNER(CLOSE)*100;
平均成本:=COST(50);
```

---

## 关键字列表

以下标识符被解析器保留为关键字，**不能**用作变量名（`IF` 可作为函数名）：

```
THEN  ELSE  AND  OR  XOR  NOT  TRUE  FALSE  PARAMS
FOR  WHILE  DO  END  TO
DRAWTEXT  DRAWICON  STICKLINE  DRAWLINE  DRAWBAND  DRAWKLINE
DRAWRECTREL  FILLRGN  PARTLINE  POLYLINE  DRAWGBK  DRAWSL
DRAWTEXT_FIX  DRAWNUMBER  VERTLINE
LINETHICK  NODRAW  NOTEXT  NOAXIS  COLORAUTO
POINTDOT  CIRCLEDOT  CROSSDOT  STICK  VOLSTICK  LINESTICK  COLORSTICK
COLORRED  COLORGREEN  COLORBLUE  COLORYELLOW  COLORWHITE  COLORBLACK
COLORCYAN  COLORMAGENTA  COLORGRAY  COLOR  COLORHEX
```

---

## 运算符优先级（从高到低）

| 优先级 | 运算符 / 结构 | 示例 |
|--------|--------------|------|
| 1 | 函数调用、下标 `[]` | `MA(C,5)[1]` |
| 2 | 一元 `NOT`、一元 `-` | `NOT A`, `-CLOSE` |
| 3 | 幂 `^` | `CLOSE^2` |
| 4 | `*` `/` `%` | `HIGH/LOW` |
| 5 | `+` `-` `&` | `CLOSE-OPEN` |
| 6 | 比较 `>` `<` `>=` `<=` `==` `!=` `<>` | `CLOSE>MA(C,20)` |
| 7 | `AND` | `A AND B` |
| 8 | `XOR` | `A XOR B` |
| 9 | `OR` | `A OR B` |

---

## 完整公式示例

### MACD（TDX）

```
DIFF:=EMA(CLOSE,12)-EMA(CLOSE,26);
DEA:=EMA(DIFF,9);
MACD:=2*(DIFF-DEA);
```

### KDJ（TDX）

```
RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100;
K:=SMA(RSV,3,1);
D:=SMA(K,3,1);
J:=3*K-2*D;
```

### 均线系统（THS）

```
MA5:MA(CLOSE,5);
MA10:MA(CLOSE,10);
MA20:MA(CLOSE,20);
MA60:MA(CLOSE,60);
```

### TRIX（DZH）

```
TR:=EMA(EMA(EMA(CLOSE,12),12),12);
TRIX:=(TR-REF(TR,1))/REF(TR,1)*100;
TRMA:=MA(TRIX,9);
```

---

## 相关资源

- 实现源码：`core/src/formula/grammar.pest`
- 解析器：`core/src/formula/parser.rs`
- 公式语料回归集：`tests/formula_corpus/`
- 公式系统参考：`docs/formula.md`
- 公式模板库：`docs/formula-templates.md`
