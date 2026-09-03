//! Maps Pine Script AST nodes to AlphaTA formula AST (`AstNode`).

use crate::formula::ast::{AstNode, BinaryOperator, UnaryOperator};
use crate::formula::pine::builtin_table::PineBuiltinTable;
use crate::formula::pine::parser::{FunctionBody, PineAst, PineAstNode, PineBinaryOp, PineUnaryOp};

/// Error during Pine → AlphaTA AST mapping.
#[derive(Debug, Clone)]
pub struct PineMapperError {
    pub message: String,
}

impl std::fmt::Display for PineMapperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pine mapper error: {}", self.message)
    }
}

impl std::error::Error for PineMapperError {}

/// Map a complete Pine AST to AlphaTA `AstNode::Statements`.
pub fn map_pine_to_alphata(pine: &PineAst) -> Result<AstNode, PineMapperError> {
    let table = PineBuiltinTable::new();
    let mapper = PineAstMapper::new(&table);
    let stmts = mapper.map_items(&pine.items)?;
    if stmts.len() == 1 {
        Ok(stmts[0].clone())
    } else {
        Ok(AstNode::Statements(stmts))
    }
}

struct PineAstMapper<'a> {
    table: &'a PineBuiltinTable,
}

impl<'a> PineAstMapper<'a> {
    fn new(table: &'a PineBuiltinTable) -> Self {
        Self { table }
    }

    fn map_items(&self, items: &[PineAstNode]) -> Result<Vec<AstNode>, PineMapperError> {
        let mut out = Vec::new();
        for item in items {
            match item {
                PineAstNode::VersionAnnotation(_) => {}
                PineAstNode::IndicatorDecl { title, .. } | PineAstNode::StudyDecl { title, .. } => {
                    out.push(AstNode::StringLit(format!("INDICATOR:{}", title)));
                }
                other => out.push(self.map_node(other)?),
            }
        }
        Ok(out)
    }

    fn map_node(&self, node: &PineAstNode) -> Result<AstNode, PineMapperError> {
        match node {
            PineAstNode::VersionAnnotation(_) => Ok(AstNode::Number(5.0)),
            PineAstNode::IndicatorDecl { .. } | PineAstNode::StudyDecl { .. } => {
                Ok(AstNode::Number(0.0))
            }
            PineAstNode::VarDecl { name, init, .. } => Ok(AstNode::Assignment {
                name: map_builtin_var(name),
                expr: Box::new(self.map_node(init)?),
            }),
            PineAstNode::InputDecl { default, .. } => self.map_node(default),
            PineAstNode::FunctionDecl {
                name,
                params: _,
                body,
            } => {
                let body_nodes = match body {
                    FunctionBody::Expr(expr) => vec![self.map_node(expr)?],
                    FunctionBody::Block(stmts) => self.map_items(stmts)?,
                };
                Ok(AstNode::FunctionCall {
                    name: format!("FN_{}", name.to_uppercase()),
                    args: body_nodes,
                })
            }
            PineAstNode::Assignment {
                name,
                is_reassign: _,
                expr,
            } => Ok(AstNode::Assignment {
                name: map_builtin_var(name),
                expr: Box::new(self.map_node(expr)?),
            }),
            PineAstNode::TupleAssign { names, expr } => self.map_tuple_assign(names, expr),
            PineAstNode::IfStmt {
                cond,
                then_body,
                else_body,
            } => {
                let then_branch = self.map_block_as_expr(then_body)?;
                let else_branch = if let Some(eb) = else_body {
                    self.map_block_as_expr(eb)?
                } else {
                    AstNode::Number(0.0)
                };
                Ok(AstNode::IfThenElse {
                    cond: Box::new(self.map_node(cond)?),
                    then_branch: Box::new(then_branch),
                    else_branch: Box::new(else_branch),
                })
            }
            PineAstNode::ForStmt {
                var,
                start,
                end,
                body,
                ..
            } => Ok(AstNode::ForLoop {
                var: var.clone(),
                start: Box::new(self.map_node(start)?),
                end: Box::new(self.map_node(end)?),
                body: self.map_items(body)?,
            }),
            PineAstNode::WhileStmt { cond, body } => Ok(AstNode::WhileLoop {
                cond: Box::new(self.map_node(cond)?),
                body: self.map_items(body)?,
            }),
            PineAstNode::PlotCall { value, .. } => Ok(AstNode::Output {
                name: "PLOT".to_string(),
                expr: Box::new(self.map_node(value)?),
                modifier: None,
            }),
            PineAstNode::HlineCall { price, .. } => Ok(AstNode::Output {
                name: "HLINE".to_string(),
                expr: Box::new(self.map_node(price)?),
                modifier: None,
            }),
            PineAstNode::FillCall { plot1, plot2, .. } => Ok(AstNode::DrawGeneric {
                command: "FILL".to_string(),
                args: vec![self.map_node(plot1)?, self.map_node(plot2)?],
                color: None,
            }),
            PineAstNode::Expr(expr) => self.map_node(expr),
            PineAstNode::Number(n) => Ok(AstNode::Number(*n)),
            PineAstNode::StringLit(s) => Ok(AstNode::StringLit(s.clone())),
            PineAstNode::NaLiteral => Ok(AstNode::Number(f64::NAN)),
            PineAstNode::Identifier(id) => {
                // Pine color constants used as values (e.g. inside a ternary that feeds a
                // plot `color=` argument). Surface them as numeric constants so evaluation
                // does not need a separate color type.
                if id.starts_with("color.") {
                    return Ok(AstNode::Number(pine_color_const(id)));
                }
                // `ta.obv` used without call parentheses (Pine treats it as OBV of close & volume).
                if id == "ta.obv" {
                    return Ok(AstNode::FunctionCall {
                        name: "OBV".to_string(),
                        args: vec![
                            AstNode::Variable("CLOSE".to_string()),
                            AstNode::Variable("VOL".to_string()),
                        ],
                    });
                }
                Ok(AstNode::Variable(map_builtin_var(id)))
            }
            PineAstNode::BinaryOp { op, left, right } => Ok(AstNode::BinaryOp {
                op: map_binary_op(*op),
                left: Box::new(self.map_node(left)?),
                right: Box::new(self.map_node(right)?),
            }),
            PineAstNode::UnaryOp { op, expr } => Ok(AstNode::UnaryOp {
                op: map_unary_op(*op),
                expr: Box::new(self.map_node(expr)?),
            }),
            PineAstNode::Ternary {
                cond,
                then_expr,
                else_expr,
            } => Ok(AstNode::IfThenElse {
                cond: Box::new(self.map_node(cond)?),
                then_branch: Box::new(self.map_node(then_expr)?),
                else_branch: Box::new(self.map_node(else_expr)?),
            }),
            PineAstNode::FunctionCall {
                namespace,
                name,
                args,
            } => self.map_function_call(namespace.as_deref(), name, args),
            PineAstNode::IndexAccess { array, index } => Ok(AstNode::IndexAccess {
                array: Box::new(self.map_node(array)?),
                index: Box::new(self.map_node(index)?),
            }),
            PineAstNode::BarstateAccess { field } => {
                Ok(AstNode::Variable(format!("barstate_{}", field)))
            }
        }
    }

    fn map_block_as_expr(&self, body: &[PineAstNode]) -> Result<AstNode, PineMapperError> {
        let stmts = self.map_items(body)?;
        if stmts.len() == 1 {
            Ok(stmts[0].clone())
        } else {
            Ok(AstNode::Statements(stmts))
        }
    }

    /// Map a Pine tuple-destructure assignment, e.g.
    /// `[macdLine, signalLine, histLine] = ta.macd(src, f, s, g)`.
    ///
    /// Each multi-return Pine function is expanded into one AlphaTA assignment
    /// per target, using the *correct* AlphaTA function and argument shape
    /// (Pine hides HLC/period semantics that AlphaTA exposes explicitly). The
    /// generic builtin-table `return_names` are not used here because several
    /// AlphaTA per-output functions have signatures that differ from a simple
    /// positional pass-through.
    fn map_tuple_assign(
        &self,
        names: &[String],
        expr: &PineAstNode,
    ) -> Result<AstNode, PineMapperError> {
        if names.is_empty() {
            return Err(PineMapperError {
                message: "tuple assignment requires at least one target".to_string(),
            });
        }

        if let PineAstNode::FunctionCall {
            namespace, name, ..
        } = expr
        {
            let expected = match (namespace.as_deref(), name.as_str()) {
                (Some("ta"), "macd" | "bb" | "dmi") => Some(3usize),
                (Some("ta"), "supertrend" | "aroon") => Some(2usize),
                _ => None,
            };
            if let Some(expected) = expected {
                if names.len() != expected {
                    return Err(PineMapperError {
                        message: format!(
                            "ta.{name} returns {expected} values but tuple has {} targets",
                            names.len()
                        ),
                    });
                }
            }
        }

        if let PineAstNode::FunctionCall {
            namespace,
            name,
            args,
        } = expr
        {
            let mapped_args: Vec<AstNode> = args
                .iter()
                .map(|(_, a)| self.map_node(a))
                .collect::<Result<_, _>>()?;

            let hi = AstNode::Variable("HIGH".to_string());
            let lo = AstNode::Variable("LOW".to_string());
            let cl = AstNode::Variable("CLOSE".to_string());

            let expanded: Option<Vec<AstNode>> = match (namespace.as_deref(), name.as_str()) {
                (Some("ta"), "macd") if mapped_args.len() >= 4 => {
                    // ta.macd(src, fast, slow, signal) -> [line(DIF), signal(DEA), hist]
                    let src = mapped_args[0].clone();
                    let f = mapped_args[1].clone();
                    let s = mapped_args[2].clone();
                    let g = mapped_args[3].clone();
                    let line = AstNode::FunctionCall {
                        name: "MACD".to_string(),
                        args: vec![src.clone(), f.clone(), s.clone(), g.clone()],
                    };
                    let signal = AstNode::FunctionCall {
                        name: "DEA".to_string(),
                        args: vec![src, f, s, g],
                    };
                    let hist = AstNode::BinaryOp {
                        op: BinaryOperator::Sub,
                        left: Box::new(line.clone()),
                        right: Box::new(signal.clone()),
                    };
                    Some(vec![
                        assignment(&names[0], line),
                        assignment(&names[1], signal),
                        assignment(&names[2], hist),
                    ])
                }
                (Some("ta"), "bb") if mapped_args.len() >= 2 => {
                    // ta.bb(src, length, mult) -> [middle, upper, lower]
                    Some(vec![
                        assignment(
                            &names[0],
                            AstNode::FunctionCall {
                                name: "BOLLMID".to_string(),
                                args: mapped_args.clone(),
                            },
                        ),
                        assignment(
                            &names[1],
                            AstNode::FunctionCall {
                                name: "BOLLUP".to_string(),
                                args: mapped_args.clone(),
                            },
                        ),
                        assignment(
                            &names[2],
                            AstNode::FunctionCall {
                                name: "BOLLDN".to_string(),
                                args: mapped_args.clone(),
                            },
                        ),
                    ])
                }
                (Some("ta"), "dmi") if mapped_args.len() >= 2 => {
                    // ta.dmi(diLength, adxSmoothing) -> [+DI, -DI, ADX] (HLC implicit)
                    let l1 = mapped_args[0].clone();
                    let l2 = mapped_args[1].clone();
                    Some(vec![
                        assignment(
                            &names[0],
                            AstNode::FunctionCall {
                                name: "PLUS_DI".to_string(),
                                args: vec![cl.clone(), l1.clone()],
                            },
                        ),
                        assignment(
                            &names[1],
                            AstNode::FunctionCall {
                                name: "MINUS_DI".to_string(),
                                args: vec![cl.clone(), l1.clone()],
                            },
                        ),
                        assignment(
                            &names[2],
                            AstNode::FunctionCall {
                                name: "ADX".to_string(),
                                args: vec![hi, lo, cl, l1, l2],
                            },
                        ),
                    ])
                }
                (Some("ta"), "supertrend") if mapped_args.len() >= 2 => {
                    // ta.supertrend(factor, atrPeriod) -> [supertrend, direction]
                    let factor = mapped_args[0].clone();
                    let atr_period = mapped_args[1].clone();
                    let st = AstNode::FunctionCall {
                        name: "SUPERTREND".to_string(),
                        args: vec![hi, lo, cl.clone(), atr_period, factor],
                    };
                    let dir = AstNode::FunctionCall {
                        name: "IF".to_string(),
                        args: vec![
                            AstNode::BinaryOp {
                                op: BinaryOperator::Gte,
                                left: Box::new(cl),
                                right: Box::new(st.clone()),
                            },
                            AstNode::Number(-1.0),
                            AstNode::Number(1.0),
                        ],
                    };
                    Some(vec![assignment(&names[0], st), assignment(&names[1], dir)])
                }
                (Some("ta"), "aroon") if !mapped_args.is_empty() => {
                    // ta.aroon(length) -> [aroonUp, aroonDown]
                    let length = mapped_args[0].clone();
                    Some(vec![
                        assignment(
                            &names[0],
                            AstNode::FunctionCall {
                                name: "AROON_UP".to_string(),
                                args: vec![hi, length.clone()],
                            },
                        ),
                        assignment(
                            &names[1],
                            AstNode::FunctionCall {
                                name: "AROON_DN".to_string(),
                                args: vec![lo, length],
                            },
                        ),
                    ])
                }
                _ => None,
            };

            if let Some(assignments) = expanded {
                return Ok(AstNode::Statements(assignments));
            }
        }

        // Fallback: bind the whole RHS to the first target name.
        let mapped = self.map_node(expr)?;
        Ok(assignment(&names[0], mapped))
    }

    fn map_function_call(
        &self,
        namespace: Option<&str>,
        name: &str,
        args: &[(Option<String>, PineAstNode)],
    ) -> Result<AstNode, PineMapperError> {
        // `request.security(sym, tf, expr)` — single-timeframe passthrough.
        // Multi-timeframe data is not available, so we evaluate the requested
        // expression on the current series (the third argument).
        if namespace == Some("request") && name == "security" {
            if let Some((_, expr_arg)) = args.get(2) {
                return self.map_node(expr_arg);
            }
            return Ok(AstNode::Variable("CLOSE".to_string()));
        }

        // `color.new(base, alpha)` — returns the base color; alpha is ignored in
        // the numeric color scheme.
        if namespace == Some("color") && name == "new" {
            if let Some((_, base)) = args.first() {
                return self.map_node(base);
            }
            return Ok(AstNode::Number(1.0));
        }

        // Special Pine na helpers
        if namespace.is_none() {
            match name {
                "nz" => {
                    let mapped_args: Vec<AstNode> = args
                        .iter()
                        .map(|(_, a)| self.map_node(a))
                        .collect::<Result<_, _>>()?;
                    if let Some(value) = mapped_args.first() {
                        let replacement =
                            mapped_args.get(1).cloned().unwrap_or(AstNode::Number(0.0));
                        return Ok(AstNode::FunctionCall {
                            name: "IF".to_string(),
                            args: vec![
                                AstNode::FunctionCall {
                                    name: "ISNA".to_string(),
                                    args: vec![value.clone()],
                                },
                                replacement,
                                value.clone(),
                            ],
                        });
                    }
                }
                "na" => {
                    if let Some((_, value)) = args.first() {
                        return Ok(AstNode::FunctionCall {
                            name: "ISNA".to_string(),
                            args: vec![self.map_node(value)?],
                        });
                    }
                    return Err(PineMapperError {
                        message: "na(x) requires one argument; bare na is parsed as NaLiteral"
                            .to_string(),
                    });
                }
                "fixnan" => {
                    let mapped_args: Vec<AstNode> = args
                        .iter()
                        .map(|(_, a)| self.map_node(a))
                        .collect::<Result<_, _>>()?;
                    return Ok(AstNode::FunctionCall {
                        name: "FIXNAN".to_string(),
                        args: mapped_args,
                    });
                }
                _ => {}
            }
        }

        // `input(...)` / `input.int(...)` → its default value expression.
        // Pine `input` is only meaningful at definition sites; when it appears as
        // an expression (e.g. `len = input(14)`) we collapse it to the default.
        let is_input = name == "input" || namespace.as_deref() == Some("input");
        if is_input {
            if let Some((_, first)) = args.first() {
                return self.map_node(first);
            }
            return Ok(AstNode::Number(f64::NAN));
        }

        let mapped_args: Vec<AstNode> = args
            .iter()
            .map(|(_, a)| self.map_node(a))
            .collect::<Result<_, _>>()?;

        // Pine `ta.*` argument normalization. Pine exposes compact calls
        // (e.g. `ta.atr(length)`); AlphaTA functions expect OHLCV-expanded args.
        if namespace == Some("ta") {
            match name {
                "atr" | "natr" => {
                    let n = mapped_args.get(0).cloned().unwrap_or(AstNode::Number(14.0));
                    return Ok(AstNode::FunctionCall {
                        name: name.to_uppercase(),
                        args: vec![v("HIGH"), v("LOW"), v("CLOSE"), n],
                    });
                }
                "cci" => {
                    let source = mapped_args.get(0).cloned().unwrap_or(v("CLOSE"));
                    let n = mapped_args.get(1).cloned().unwrap_or(AstNode::Number(20.0));
                    return Ok(AstNode::FunctionCall {
                        name: "CCI".to_string(),
                        args: vec![source, n],
                    });
                }
                "wpr" | "williamspercentr" => {
                    let n = mapped_args.get(0).cloned().unwrap_or(AstNode::Number(14.0));
                    return Ok(AstNode::FunctionCall {
                        name: "WILLR".to_string(),
                        args: vec![v("HIGH"), v("LOW"), v("CLOSE"), n],
                    });
                }
                "vwap" => {
                    let source = mapped_args.get(0).cloned().unwrap_or(v("CLOSE"));
                    return Ok(AstNode::FunctionCall {
                        name: "VWAP".to_string(),
                        args: vec![source, v("VOL")],
                    });
                }
                "obv" => {
                    return Ok(AstNode::FunctionCall {
                        name: "OBV".to_string(),
                        args: vec![v("CLOSE"), v("VOL")],
                    });
                }
                "sar" => {
                    let start = mapped_args.get(0).cloned().unwrap_or(AstNode::Number(0.02));
                    let increment = mapped_args.get(1).cloned().unwrap_or(AstNode::Number(0.02));
                    let maximum = mapped_args.get(2).cloned().unwrap_or(AstNode::Number(0.2));
                    return Ok(AstNode::FunctionCall {
                        name: "SAR".to_string(),
                        args: vec![v("HIGH"), v("LOW"), start, increment, maximum],
                    });
                }
                "stoch" => {
                    let source = mapped_args.get(0).cloned().unwrap_or(v("CLOSE"));
                    let high = mapped_args.get(1).cloned().unwrap_or(v("HIGH"));
                    let low = mapped_args.get(2).cloned().unwrap_or(v("LOW"));
                    let n = mapped_args.get(3).cloned().unwrap_or(AstNode::Number(14.0));
                    return Ok(AstNode::FunctionCall {
                        // Pine ta.stoch is the unsmoothed stochastic value.  STOCHF
                        // with fast-D period 1 preserves that contract; generic STOCH
                        // keeps its terminal slow-K defaults independently.
                        name: "STOCHF".to_string(),
                        args: vec![high, low, source, n, AstNode::Number(1.0)],
                    });
                }
                "change" => {
                    let source = mapped_args.get(0).cloned().unwrap_or(v("CLOSE"));
                    let n = mapped_args.get(1).cloned().unwrap_or(AstNode::Number(1.0));
                    return Ok(AstNode::FunctionCall {
                        name: "MOM".to_string(),
                        args: vec![source, n],
                    });
                }
                "sma" => {
                    let source = mapped_args.get(0).cloned().unwrap_or(v("CLOSE"));
                    let n = mapped_args.get(1).cloned().unwrap_or(AstNode::Number(1.0));
                    return Ok(AstNode::FunctionCall {
                        name: "MA".to_string(),
                        args: vec![source, n],
                    });
                }
                "vwma" => {
                    let source = mapped_args.get(0).cloned().unwrap_or(v("CLOSE"));
                    let n = mapped_args.get(1).cloned().unwrap_or(AstNode::Number(20.0));
                    return Ok(AstNode::FunctionCall {
                        name: "VWMA".to_string(),
                        args: vec![source, v("VOL"), n],
                    });
                }
                _ => {}
            }
        }

        if let Some(mapping) = self.table.resolve(namespace, name) {
            Ok(AstNode::FunctionCall {
                name: mapping.alpha_ta_name.clone(),
                args: mapped_args,
            })
        } else {
            let full_name = match namespace {
                Some(ns) => format!("{}_{}", ns.to_uppercase(), name.to_uppercase()),
                None => name.to_uppercase(),
            };
            Ok(AstNode::FunctionCall {
                name: full_name,
                args: mapped_args,
            })
        }
    }
}

/// Build an `Assignment` node, normalizing the target name the same way
/// identifier references are normalized (see `map_builtin_var`) so later
/// lookups resolve.
fn assignment(name: &str, expr: AstNode) -> AstNode {
    AstNode::Assignment {
        name: map_builtin_var(name),
        expr: Box::new(expr),
    }
}

fn map_builtin_var(id: &str) -> String {
    match id {
        "open" => "OPEN".to_string(),
        "high" => "HIGH".to_string(),
        "low" => "LOW".to_string(),
        "close" => "CLOSE".to_string(),
        "volume" => "VOL".to_string(),
        "time" => "DATE".to_string(),
        "hl2" => "HL2".to_string(),
        "hlc3" => "HLC3".to_string(),
        "ohlc4" => "OHLC4".to_string(),
        other => other.to_uppercase(),
    }
}

/// Shorthand for a variable reference node.
fn v(name: &str) -> AstNode {
    AstNode::Variable(name.to_string())
}

/// Map a Pine `color.*` constant to a numeric value. Colors are not first-class
/// in the evaluation engine; when they appear as values (e.g. in a ternary
/// feeding a plot `color=` argument) we surface a stable numeric stand-in.
fn pine_color_const(id: &str) -> f64 {
    match id {
        "color.green" => 1.0,
        "color.red" => 0.0,
        "color.blue" => 2.0,
        "color.yellow" => 3.0,
        "color.orange" => 4.0,
        "color.purple" => 5.0,
        "color.white" => 6.0,
        "color.black" => 7.0,
        "color.gray" => 8.0,
        "color.teal" => 9.0,
        "color.lime" => 10.0,
        "color.aqua" => 11.0,
        "color.fuchsia" => 12.0,
        "color.silver" => 13.0,
        "color.navy" => 14.0,
        "color.maroon" => 15.0,
        "color.olive" => 16.0,
        _ => 1.0,
    }
}

fn map_binary_op(op: PineBinaryOp) -> BinaryOperator {
    match op {
        PineBinaryOp::Add => BinaryOperator::Add,
        PineBinaryOp::Sub => BinaryOperator::Sub,
        PineBinaryOp::Mul => BinaryOperator::Mul,
        PineBinaryOp::Div => BinaryOperator::Div,
        PineBinaryOp::Mod => BinaryOperator::Mod,
        PineBinaryOp::Eq => BinaryOperator::Eq,
        PineBinaryOp::Ne => BinaryOperator::Neq,
        PineBinaryOp::Gt => BinaryOperator::Gt,
        PineBinaryOp::Lt => BinaryOperator::Lt,
        PineBinaryOp::Gte => BinaryOperator::Gte,
        PineBinaryOp::Lte => BinaryOperator::Lte,
        PineBinaryOp::And => BinaryOperator::And,
        PineBinaryOp::Or => BinaryOperator::Or,
    }
}

fn map_unary_op(op: PineUnaryOp) -> UnaryOperator {
    match op {
        PineUnaryOp::Not => UnaryOperator::Not,
        PineUnaryOp::Neg => UnaryOperator::Neg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::pine::parser::parse_pine;

    #[test]
    fn test_map_sma_call() {
        let src = "//@version=5\nindicator(\"MA\")\nma = ta.sma(close, 20)\n";
        let pine = parse_pine(src).unwrap();
        let ast = map_pine_to_alphata(&pine).unwrap();
        let json = format!("{:?}", ast);
        assert!(json.contains("FunctionCall { name: \"MA\""));
        assert!(!json.contains("FunctionCall { name: \"SMA\""));
        assert!(json.contains("CLOSE"));
    }

    #[test]
    fn test_map_plot() {
        let src = "//@version=5\nindicator(\"P\")\nplot(close)\n";
        let pine = parse_pine(src).unwrap();
        let ast = map_pine_to_alphata(&pine).unwrap();
        let json = format!("{:?}", ast);
        assert!(json.contains("PLOT"));
    }
}

#[cfg(test)]
mod pr14_semantic_mapper_v3_tests {
    use super::*;
    use crate::formula::pine::parser::parse_pine;

    fn mapped(source: &str) -> String {
        let pine = parse_pine(source).unwrap();
        format!("{:?}", map_pine_to_alphata(&pine).unwrap())
    }

    #[test]
    fn pine_change_defaults_to_one_bar_momentum() {
        let debug = mapped("//@version=5\nindicator(\"C\")\nc = ta.change(close)\n");
        assert!(debug.contains("FunctionCall { name: \"MOM\""));
        assert!(debug.contains("Number(1.0)"));
    }

    #[test]
    fn pine_stoch_uses_unsmoothed_fast_k_contract() {
        let debug = mapped("//@version=5\nindicator(\"S\")\ns = ta.stoch(close, high, low, 3)\n");
        assert!(debug.contains("FunctionCall { name: \"STOCHF\""));
        assert!(debug.contains("Number(1.0)"));
    }

    #[test]
    fn pine_aroon_preserves_high_low_sources() {
        let debug = mapped("//@version=5\nindicator(\"A\")\n[u, d] = ta.aroon(3)\n");
        assert!(debug.contains("AROON_UP"));
        assert!(debug.contains("AROON_DN"));
        assert!(debug.contains("Variable(\"HIGH\")"));
        assert!(debug.contains("Variable(\"LOW\")"));
    }
}

#[cfg(test)]
mod pr14_tuple_arity_tests {
    use super::*;
    use crate::formula::pine::parser::parse_pine;

    #[test]
    fn malformed_multi_return_tuple_is_an_error_not_a_panic() {
        let pine =
            parse_pine("//@version=5\nindicator(\"M\")\n[a, b] = ta.macd(close, 12, 26, 9)\n")
                .unwrap();
        let err = map_pine_to_alphata(&pine).unwrap_err();
        assert!(err.message.contains("ta.macd returns 3 values"));
        assert!(err.message.contains("tuple has 2 targets"));
    }
}
