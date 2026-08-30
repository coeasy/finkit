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
}
