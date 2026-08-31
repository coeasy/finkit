use crate::formula::ast::AstNode;
use crate::formula::types::FormulaError;
use std::collections::HashMap;

/// 参数定义
#[derive(Debug, Clone)]
pub struct ParamDef {
    pub name: String,
    pub min: f64,
    pub max: f64,
    pub default: f64,
}

/// 参数值
pub type ParamValues = HashMap<String, f64>;

/// 从AST提取参数声明
pub fn parse_params(ast: &AstNode) -> Result<Vec<ParamDef>, FormulaError> {
    let mut params = Vec::new();
    collect_params(ast, &mut params);
    Ok(params)
}

fn collect_params(ast: &AstNode, params: &mut Vec<ParamDef>) {
    match ast {
        AstNode::ParamDecl {
            name,
            min,
            max,
            default,
        } => {
            params.push(ParamDef {
                name: name.clone(),
                min: *min,
                max: *max,
                default: *default,
            });
        }
        AstNode::Statements(stmts) => {
            for stmt in stmts {
                collect_params(stmt, params);
            }
        }
        _ => {}
    }
}

/// 验证参数值是否在合法范围内
pub fn validate_params(params: &[ParamDef], values: &ParamValues) -> Result<(), FormulaError> {
    for param in params {
        if let Some(&value) = values.get(&param.name) {
            if value < param.min || value > param.max {
                return Err(FormulaError::InvalidParameter(format!(
                    "Parameter '{}' value {} is out of range [{}, {}]",
                    param.name, value, param.min, param.max
                )));
            }
        }
    }
    Ok(())
}

/// 获取参数值（如果未提供则使用默认值）
pub fn get_param_value(param: &ParamDef, values: &ParamValues) -> f64 {
    values.get(&param.name).copied().unwrap_or(param.default)
}

/// 将参数应用到AST（替换ParamDecl为实际值，替换Variable引用为Number）
pub fn apply_params(ast: &AstNode, values: &ParamValues) -> AstNode {
    match ast {
        AstNode::ParamDecl {
            name,
            min: _,
            max: _,
            default: _,
        } => {
            let value = values.get(name).copied().unwrap_or(0.0);
            AstNode::Number(value)
        }
        AstNode::Variable(name) => {
            if let Some(&value) = values.get(name) {
                AstNode::Number(value)
            } else {
                AstNode::Variable(name.clone())
            }
        }
        AstNode::Statements(stmts) => {
            let new_stmts: Vec<AstNode> = stmts.iter().map(|s| apply_params(s, values)).collect();
            if new_stmts.len() == 1 {
                new_stmts.into_iter().next().unwrap()
            } else {
                AstNode::Statements(new_stmts)
            }
        }
        AstNode::Assignment { name, expr } => AstNode::Assignment {
            name: name.clone(),
            expr: Box::new(apply_params(expr, values)),
        },
        AstNode::Output {
            name,
            expr,
            modifier,
        } => AstNode::Output {
            name: name.clone(),
            expr: Box::new(apply_params(expr, values)),
            modifier: modifier.clone(),
        },
        AstNode::BinaryOp { op, left, right } => AstNode::BinaryOp {
            op: op.clone(),
            left: Box::new(apply_params(left, values)),
            right: Box::new(apply_params(right, values)),
        },
        AstNode::UnaryOp { op, expr } => AstNode::UnaryOp {
            op: op.clone(),
            expr: Box::new(apply_params(expr, values)),
        },
        AstNode::FunctionCall { name, args } => {
            let new_args: Vec<AstNode> = args.iter().map(|a| apply_params(a, values)).collect();
            AstNode::FunctionCall {
                name: name.clone(),
                args: new_args,
            }
        }
        AstNode::IndexAccess { array, index } => AstNode::IndexAccess {
            array: Box::new(apply_params(array, values)),
            index: Box::new(apply_params(index, values)),
        },
        AstNode::CompoundAssignment { name, op, expr } => AstNode::CompoundAssignment {
            name: name.clone(),
            op: op.clone(),
            expr: Box::new(apply_params(expr, values)),
        },
        AstNode::Number(_) | AstNode::StringLit(_) => ast.clone(),
        AstNode::DrawText {
            cond,
            price,
            text,
            color,
        } => AstNode::DrawText {
            cond: Box::new(apply_params(cond, values)),
            price: Box::new(apply_params(price, values)),
            text: text.clone(),
            color: color.clone(),
        },
        AstNode::DrawIcon {
            cond,
            price,
            icon,
            color,
        } => AstNode::DrawIcon {
            cond: Box::new(apply_params(cond, values)),
            price: Box::new(apply_params(price, values)),
            icon: Box::new(apply_params(icon, values)),
            color: color.clone(),
        },
        AstNode::StickLine {
            cond,
            price1,
            price2,
            width,
            empty,
            color,
        } => AstNode::StickLine {
            cond: Box::new(apply_params(cond, values)),
            price1: Box::new(apply_params(price1, values)),
            price2: Box::new(apply_params(price2, values)),
            width: Box::new(apply_params(width, values)),
            empty: *empty,
            color: color.clone(),
        },
        AstNode::DrawGeneric {
            command,
            args,
            color,
        } => AstNode::DrawGeneric {
            command: command.clone(),
            args: args.iter().map(|a| apply_params(a, values)).collect(),
            color: color.clone(),
        },
        AstNode::IfThenElse {
            cond,
            then_branch,
            else_branch,
        } => AstNode::IfThenElse {
            cond: Box::new(apply_params(cond, values)),
            then_branch: Box::new(apply_params(then_branch, values)),
            else_branch: Box::new(apply_params(else_branch, values)),
        },
        AstNode::ForLoop {
            var,
            start,
            end,
            body,
        } => {
            let new_body: Vec<AstNode> = body.iter().map(|s| apply_params(s, values)).collect();
            AstNode::ForLoop {
                var: var.clone(),
                start: Box::new(apply_params(start, values)),
                end: Box::new(apply_params(end, values)),
                body: new_body,
            }
        }
        AstNode::WhileLoop { cond, body } => {
            let new_body: Vec<AstNode> = body.iter().map(|s| apply_params(s, values)).collect();
            AstNode::WhileLoop {
                cond: Box::new(apply_params(cond, values)),
                body: new_body,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_params_single() {
        let ast = AstNode::ParamDecl {
            name: "N".to_string(),
            min: 1.0,
            max: 100.0,
            default: 20.0,
        };
        let params = parse_params(&ast).unwrap();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "N");
        assert_eq!(params[0].min, 1.0);
        assert_eq!(params[0].max, 100.0);
        assert_eq!(params[0].default, 20.0);
    }

    #[test]
    fn test_parse_params_multiple() {
        let ast = AstNode::Statements(vec![
            AstNode::ParamDecl {
                name: "N".to_string(),
                min: 1.0,
                max: 100.0,
                default: 20.0,
            },
            AstNode::ParamDecl {
                name: "M".to_string(),
                min: 1.0,
                max: 50.0,
                default: 10.0,
            },
            AstNode::Variable("C".to_string()),
        ]);
        let params = parse_params(&ast).unwrap();
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "N");
        assert_eq!(params[1].name, "M");
    }

    #[test]
    fn test_validate_params_valid() {
        let params = vec![ParamDef {
            name: "N".to_string(),
            min: 1.0,
            max: 100.0,
            default: 20.0,
        }];
        let mut values = ParamValues::new();
        values.insert("N".to_string(), 50.0);
        assert!(validate_params(&params, &values).is_ok());
    }

    #[test]
    fn test_validate_params_out_of_range() {
        let params = vec![ParamDef {
            name: "N".to_string(),
            min: 1.0,
            max: 100.0,
            default: 20.0,
        }];
        let mut values = ParamValues::new();
        values.insert("N".to_string(), 150.0);
        assert!(validate_params(&params, &values).is_err());
    }

    #[test]
    fn test_validate_params_missing_value_uses_default() {
        let params = vec![ParamDef {
            name: "N".to_string(),
            min: 1.0,
            max: 100.0,
            default: 20.0,
        }];
        let values = ParamValues::new();
        assert!(validate_params(&params, &values).is_ok());
    }

    #[test]
    fn test_get_param_value_provided() {
        let param = ParamDef {
            name: "N".to_string(),
            min: 1.0,
            max: 100.0,
            default: 20.0,
        };
        let mut values = ParamValues::new();
        values.insert("N".to_string(), 50.0);
        assert_eq!(get_param_value(&param, &values), 50.0);
    }

    #[test]
    fn test_get_param_value_default() {
        let param = ParamDef {
            name: "N".to_string(),
            min: 1.0,
            max: 100.0,
            default: 20.0,
        };
        let values = ParamValues::new();
        assert_eq!(get_param_value(&param, &values), 20.0);
    }

    #[test]
    fn test_apply_params_to_variable() {
        let ast = AstNode::Variable("N".to_string());
        let mut values = ParamValues::new();
        values.insert("N".to_string(), 30.0);
        let result = apply_params(&ast, &values);
        match result {
            AstNode::Number(val) => assert_eq!(val, 30.0),
            _ => panic!("Expected Number"),
        }
    }

    #[test]
    fn test_apply_params_to_param_decl() {
        let ast = AstNode::ParamDecl {
            name: "N".to_string(),
            min: 1.0,
            max: 100.0,
            default: 20.0,
        };
        let mut values = ParamValues::new();
        values.insert("N".to_string(), 30.0);
        let result = apply_params(&ast, &values);
        match result {
            AstNode::Number(val) => assert_eq!(val, 30.0),
            _ => panic!("Expected Number"),
        }
    }

    #[test]
    fn test_apply_params_to_binary_op() {
        let ast = AstNode::BinaryOp {
            op: crate::formula::ast::BinaryOperator::Add,
            left: Box::new(AstNode::Variable("N".to_string())),
            right: Box::new(AstNode::Number(10.0)),
        };
        let mut values = ParamValues::new();
        values.insert("N".to_string(), 30.0);
        let result = apply_params(&ast, &values);
        match result {
            AstNode::BinaryOp { left, right, .. } => {
                match *left {
                    AstNode::Number(val) => assert_eq!(val, 30.0),
                    _ => panic!("Expected Number on left"),
                }
                match *right {
                    AstNode::Number(val) => assert_eq!(val, 10.0),
                    _ => panic!("Expected Number on right"),
                }
            }
            _ => panic!("Expected BinaryOp"),
        }
    }

    #[test]
    fn test_apply_params_to_function_call() {
        let ast = AstNode::FunctionCall {
            name: "MA".to_string(),
            args: vec![
                AstNode::Variable("CLOSE".to_string()),
                AstNode::Variable("N".to_string()),
            ],
        };
        let mut values = ParamValues::new();
        values.insert("N".to_string(), 20.0);
        let result = apply_params(&ast, &values);
        match result {
            AstNode::FunctionCall { args, .. } => match &args[1] {
                AstNode::Number(val) => assert_eq!(*val, 20.0),
                _ => panic!("Expected Number for N"),
            },
            _ => panic!("Expected FunctionCall"),
        }
    }

    #[test]
    fn test_apply_params_preserves_non_param_variables() {
        let ast = AstNode::Variable("CLOSE".to_string());
        let mut values = ParamValues::new();
        values.insert("N".to_string(), 20.0);
        let result = apply_params(&ast, &values);
        match result {
            AstNode::Variable(name) => assert_eq!(name, "CLOSE"),
            _ => panic!("Expected Variable to be preserved"),
        }
    }

    #[test]
    fn test_apply_params_to_statements() {
        let ast = AstNode::Statements(vec![
            AstNode::ParamDecl {
                name: "N".to_string(),
                min: 1.0,
                max: 100.0,
                default: 20.0,
            },
            AstNode::Assignment {
                name: "RESULT".to_string(),
                expr: Box::new(AstNode::FunctionCall {
                    name: "MA".to_string(),
                    args: vec![
                        AstNode::Variable("CLOSE".to_string()),
                        AstNode::Variable("N".to_string()),
                    ],
                }),
            },
        ]);
        let mut values = ParamValues::new();
        values.insert("N".to_string(), 10.0);
        let result = apply_params(&ast, &values);

        match result {
            AstNode::Statements(stmts) => match &stmts[0] {
                AstNode::Number(val) => assert_eq!(*val, 10.0),
                _ => panic!("Expected Number for replaced ParamDecl"),
            },
            _ => panic!("Expected Statements"),
        }
    }
}
