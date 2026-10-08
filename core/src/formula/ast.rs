/// AST节点类型
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AstNode {
    /// 数值常量
    Number(f64),
    /// 字符串常量
    StringLit(String),
    /// 变量引用（如 C, CLOSE）
    Variable(String),
    /// 二元运算（+、-、*、/、>、<、AND、OR等）
    BinaryOp {
        op: BinaryOperator,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    /// 一元运算（NOT、负号等）
    UnaryOp {
        op: UnaryOperator,
        expr: Box<AstNode>,
    },
    /// 函数调用
    FunctionCall { name: String, args: Vec<AstNode> },
    /// 数组元素访问（expr\[index\]）
    IndexAccess {
        array: Box<AstNode>,
        index: Box<AstNode>,
    },
    /// 变量赋值（:=）
    Assignment { name: String, expr: Box<AstNode> },
    /// 复合赋值（+=, -=, *=, /=）
    CompoundAssignment {
        name: String,
        op: CompoundAssignOp,
        expr: Box<AstNode>,
    },
    /// 输出变量（:）
    Output {
        name: String,
        expr: Box<AstNode>,
        modifier: Option<OutputModifier>,
    },
    /// 语句序列
    Statements(Vec<AstNode>),
    /// 参数声明
    ParamDecl {
        name: String,
        min: f64,
        max: f64,
        default: f64,
    },
    /// 绘图指令
    DrawText {
        cond: Box<AstNode>,
        price: Box<AstNode>,
        text: String,
        color: Option<ColorSpec>,
    },
    DrawIcon {
        cond: Box<AstNode>,
        price: Box<AstNode>,
        icon: Box<AstNode>,
        color: Option<ColorSpec>,
    },
    StickLine {
        cond: Box<AstNode>,
        price1: Box<AstNode>,
        price2: Box<AstNode>,
        width: Box<AstNode>,
        empty: bool,
        color: Option<ColorSpec>,
    },
    /// Generic draw command (DRAWLINE, DRAWBAND, FILLRGN, etc.)
    DrawGeneric {
        command: String,
        args: Vec<AstNode>,
        color: Option<ColorSpec>,
    },
    /// IF-THEN-ELSE 语句
    IfThenElse {
        cond: Box<AstNode>,
        then_branch: Box<AstNode>,
        else_branch: Box<AstNode>,
    },
    /// For循环
    ForLoop {
        var: String,
        start: Box<AstNode>,
        end: Box<AstNode>,
        body: Vec<AstNode>,
    },
    /// While循环
    WhileLoop {
        cond: Box<AstNode>,
        body: Vec<AstNode>,
    },
}

/// Replacement leaf used by the iterative teardown below: cheap to construct,
/// carries no children, and never appears in a live AST afterwards.
fn ast_placeholder() -> AstNode {
    AstNode::Number(f64::NAN)
}

fn take_boxed(node: &mut Box<AstNode>) -> AstNode {
    std::mem::replace(node.as_mut(), ast_placeholder())
}

impl AstNode {
    /// Move every child subtree out of `self`, leaving a placeholder leaf in
    /// each vacated slot.
    ///
    /// This is the machinery behind [`AstNode::dismantle_ast`]: children are
    /// harvested onto an explicit worklist instead of being dropped through
    /// recursive `Box` teardown.
    fn take_children(&mut self) -> Vec<AstNode> {
        match self {
            AstNode::BinaryOp { left, right, .. }
            | AstNode::IndexAccess {
                array: left,
                index: right,
            } => vec![take_boxed(left), take_boxed(right)],
            AstNode::UnaryOp { expr, .. }
            | AstNode::Assignment { expr, .. }
            | AstNode::CompoundAssignment { expr, .. }
            | AstNode::Output { expr, .. } => vec![take_boxed(expr)],
            AstNode::FunctionCall { args, .. } | AstNode::DrawGeneric { args, .. } => {
                std::mem::take(args)
            }
            AstNode::Statements(statements) => std::mem::take(statements),
            AstNode::DrawText { cond, price, .. } => {
                vec![take_boxed(cond), take_boxed(price)]
            }
            AstNode::DrawIcon {
                cond, price, icon, ..
            } => {
                vec![take_boxed(cond), take_boxed(price), take_boxed(icon)]
            }
            AstNode::StickLine {
                cond,
                price1,
                price2,
                ..
            } => vec![take_boxed(cond), take_boxed(price1), take_boxed(price2)],
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => vec![
                take_boxed(cond),
                take_boxed(then_branch),
                take_boxed(else_branch),
            ],
            AstNode::ForLoop {
                start, end, body, ..
            } => {
                let mut children = vec![take_boxed(start), take_boxed(end)];
                children.extend(std::mem::take(body));
                children
            }
            AstNode::WhileLoop { cond, body } => {
                let mut children = vec![take_boxed(cond)];
                children.extend(std::mem::take(body));
                children
            }
            AstNode::Number(_)
            | AstNode::StringLit(_)
            | AstNode::Variable(_)
            | AstNode::ParamDecl { .. } => Vec::new(),
        }
    }

    /// Depth of the subtree rooted here, measured with an explicit stack.
    ///
    /// Deliberately iterative: the trees this guards against are exactly the
    /// ones a recursive visitor would die on. See `parser::MAX_AST_DEPTH`.
    pub(crate) fn ast_depth(&self) -> usize {
        let mut max = 0usize;
        let mut stack: Vec<(&AstNode, usize)> = vec![(self, 1)];
        while let Some((node, depth)) = stack.pop() {
            max = max.max(depth);
            let child_depth = depth + 1;
            match node {
                AstNode::BinaryOp { left, right, .. }
                | AstNode::IndexAccess {
                    array: left,
                    index: right,
                } => {
                    stack.push((left, child_depth));
                    stack.push((right, child_depth));
                }
                AstNode::UnaryOp { expr, .. }
                | AstNode::Assignment { expr, .. }
                | AstNode::CompoundAssignment { expr, .. }
                | AstNode::Output { expr, .. } => stack.push((expr, child_depth)),
                AstNode::FunctionCall { args, .. } | AstNode::DrawGeneric { args, .. } => {
                    stack.extend(args.iter().map(|arg| (arg, child_depth)));
                }
                AstNode::Statements(statements) => {
                    stack.extend(statements.iter().map(|s| (s, child_depth)));
                }
                AstNode::DrawText { cond, price, .. } | AstNode::DrawIcon { cond, price, .. } => {
                    stack.push((cond, child_depth));
                    stack.push((price, child_depth));
                }
                AstNode::StickLine {
                    cond,
                    price1,
                    price2,
                    ..
                } => {
                    stack.push((cond, child_depth));
                    stack.push((price1, child_depth));
                    stack.push((price2, child_depth));
                }
                AstNode::IfThenElse {
                    cond,
                    then_branch,
                    else_branch,
                } => {
                    stack.push((cond, child_depth));
                    stack.push((then_branch, child_depth));
                    stack.push((else_branch, child_depth));
                }
                AstNode::ForLoop {
                    start, end, body, ..
                } => {
                    stack.push((start, child_depth));
                    stack.push((end, child_depth));
                    stack.extend(body.iter().map(|s| (s, child_depth)));
                }
                AstNode::WhileLoop { cond, body } => {
                    stack.push((cond, child_depth));
                    stack.extend(body.iter().map(|s| (s, child_depth)));
                }
                AstNode::Number(_)
                | AstNode::StringLit(_)
                | AstNode::Variable(_)
                | AstNode::ParamDecl { .. } => {}
            }
        }
        max
    }

    /// Tear this tree down with an explicit stack instead of recursive
    /// `Box`-drop glue.
    ///
    /// The derived drop of a `Box<AstNode>` tree recurses once per level: a
    /// deeply nested expression — a flat `1+1+…` chain the parser builds
    /// iteratively, or an AST assembled programmatically — would overflow the
    /// stack *while being dropped*, even after a check had already rejected
    /// it. A stack overflow is an abort, which no FFI `catch_unwind` can
    /// catch. The parser calls this on every tree it rejects for excessive
    /// depth, so the rejected input never reaches recursive teardown.
    pub(crate) fn dismantle_ast(self) {
        let mut stack = vec![self];
        while let Some(mut node) = stack.pop() {
            stack.extend(node.take_children());
            // `node` now holds only placeholder leaves; its own (recursive)
            // drop is constant-depth.
        }
    }
}

impl AstNode {
    /// Whether this statement contributes the formula's numeric result.
    ///
    /// A formula's result is *the value of its last value-producing statement*,
    /// not simply its last statement. Two node families are side-effect-only:
    ///
    /// * **Drawing directives** — [`AstNode::DrawText`], [`AstNode::DrawIcon`],
    ///   [`AstNode::StickLine`] and [`AstNode::DrawGeneric`] push draw commands
    ///   into the context and return a scratch buffer. Selecting one as the
    ///   result would report that scratch buffer (which is always zeroed).
    /// * **Level markers** — an [`AstNode::Output`] tagged with
    ///   [`DrawModifier::LevelLine`], i.e. Pine's `hline`. It marks a price
    ///   level and carries no series of its own.
    ///
    /// Skipping these is what makes `plot(rsi)` the result of an RSI script
    /// whose final statements are `hline(70)` / `hline(30)`, instead of the
    /// constant `30`.
    ///
    /// Every statement is still *evaluated* for its side effects — this only
    /// decides which statement's value is reported.
    pub fn produces_value(&self) -> bool {
        match self {
            AstNode::DrawText { .. }
            | AstNode::DrawIcon { .. }
            | AstNode::StickLine { .. }
            | AstNode::DrawGeneric { .. } => false,
            AstNode::Output { modifier, .. } => !modifier
                .as_ref()
                .is_some_and(OutputModifier::is_level_marker),
            _ => true,
        }
    }
}

/// Index of the statement whose value a statement block reports.
///
/// This is the last statement that [`AstNode::produces_value`]. `None` means
/// every statement was side-effect-only, in which case callers keep their
/// existing placeholder result.
pub fn result_statement_index(statements: &[AstNode]) -> Option<usize> {
    statements.iter().rposition(AstNode::produces_value)
}

/// 颜色规格
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ColorSpec {
    Named(String),
    Rgb(u8, u8, u8),
    Hex(String),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LineStyle {
    pub width: u32,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DrawModifier {
    NoDraw,
    NoText,
    NoAxis,
    ColorAuto,
    /// The output is a *level marker* rather than a data series: it draws a
    /// horizontal line at a fixed price (Pine `hline`) and carries no data of
    /// its own. [`AstNode::produces_value`] therefore refuses to select it as a
    /// formula's result, otherwise a script ending in `hline(30)` would report
    /// the constant `30` as its entire output.
    LevelLine,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PointStyle {
    PointDot,
    CircleDot,
    CrossDot,
    Stick,
    VolStick,
    LineStick,
    ColorStick,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OutputModifier {
    pub line_style: Option<LineStyle>,
    pub draw_modifier: Option<DrawModifier>,
    pub point_style: Option<PointStyle>,
    pub color: Option<ColorSpec>,
}

impl OutputModifier {
    /// Whether this output is a level marker rather than a data series.
    pub fn is_level_marker(&self) -> bool {
        matches!(self.draw_modifier, Some(DrawModifier::LevelLine))
    }
}

/// 二元运算符
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,          // 算术
    StringConcat, // 字符串连接（&）
    Gt,
    Lt,
    Gte,
    Lte,
    Eq,
    Neq, // 比较
    And,
    Or,
    Xor, // 逻辑
}

/// 一元运算符
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UnaryOperator {
    Not,
    Neg,
}

/// 复合赋值运算符
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CompoundAssignOp {
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_number_node() {
        let node = AstNode::Number(42.0);
        match node {
            AstNode::Number(val) => assert_eq!(val, 42.0),
            _ => panic!("Expected Number node"),
        }
    }

    #[test]
    fn test_variable_node() {
        let node = AstNode::Variable(String::from("CLOSE"));
        match node {
            AstNode::Variable(name) => assert_eq!(name, "CLOSE"),
            _ => panic!("Expected Variable node"),
        }
    }

    #[test]
    fn test_binary_op_node() {
        let left = Box::new(AstNode::Variable(String::from("CLOSE")));
        let right = Box::new(AstNode::Number(10.0));
        let node = AstNode::BinaryOp {
            op: BinaryOperator::Add,
            left,
            right,
        };
        match node {
            AstNode::BinaryOp { op, left, right } => {
                assert_eq!(op, BinaryOperator::Add);
                match *left {
                    AstNode::Variable(name) => assert_eq!(name, "CLOSE"),
                    _ => panic!("Expected Variable in left"),
                }
                match *right {
                    AstNode::Number(val) => assert_eq!(val, 10.0),
                    _ => panic!("Expected Number in right"),
                }
            }
            _ => panic!("Expected BinaryOp node"),
        }
    }

    #[test]
    fn test_function_call_node() {
        let args = vec![
            AstNode::Variable(String::from("CLOSE")),
            AstNode::Number(20.0),
        ];
        let node = AstNode::FunctionCall {
            name: String::from("MA"),
            args,
        };
        match node {
            AstNode::FunctionCall { name, args } => {
                assert_eq!(name, "MA");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected FunctionCall node"),
        }
    }

    #[test]
    fn test_assignment_node() {
        let expr = Box::new(AstNode::BinaryOp {
            op: BinaryOperator::Add,
            left: Box::new(AstNode::Variable(String::from("CLOSE"))),
            right: Box::new(AstNode::Number(1.0)),
        });
        let node = AstNode::Assignment {
            name: String::from("UP"),
            expr,
        };
        match node {
            AstNode::Assignment { name, expr } => {
                assert_eq!(name, "UP");
                assert!(matches!(*expr, AstNode::BinaryOp { .. }));
            }
            _ => panic!("Expected Assignment node"),
        }
    }

    #[test]
    fn test_statements_node() {
        let stmts = vec![
            AstNode::Assignment {
                name: String::from("MA5"),
                expr: Box::new(AstNode::FunctionCall {
                    name: String::from("MA"),
                    args: vec![
                        AstNode::Variable(String::from("CLOSE")),
                        AstNode::Number(5.0),
                    ],
                }),
            },
            AstNode::Output {
                name: String::from("MA5"),
                expr: Box::new(AstNode::Variable(String::from("MA5"))),
                modifier: None,
            },
        ];
        let node = AstNode::Statements(stmts);
        match node {
            AstNode::Statements(stmts) => {
                assert_eq!(stmts.len(), 2);
                assert!(matches!(&stmts[0], AstNode::Assignment { .. }));
                assert!(matches!(&stmts[1], AstNode::Output { .. }));
            }
            _ => panic!("Expected Statements node"),
        }
    }

    #[test]
    fn test_param_decl_node() {
        let node = AstNode::ParamDecl {
            name: String::from("N"),
            min: 1.0,
            max: 100.0,
            default: 20.0,
        };
        match node {
            AstNode::ParamDecl {
                name,
                min,
                max,
                default,
            } => {
                assert_eq!(name, "N");
                assert_eq!(min, 1.0);
                assert_eq!(max, 100.0);
                assert_eq!(default, 20.0);
            }
            _ => panic!("Expected ParamDecl node"),
        }
    }

    #[test]
    fn test_draw_text_node() {
        let node = AstNode::DrawText {
            cond: Box::new(AstNode::Variable(String::from("COND"))),
            price: Box::new(AstNode::Variable(String::from("CLOSE"))),
            text: String::from("BUY"),
            color: None,
        };
        match node {
            AstNode::DrawText {
                cond, price, text, ..
            } => {
                assert!(matches!(*cond, AstNode::Variable { .. }));
                assert!(matches!(*price, AstNode::Variable { .. }));
                assert_eq!(text, "BUY");
            }
            _ => panic!("Expected DrawText node"),
        }
    }

    #[test]
    fn test_stick_line_node() {
        let node = AstNode::StickLine {
            cond: Box::new(AstNode::Variable(String::from("COND"))),
            price1: Box::new(AstNode::Variable(String::from("HIGH"))),
            price2: Box::new(AstNode::Variable(String::from("LOW"))),
            width: Box::new(AstNode::Number(2.0)),
            empty: false,
            color: None,
        };
        match node {
            AstNode::StickLine {
                cond,
                price1,
                price2,
                width,
                empty,
                ..
            } => {
                assert!(matches!(*cond, AstNode::Variable { .. }));
                assert!(matches!(*price1, AstNode::Variable { .. }));
                assert!(matches!(*price2, AstNode::Variable { .. }));
                assert!(matches!(*width, AstNode::Number { .. }));
                assert!(!empty);
            }
            _ => panic!("Expected StickLine node"),
        }
    }

    #[test]
    fn test_color_spec_named() {
        let color = ColorSpec::Named("COLORRED".to_string());
        match color {
            ColorSpec::Named(name) => assert_eq!(name, "COLORRED"),
            _ => panic!("Expected Named color"),
        }
    }

    #[test]
    fn test_color_spec_rgb() {
        let color = ColorSpec::Rgb(255, 0, 0);
        match color {
            ColorSpec::Rgb(r, g, b) => {
                assert_eq!(r, 255);
                assert_eq!(g, 0);
                assert_eq!(b, 0);
            }
            _ => panic!("Expected Rgb color"),
        }
    }

    #[test]
    fn test_color_spec_hex() {
        let color = ColorSpec::Hex("FF0000".to_string());
        match color {
            ColorSpec::Hex(hex) => assert_eq!(hex, "FF0000"),
            _ => panic!("Expected Hex color"),
        }
    }

    #[test]
    fn test_if_then_else_node() {
        let node = AstNode::IfThenElse {
            cond: Box::new(AstNode::Variable(String::from("COND"))),
            then_branch: Box::new(AstNode::Number(1.0)),
            else_branch: Box::new(AstNode::Number(0.0)),
        };
        match node {
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => {
                assert!(matches!(*cond, AstNode::Variable { .. }));
                assert!(matches!(*then_branch, AstNode::Number { .. }));
                assert!(matches!(*else_branch, AstNode::Number { .. }));
            }
            _ => panic!("Expected IfThenElse node"),
        }
    }

    #[test]
    fn test_unary_op_node() {
        let node = AstNode::UnaryOp {
            op: UnaryOperator::Neg,
            expr: Box::new(AstNode::Number(10.0)),
        };
        match node {
            AstNode::UnaryOp { op, expr } => {
                assert_eq!(op, UnaryOperator::Neg);
                match *expr {
                    AstNode::Number(val) => assert_eq!(val, 10.0),
                    _ => panic!("Expected Number in expr"),
                }
            }
            _ => panic!("Expected UnaryOp node"),
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_ast_node_serde_roundtrip() {
        let node = AstNode::Statements(vec![
            AstNode::Assignment {
                name: "RSI".to_string(),
                expr: Box::new(AstNode::FunctionCall {
                    name: "RSI".to_string(),
                    args: vec![
                        AstNode::Variable("CLOSE".to_string()),
                        AstNode::Number(14.0),
                    ],
                }),
            },
            AstNode::Output {
                name: "RSI".to_string(),
                expr: Box::new(AstNode::Variable("RSI".to_string())),
                modifier: Some(OutputModifier {
                    line_style: Some(LineStyle { width: 2 }),
                    draw_modifier: Some(DrawModifier::ColorAuto),
                    point_style: None,
                    color: Some(ColorSpec::Rgb(255, 0, 0)),
                }),
            },
        ]);
        let json = serde_json::to_string(&node).expect("serialize AstNode");
        let back: AstNode = serde_json::from_str(&json).expect("deserialize AstNode");
        let json2 = serde_json::to_string(&back).expect("re-serialize AstNode");
        assert_eq!(json, json2, "serde round-trip must be stable");
    }

    fn level_marker(name: &str) -> AstNode {
        AstNode::Output {
            name: name.to_string(),
            expr: Box::new(AstNode::Number(30.0)),
            modifier: Some(OutputModifier {
                line_style: None,
                draw_modifier: Some(DrawModifier::LevelLine),
                point_style: None,
                color: None,
            }),
        }
    }

    #[test]
    fn drawing_directives_do_not_produce_values() {
        let directives = [
            AstNode::DrawText {
                cond: Box::new(AstNode::Number(1.0)),
                price: Box::new(AstNode::Number(1.0)),
                text: "t".to_string(),
                color: None,
            },
            AstNode::DrawIcon {
                cond: Box::new(AstNode::Number(1.0)),
                price: Box::new(AstNode::Number(1.0)),
                icon: Box::new(AstNode::Number(1.0)),
                color: None,
            },
            AstNode::StickLine {
                cond: Box::new(AstNode::Number(1.0)),
                price1: Box::new(AstNode::Number(1.0)),
                price2: Box::new(AstNode::Number(2.0)),
                width: Box::new(AstNode::Number(1.0)),
                empty: false,
                color: None,
            },
            AstNode::DrawGeneric {
                command: "FILL".to_string(),
                args: vec![AstNode::Number(1.0), AstNode::Number(2.0)],
                color: None,
            },
            level_marker("HLINE"),
        ];
        for directive in &directives {
            assert!(
                !directive.produces_value(),
                "a drawing directive or level marker must not be a formula result"
            );
        }
    }

    /// A plain output is still a value; only level markers are excluded.
    #[test]
    fn plain_outputs_still_produce_values() {
        let plain = AstNode::Output {
            name: "RSI".to_string(),
            expr: Box::new(AstNode::Variable("RSI".to_string())),
            modifier: Some(OutputModifier {
                line_style: None,
                draw_modifier: Some(DrawModifier::NoDraw),
                point_style: None,
                color: None,
            }),
        };
        assert!(plain.produces_value());
        assert!(AstNode::Number(1.0).produces_value());
        assert!(AstNode::Variable("CLOSE".to_string()).produces_value());
    }

    /// The reported result is the last statement that carries a value, so a
    /// trailing `hline(30, "Oversold")` cannot become an RSI script's output.
    #[test]
    fn result_statement_skips_trailing_side_effects() {
        let statements = vec![
            AstNode::Assignment {
                name: "RSI".to_string(),
                expr: Box::new(AstNode::FunctionCall {
                    name: "RSI".to_string(),
                    args: vec![
                        AstNode::Variable("CLOSE".to_string()),
                        AstNode::Number(14.0),
                    ],
                }),
            },
            level_marker("HLINE"),
            level_marker("HLINE"),
        ];
        assert_eq!(result_statement_index(&statements), Some(0));

        // No value-producing statement at all: the caller keeps its placeholder.
        let only_markers = vec![level_marker("HLINE")];
        assert_eq!(result_statement_index(&only_markers), None);

        // The ordinary case is unchanged: the last statement wins.
        let plain = vec![AstNode::Number(1.0), AstNode::Number(2.0)];
        assert_eq!(result_statement_index(&plain), Some(1));
    }
}
