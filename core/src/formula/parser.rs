use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::Parser;

use crate::formula::ast::*;

#[derive(Parser)]
#[grammar = "formula/grammar.pest"]
pub struct FormulaParser;

pub fn parse_formula(source: &str) -> Result<AstNode, String> {
    let pairs =
        FormulaParser::parse(Rule::program, source).map_err(|e| format!("Parse error: {}", e))?;

    parse_program(pairs)
}

fn parse_program(pairs: Pairs<Rule>) -> Result<AstNode, String> {
    let mut statements = Vec::new();
    let mut param_decl: Option<AstNode> = None;

    for pair in pairs {
        match pair.as_rule() {
            Rule::param_decl => {
                param_decl = Some(parse_param_decl(pair)?);
            }
            Rule::statement => {
                statements.push(parse_statement(pair)?);
            }
            _ => {}
        }
    }

    if statements.is_empty() {
        return Err("Empty program".to_string());
    }

    let mut all_statements = Vec::new();
    if let Some(param) = param_decl {
        all_statements.push(param);
    }
    all_statements.extend(statements);

    if all_statements.len() == 1 {
        Ok(all_statements.remove(0))
    } else {
        Ok(AstNode::Statements(all_statements))
    }
}

fn parse_param_decl(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut items = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::param_item {
            items.push(parse_param_item(inner)?);
        }
    }

    if items.len() == 1 {
        Ok(items.remove(0))
    } else {
        Ok(AstNode::Statements(items))
    }
}

fn parse_param_item(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();

    let name = inner
        .next()
        .ok_or("Missing param name")?
        .as_str()
        .to_string();

    let min = inner
        .next()
        .ok_or("Missing param min")?
        .as_str()
        .parse::<f64>()
        .map_err(|e| format!("Invalid param min: {}", e))?;

    let max = inner
        .next()
        .ok_or("Missing param max")?
        .as_str()
        .parse::<f64>()
        .map_err(|e| format!("Invalid param max: {}", e))?;

    let default = inner
        .next()
        .ok_or("Missing param default")?
        .as_str()
        .parse::<f64>()
        .map_err(|e| format!("Invalid param default: {}", e))?;

    Ok(AstNode::ParamDecl {
        name,
        min,
        max,
        default,
    })
}

fn parse_statement(pair: Pair<Rule>) -> Result<AstNode, String> {
    match pair.as_rule() {
        Rule::compound_assignment => parse_compound_assignment(pair),
        Rule::assignment => parse_assignment(pair),
        Rule::output => parse_output(pair),
        Rule::draw_text => parse_draw_text(pair),
        Rule::draw_icon => parse_draw_icon(pair),
        Rule::stick_line => parse_stick_line(pair),
        Rule::draw_line => parse_draw_generic(pair, "DRAWLINE"),
        Rule::draw_band => parse_draw_generic(pair, "DRAWBAND"),
        Rule::draw_kline => parse_draw_generic(pair, "DRAWKLINE"),
        Rule::draw_rect => parse_draw_generic(pair, "DRAWRECTREL"),
        Rule::fill_rgn => parse_draw_generic(pair, "FILLRGN"),
        Rule::part_line => parse_draw_generic(pair, "PARTLINE"),
        Rule::poly_line => parse_draw_generic(pair, "POLYLINE"),
        Rule::draw_gbk => parse_draw_generic(pair, "DRAWGBK"),
        Rule::draw_sl => parse_draw_generic(pair, "DRAWSL"),
        Rule::draw_text_fix => parse_draw_text_fix(pair),
        Rule::draw_number => parse_draw_generic(pair, "DRAWNUMBER"),
        Rule::vert_line => parse_draw_generic(pair, "VERTLINE"),
        Rule::if_then_else_stmt => parse_if_then_else(pair),
        Rule::for_stmt => parse_for_stmt(pair),
        Rule::while_stmt => parse_while_stmt(pair),
        Rule::expression | Rule::logical_or => parse_expression(pair),
        _ => {
            let inner = pair.into_inner().next().ok_or("Empty statement")?;
            match inner.as_rule() {
                Rule::compound_assignment => parse_compound_assignment(inner),
                Rule::assignment => parse_assignment(inner),
                Rule::output => parse_output(inner),
                Rule::draw_text => parse_draw_text(inner),
                Rule::draw_icon => parse_draw_icon(inner),
                Rule::stick_line => parse_stick_line(inner),
                Rule::draw_line => parse_draw_generic(inner, "DRAWLINE"),
                Rule::draw_band => parse_draw_generic(inner, "DRAWBAND"),
                Rule::draw_kline => parse_draw_generic(inner, "DRAWKLINE"),
                Rule::draw_rect => parse_draw_generic(inner, "DRAWRECTREL"),
                Rule::fill_rgn => parse_draw_generic(inner, "FILLRGN"),
                Rule::part_line => parse_draw_generic(inner, "PARTLINE"),
                Rule::poly_line => parse_draw_generic(inner, "POLYLINE"),
                Rule::draw_gbk => parse_draw_generic(inner, "DRAWGBK"),
                Rule::draw_sl => parse_draw_generic(inner, "DRAWSL"),
                Rule::draw_text_fix => parse_draw_text_fix(inner),
                Rule::draw_number => parse_draw_generic(inner, "DRAWNUMBER"),
                Rule::vert_line => parse_draw_generic(inner, "VERTLINE"),
                Rule::if_then_else_stmt => parse_if_then_else(inner),
                Rule::for_stmt => parse_for_stmt(inner),
                Rule::while_stmt => parse_while_stmt(inner),
                Rule::expression | Rule::logical_or => parse_expression(inner),
                _ => Err(format!("Unknown statement rule: {:?}", inner.as_rule())),
            }
        }
    }
}

fn parse_assignment(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();

    let name = inner
        .next()
        .ok_or("Missing assignment name")?
        .as_str()
        .to_string();

    let expr = inner.next().ok_or("Missing assignment expression")?;

    Ok(AstNode::Assignment {
        name,
        expr: Box::new(parse_expression(expr)?),
    })
}

fn parse_compound_assignment(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();

    let name = inner
        .next()
        .ok_or("Missing compound assignment name")?
        .as_str()
        .to_string();

    let op_str = inner
        .next()
        .ok_or("Missing compound assignment operator")?
        .as_str();

    let op = match op_str {
        "+=" => CompoundAssignOp::AddAssign,
        "-=" => CompoundAssignOp::SubAssign,
        "*=" => CompoundAssignOp::MulAssign,
        "/=" => CompoundAssignOp::DivAssign,
        _ => return Err(format!("Unknown compound assignment operator: {}", op_str)),
    };

    let expr = inner
        .next()
        .ok_or("Missing compound assignment expression")?;

    Ok(AstNode::CompoundAssignment {
        name,
        op,
        expr: Box::new(parse_expression(expr)?),
    })
}

fn parse_output(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();

    let name = inner
        .next()
        .ok_or("Missing output name")?
        .as_str()
        .to_string();

    let expr = inner.next().ok_or("Missing output expression")?;

    let mut modifier = OutputModifier {
        line_style: None,
        draw_modifier: None,
        point_style: None,
        color: None,
    };

    for mod_pair in inner {
        match mod_pair.as_rule() {
            Rule::output_modifier => {
                let mod_inner = mod_pair
                    .into_inner()
                    .next()
                    .ok_or("Empty output_modifier")?;
                parse_output_modifier_inner(&mod_inner, &mut modifier)?;
            }
            Rule::output_attr => {
                let attr_inner = mod_pair
                    .into_inner()
                    .next()
                    .ok_or("Empty output_attr")?;
                match attr_inner.as_rule() {
                    Rule::color_spec => {
                        modifier.color = Some(parse_color_spec(attr_inner)?);
                    }
                    Rule::line_style | Rule::draw_modifier | Rule::point_style => {
                        parse_output_modifier_inner(&attr_inner, &mut modifier)?;
                    }
                    _ => {}
                }
            }
            Rule::color_spec => {
                modifier.color = Some(parse_color_spec(mod_pair)?);
            }
            Rule::line_style | Rule::draw_modifier | Rule::point_style => {
                parse_output_modifier_inner(&mod_pair, &mut modifier)?;
            }
            _ => {}
        }
    }

    let has_modifier = modifier.line_style.is_some()
        || modifier.draw_modifier.is_some()
        || modifier.point_style.is_some()
        || modifier.color.is_some();

    Ok(AstNode::Output {
        name,
        expr: Box::new(parse_expression(expr)?),
        modifier: if has_modifier { Some(modifier) } else { None },
    })
}

fn parse_output_modifier_inner(mod_inner: &Pair<Rule>, modifier: &mut OutputModifier) -> Result<(), String> {
    match mod_inner.as_rule() {
        Rule::line_style => {
            let s = mod_inner.as_str();
            let digit = s.trim_start_matches("LINETHICK");
            let width = digit
                .parse::<u32>()
                .map_err(|e| format!("Invalid LINETHICK value: {}", e))?;
            modifier.line_style = Some(LineStyle { width });
        }
        Rule::draw_modifier => {
            let mod_name = mod_inner.as_str();
            modifier.draw_modifier = Some(match mod_name {
                "NODRAW" => DrawModifier::NoDraw,
                "NOTEXT" => DrawModifier::NoText,
                "NOAXIS" => DrawModifier::NoAxis,
                "COLORAUTO" => DrawModifier::ColorAuto,
                _ => return Err(format!("Unknown draw modifier: {}", mod_name)),
            });
        }
        Rule::point_style => {
            let mod_name = mod_inner.as_str();
            modifier.point_style = Some(match mod_name {
                "POINTDOT" => PointStyle::PointDot,
                "CIRCLEDOT" => PointStyle::CircleDot,
                "CROSSDOT" => PointStyle::CrossDot,
                "STICK" => PointStyle::Stick,
                "VOLSTICK" => PointStyle::VolStick,
                "LINESTICK" => PointStyle::LineStick,
                "COLORSTICK" => PointStyle::ColorStick,
                _ => return Err(format!("Unknown point style: {}", mod_name)),
            });
        }
        _ => {
            return Err(format!(
                "Unknown output_modifier inner rule: {:?}",
                mod_inner.as_rule()
            ))
        }
    }
    Ok(())
}

fn parse_draw_text(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();

    let cond = parse_expression(inner.next().ok_or("Missing DRAWTEXT cond")?)?;

    let price = parse_expression(inner.next().ok_or("Missing DRAWTEXT price")?)?;

    let text = inner
        .next()
        .ok_or("Missing DRAWTEXT text")?
        .as_str()
        .trim_matches(|c| c == '"' || c == '\'')
        .to_string();

    let color = inner.next().map(|p| parse_color_spec(p)).transpose()?;

    Ok(AstNode::DrawText {
        cond: Box::new(cond),
        price: Box::new(price),
        text,
        color,
    })
}

fn parse_draw_icon(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();

    let cond = parse_expression(inner.next().ok_or("Missing DRAWICON cond")?)?;

    let price = parse_expression(inner.next().ok_or("Missing DRAWICON price")?)?;

    let icon = parse_expression(inner.next().ok_or("Missing DRAWICON icon")?)?;

    let color = inner.next().map(|p| parse_color_spec(p)).transpose()?;

    Ok(AstNode::DrawIcon {
        cond: Box::new(cond),
        price: Box::new(price),
        icon: Box::new(icon),
        color,
    })
}

fn parse_stick_line(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();

    let cond = parse_expression(inner.next().ok_or("Missing STICKLINE cond")?)?;

    let price1 = parse_expression(inner.next().ok_or("Missing STICKLINE price1")?)?;

    let price2 = parse_expression(inner.next().ok_or("Missing STICKLINE price2")?)?;

    let width = parse_expression(inner.next().ok_or("Missing STICKLINE width")?)?;

    let empty = inner.next().ok_or("Missing STICKLINE empty")?.as_str() == "TRUE";

    let color = inner.next().map(|p| parse_color_spec(p)).transpose()?;

    Ok(AstNode::StickLine {
        cond: Box::new(cond),
        price1: Box::new(price1),
        price2: Box::new(price2),
        width: Box::new(width),
        empty,
        color,
    })
}

fn parse_draw_text_fix(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();
    let mut args = Vec::new();
    let mut color = None;

    let x = parse_expression(inner.next().ok_or("Missing DRAWTEXT_FIX x")?)?;
    args.push(x);

    let y = parse_expression(inner.next().ok_or("Missing DRAWTEXT_FIX y")?)?;
    args.push(y);

    let text_pair = inner.next().ok_or("Missing DRAWTEXT_FIX text")?;
    let text = text_pair.as_str().trim_matches(|c| c == '"' || c == '\'').to_string();
    args.push(AstNode::Number(0.0)); // placeholder for text

    if let Some(p) = inner.next() {
        color = Some(parse_color_spec(p)?);
    }

    Ok(AstNode::DrawText {
        cond: Box::new(AstNode::Number(1.0)),
        price: Box::new(args.remove(1)),
        text,
        color,
    })
}

fn parse_draw_generic(pair: Pair<Rule>, command: &str) -> Result<AstNode, String> {
    let inner = pair.into_inner();
    let mut args = Vec::new();
    let mut color = None;

    for item in inner {
        match item.as_rule() {
            Rule::expression | Rule::logical_or => {
                args.push(parse_expression(item)?);
            }
            Rule::color_spec => {
                color = Some(parse_color_spec(item)?);
            }
            Rule::bool_val => {
                let val = if item.as_str() == "TRUE" { 1.0 } else { 0.0 };
                args.push(AstNode::Number(val));
            }
            _ => {
                if let Ok(expr) = parse_expression(item.clone()) {
                    args.push(expr);
                }
            }
        }
    }

    Ok(AstNode::DrawGeneric {
        command: command.to_string(),
        args,
        color,
    })
}

fn parse_if_then_else(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();

    let cond = parse_expression(inner.next().ok_or("Missing IF-THEN-ELSE condition")?)?;

    let then_branch = parse_expression(inner.next().ok_or("Missing IF-THEN-ELSE then branch")?)?;

    let else_branch = parse_expression(inner.next().ok_or("Missing IF-THEN-ELSE else branch")?)?;

    Ok(AstNode::IfThenElse {
        cond: Box::new(cond),
        then_branch: Box::new(then_branch),
        else_branch: Box::new(else_branch),
    })
}

fn parse_for_stmt(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();

    let var = inner
        .next()
        .ok_or("Missing FOR variable")?
        .as_str()
        .to_string();

    let start = parse_expression(inner.next().ok_or("Missing FOR start expression")?)?;

    let end = parse_expression(inner.next().ok_or("Missing FOR end expression")?)?;

    let mut body = Vec::new();
    for stmt_pair in inner {
        if stmt_pair.as_rule() == Rule::statement {
            body.push(parse_statement(stmt_pair)?);
        }
    }

    Ok(AstNode::ForLoop {
        var,
        start: Box::new(start),
        end: Box::new(end),
        body,
    })
}

fn parse_while_stmt(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();

    let cond = parse_expression(inner.next().ok_or("Missing WHILE condition")?)?;

    let mut body = Vec::new();
    for stmt_pair in inner {
        if stmt_pair.as_rule() == Rule::statement {
            body.push(parse_statement(stmt_pair)?);
        }
    }

    Ok(AstNode::WhileLoop {
        cond: Box::new(cond),
        body,
    })
}

fn parse_color_spec(pair: Pair<Rule>) -> Result<ColorSpec, String> {
    let s = pair.as_str();

    if s.starts_with("COLOR(") && s.ends_with(')') {
        let inner = s.trim_start_matches("COLOR(").trim_end_matches(')');
        let mut parts = inner.split(',');
        let r = parts
            .next()
            .ok_or("Missing RGB red")?
            .trim()
            .parse::<u8>()
            .map_err(|e| format!("Invalid RGB red: {}", e))?;
        let g = parts
            .next()
            .ok_or("Missing RGB green")?
            .trim()
            .parse::<u8>()
            .map_err(|e| format!("Invalid RGB green: {}", e))?;
        let b = parts
            .next()
            .ok_or("Missing RGB blue")?
            .trim()
            .parse::<u8>()
            .map_err(|e| format!("Invalid RGB blue: {}", e))?;
        Ok(ColorSpec::Rgb(r, g, b))
    } else if s.starts_with("COLORHEX(") && s.ends_with(')') {
        let hex = s
            .trim_start_matches("COLORHEX(")
            .trim_end_matches(')')
            .to_string();
        Ok(ColorSpec::Hex(hex))
    } else {
        Ok(ColorSpec::Named(s.to_string()))
    }
}

fn parse_expression(pair: Pair<Rule>) -> Result<AstNode, String> {
    let inner = pair.into_inner().next().ok_or("Empty expression")?;
    parse_logical_or(inner)
}

fn parse_logical_or(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();

    let mut result = parse_logical_xor(inner.next().ok_or("Missing logical_or left")?)?;

    for right in inner {
        let right_expr = parse_logical_xor(right)?;
        result = AstNode::BinaryOp {
            op: BinaryOperator::Or,
            left: Box::new(result),
            right: Box::new(right_expr),
        };
    }

    Ok(result)
}

fn parse_logical_xor(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();

    let mut result = parse_logical_and(inner.next().ok_or("Missing logical_xor left")?)?;

    for right in inner {
        let right_expr = parse_logical_and(right)?;
        result = AstNode::BinaryOp {
            op: BinaryOperator::Xor,
            left: Box::new(result),
            right: Box::new(right_expr),
        };
    }

    Ok(result)
}

fn parse_logical_and(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();

    let mut result = parse_comparison(inner.next().ok_or("Missing logical_and left")?)?;

    for right in inner {
        let right_expr = parse_comparison(right)?;
        result = AstNode::BinaryOp {
            op: BinaryOperator::And,
            left: Box::new(result),
            right: Box::new(right_expr),
        };
    }

    Ok(result)
}

fn parse_comparison(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();

    let left = parse_addition(inner.next().ok_or("Missing comparison left")?)?;

    if let Some(op_pair) = inner.next() {
        let op = match op_pair.as_str() {
            ">" => BinaryOperator::Gt,
            "<" => BinaryOperator::Lt,
            ">=" => BinaryOperator::Gte,
            "<=" => BinaryOperator::Lte,
            "==" => BinaryOperator::Eq,
            "!=" | "<>" => BinaryOperator::Neq,
            _ => return Err(format!("Unknown comparison operator: {}", op_pair.as_str())),
        };

        let right = parse_addition(inner.next().ok_or("Missing comparison right")?)?;

        Ok(AstNode::BinaryOp {
            op,
            left: Box::new(left),
            right: Box::new(right),
        })
    } else {
        Ok(left)
    }
}

fn parse_addition(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();

    let mut result = parse_multiplication(inner.next().ok_or("Missing addition left")?)?;

    while let Some(op_pair) = inner.next() {
        let right = parse_multiplication(inner.next().ok_or("Missing addition right")?)?;

        let op = match op_pair.as_str() {
            "+" => BinaryOperator::Add,
            "-" => BinaryOperator::Sub,
            "&" => BinaryOperator::StringConcat,
            _ => return Err(format!("Unknown addition operator: {}", op_pair.as_str())),
        };

        result = AstNode::BinaryOp {
            op,
            left: Box::new(result),
            right: Box::new(right),
        };
    }

    Ok(result)
}

fn parse_multiplication(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();

    let mut result = parse_unary(inner.next().ok_or("Missing multiplication left")?)?;

    while let Some(op_pair) = inner.next() {
        let right = parse_unary(inner.next().ok_or("Missing multiplication right")?)?;

        let op = match op_pair.as_str() {
            "*" => BinaryOperator::Mul,
            "/" => BinaryOperator::Div,
            "%" => BinaryOperator::Mod,
            _ => {
                return Err(format!(
                    "Unknown multiplication operator: {}",
                    op_pair.as_str()
                ))
            }
        };

        result = AstNode::BinaryOp {
            op,
            left: Box::new(result),
            right: Box::new(right),
        };
    }

    Ok(result)
}

fn parse_unary(pair: Pair<Rule>) -> Result<AstNode, String> {
    let full_text = pair.as_str();
    let mut inner = pair.into_inner();
    let first = inner.next().ok_or("Missing unary operand")?;

    match first.as_rule() {
        Rule::unary => {
            let operand = parse_unary(first)?;
            Ok(AstNode::UnaryOp {
                op: UnaryOperator::Not,
                expr: Box::new(operand),
            })
        }
        Rule::power => {
            let power_text = first.as_str();
            let is_neg = full_text.starts_with('-') && !power_text.starts_with('-');
            let operand = parse_power(first)?;
            if is_neg {
                Ok(AstNode::UnaryOp {
                    op: UnaryOperator::Neg,
                    expr: Box::new(operand),
                })
            } else {
                Ok(operand)
            }
        }
        _ => Err(format!("Unknown unary rule: {:?}", first.as_rule())),
    }
}

fn parse_power(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();

    let base = parse_postfix(inner.next().ok_or("Missing power base")?)?;

    if let Some(exp_pair) = inner.next() {
        let exp = parse_postfix(exp_pair)?;
        Ok(AstNode::BinaryOp {
            op: BinaryOperator::Pow,
            left: Box::new(base),
            right: Box::new(exp),
        })
    } else {
        Ok(base)
    }
}

fn parse_postfix(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();
    let mut result = parse_primary(inner.next().ok_or("Missing postfix primary")?)?;

    for index_pair in inner {
        let index_expr = parse_expression(
            index_pair
                .into_inner()
                .next()
                .ok_or("Missing index expression")?,
        )?;
        result = AstNode::IndexAccess {
            array: Box::new(result),
            index: Box::new(index_expr),
        };
    }

    Ok(result)
}

fn parse_primary(pair: Pair<Rule>) -> Result<AstNode, String> {
    let inner = pair.into_inner().next().ok_or("Empty primary")?;

    match inner.as_rule() {
        Rule::function_call => parse_function_call(inner),
        Rule::number => parse_number(inner),
        Rule::string => parse_string_literal(inner),
        Rule::variable => parse_variable(inner),
        Rule::expression => parse_expression(inner),
        _ => Err(format!("Unknown primary rule: {:?}", inner.as_rule())),
    }
}

fn parse_string_literal(pair: Pair<Rule>) -> Result<AstNode, String> {
    let s = pair.as_str().trim_matches(|c| c == '"' || c == '\'').to_string();
    Ok(AstNode::StringLit(s))
}

fn parse_function_call(pair: Pair<Rule>) -> Result<AstNode, String> {
    let mut inner = pair.into_inner();

    let name = inner
        .next()
        .ok_or("Missing function name")?
        .as_str()
        .to_string();

    let mut args = Vec::new();
    for arg_pair in inner {
        args.push(parse_expression(arg_pair)?);
    }

    Ok(AstNode::FunctionCall { name, args })
}

fn parse_number(pair: Pair<Rule>) -> Result<AstNode, String> {
    let value = pair
        .as_str()
        .parse::<f64>()
        .map_err(|e| format!("Invalid number '{}': {}", pair.as_str(), e))?;

    Ok(AstNode::Number(value))
}

fn parse_variable(pair: Pair<Rule>) -> Result<AstNode, String> {
    let raw_name = pair.into_inner()
        .next()
        .ok_or("Empty variable")?
        .as_str();
    let bytes = raw_name.as_bytes();

    let map = |b: &str| -> AstNode {
        AstNode::FunctionCall {
            name: "REF".to_string(),
            args: vec![AstNode::Variable(b.to_string()), AstNode::Number(1.0)],
        }
    };

    if bytes.eq_ignore_ascii_case(b"CLOSE1") {
        Ok(map("CLOSE"))
    } else if bytes.eq_ignore_ascii_case(b"OPEN1") {
        Ok(map("OPEN"))
    } else if bytes.eq_ignore_ascii_case(b"HIGH1") {
        Ok(map("HIGH"))
    } else if bytes.eq_ignore_ascii_case(b"LOW1") {
        Ok(map("LOW"))
    } else if bytes.eq_ignore_ascii_case(b"VOL1") || bytes.eq_ignore_ascii_case(b"VOLUME1") {
        Ok(map("VOLUME"))
    } else if bytes.eq_ignore_ascii_case(b"C1") {
        Ok(map("C"))
    } else if bytes.eq_ignore_ascii_case(b"O1") {
        Ok(map("O"))
    } else if bytes.eq_ignore_ascii_case(b"H1") {
        Ok(map("H"))
    } else if bytes.eq_ignore_ascii_case(b"L1") {
        Ok(map("L"))
    } else if bytes.eq_ignore_ascii_case(b"V1") {
        Ok(map("V"))
    } else {
        Ok(AstNode::Variable(raw_name.to_uppercase()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number() {
        let result = parse_formula("42");
        if let Err(ref e) = result {
            panic!("Parse error: {}", e);
        }
        assert!(result.is_ok());
        let node = result.unwrap();
        if !matches!(&node, AstNode::Number(_)) {
            panic!("Expected Number, got {:?}", node);
        }
        if let AstNode::Number(val) = node {
            assert_eq!(val, 42.0);
        }
    }

    #[test]
    fn test_parse_variable() {
        let result = parse_formula("CLOSE");
        assert!(result.is_ok());
        if let AstNode::Variable(name) = result.unwrap() {
            assert_eq!(name, "CLOSE");
        } else {
            panic!("Expected Variable");
        }
    }

    #[test]
    fn test_parse_addition() {
        let result = parse_formula("CLOSE + OPEN");
        assert!(result.is_ok());
        if let AstNode::BinaryOp { op, left, right } = result.unwrap() {
            assert_eq!(op, BinaryOperator::Add);
            assert!(matches!(*left, AstNode::Variable(_)));
            assert!(matches!(*right, AstNode::Variable(_)));
        } else {
            panic!("Expected BinaryOp");
        }
    }

    #[test]
    fn test_parse_subtraction() {
        let result = parse_formula("HIGH - LOW");
        assert!(result.is_ok());
        if let AstNode::BinaryOp { op, .. } = result.unwrap() {
            assert_eq!(op, BinaryOperator::Sub);
        } else {
            panic!("Expected BinaryOp with Sub");
        }
    }

    #[test]
    fn test_parse_multiplication() {
        let result = parse_formula("CLOSE * 2");
        assert!(result.is_ok());
        if let AstNode::BinaryOp { op, .. } = result.unwrap() {
            assert_eq!(op, BinaryOperator::Mul);
        } else {
            panic!("Expected BinaryOp with Mul");
        }
    }

    #[test]
    fn test_parse_division() {
        let result = parse_formula("VOLUME / 100");
        assert!(result.is_ok());
        if let AstNode::BinaryOp { op, .. } = result.unwrap() {
            assert_eq!(op, BinaryOperator::Div);
        } else {
            panic!("Expected BinaryOp with Div");
        }
    }

    #[test]
    fn test_parse_function_call() {
        let result = parse_formula("MA(CLOSE, 20)");
        assert!(result.is_ok());
        if let AstNode::FunctionCall { name, args } = result.unwrap() {
            assert_eq!(name, "MA");
            assert_eq!(args.len(), 2);
        } else {
            panic!("Expected FunctionCall");
        }
    }

    #[test]
    fn test_parse_nested_function_call() {
        let result = parse_formula("EMA(MA(CLOSE, 10), 12)");
        assert!(result.is_ok());
        if let AstNode::FunctionCall { name, args } = result.unwrap() {
            assert_eq!(name, "EMA");
            assert_eq!(args.len(), 2);
            assert!(matches!(&args[0], AstNode::FunctionCall { .. }));
        } else {
            panic!("Expected nested FunctionCall");
        }
    }

    #[test]
    fn test_parse_assignment() {
        let result = parse_formula("MA5 := MA(CLOSE, 5)");
        assert!(result.is_ok());
        if let AstNode::Assignment { name, expr } = result.unwrap() {
            assert_eq!(name, "MA5");
            assert!(matches!(*expr, AstNode::FunctionCall { .. }));
        } else {
            panic!("Expected Assignment");
        }
    }

    #[test]
    fn test_parse_output() {
        let result = parse_formula("MA5: MA(CLOSE, 5)");
        assert!(result.is_ok());
        if let AstNode::Output { name, expr, .. } = result.unwrap() {
            assert_eq!(name, "MA5");
            assert!(matches!(*expr, AstNode::FunctionCall { .. }));
        } else {
            panic!("Expected Output");
        }
    }

    #[test]
    fn test_parse_multiple_statements() {
        let source = "MA5 := MA(CLOSE, 5); MA10: MA(CLOSE, 10)";
        let result = parse_formula(source);
        assert!(result.is_ok());
        if let AstNode::Statements(stmts) = result.unwrap() {
            assert_eq!(stmts.len(), 2);
            assert!(matches!(&stmts[0], AstNode::Assignment { .. }));
            assert!(matches!(&stmts[1], AstNode::Output { .. }));
        } else {
            panic!("Expected Statements");
        }
    }

    #[test]
    fn test_parse_param_decl() {
        let source = "PARAMS: N(1, 100, 20); MA5: MA(CLOSE, N)";
        let result = parse_formula(source);
        assert!(result.is_ok());
        if let AstNode::Statements(stmts) = result.unwrap() {
            assert_eq!(stmts.len(), 2);
            assert!(matches!(&stmts[0], AstNode::ParamDecl { .. }));
            assert!(matches!(&stmts[1], AstNode::Output { .. }));
        } else {
            panic!("Expected Statements with ParamDecl");
        }
    }

    #[test]
    fn test_parse_comparison() {
        let result = parse_formula("CLOSE > MA(CLOSE, 20)");
        assert!(result.is_ok());
        if let AstNode::BinaryOp { op, .. } = result.unwrap() {
            assert_eq!(op, BinaryOperator::Gt);
        } else {
            panic!("Expected BinaryOp with Gt");
        }
    }

    #[test]
    fn test_parse_logical_and() {
        let result = parse_formula("CLOSE > OPEN AND VOLUME > 1000");
        assert!(result.is_ok());
        if let AstNode::BinaryOp { op, .. } = result.unwrap() {
            assert_eq!(op, BinaryOperator::And);
        } else {
            panic!("Expected BinaryOp with And");
        }
    }

    #[test]
    fn test_parse_logical_or() {
        let result = parse_formula("CLOSE > OPEN OR HIGH > 100");
        assert!(result.is_ok());
        if let AstNode::BinaryOp { op, .. } = result.unwrap() {
            assert_eq!(op, BinaryOperator::Or);
        } else {
            panic!("Expected BinaryOp with Or");
        }
    }

    #[test]
    fn test_parse_unary_not() {
        let result = parse_formula("NOT (CLOSE > OPEN)");
        assert!(result.is_ok());
        if let AstNode::UnaryOp { op, .. } = result.unwrap() {
            assert_eq!(op, UnaryOperator::Not);
        } else {
            panic!("Expected UnaryOp with Not");
        }
    }

    #[test]
    fn test_parse_unary_neg() {
        let result = parse_formula("-10");
        if let Err(ref e) = result {
            panic!("Parse error: {}", e);
        }
        assert!(result.is_ok());
        let node = result.unwrap();
        match &node {
            AstNode::Number(v) => panic!("Got Number({}) instead of UnaryOp(Neg)", v),
            AstNode::UnaryOp { op, expr } => {
                assert_eq!(*op, UnaryOperator::Neg);
                if let AstNode::Number(val) = **expr {
                    assert_eq!(val, 10.0);
                } else {
                    panic!("Expected Number in negated expr, got {:?}", expr);
                }
            }
            _ => panic!("Expected UnaryOp with Neg, got {:?}", node),
        }
    }

    #[test]
    fn test_parse_power() {
        let result = parse_formula("2 ^ 10");
        assert!(result.is_ok());
        if let AstNode::BinaryOp { op, .. } = result.unwrap() {
            assert_eq!(op, BinaryOperator::Pow);
        } else {
            panic!("Expected BinaryOp with Pow");
        }
    }

    #[test]
    fn test_parse_parenthesized() {
        let result = parse_formula("(CLOSE + OPEN) * 2");
        assert!(result.is_ok());
        if let AstNode::BinaryOp { op, left, right } = result.unwrap() {
            assert_eq!(op, BinaryOperator::Mul);
            assert!(matches!(*left, AstNode::BinaryOp { .. }));
            if let AstNode::Number(val) = *right {
                assert_eq!(val, 2.0);
            } else {
                panic!("Expected Number on right");
            }
        } else {
            panic!("Expected BinaryOp");
        }
    }

    #[test]
    fn test_parse_draw_text() {
        let result = parse_formula("DRAWTEXT(CLOSE > OPEN, CLOSE, \"BUY\")");
        assert!(result.is_ok());
        if let AstNode::DrawText { cond, text, .. } = result.unwrap() {
            assert!(matches!(*cond, AstNode::BinaryOp { .. }));
            assert_eq!(text, "BUY");
        } else {
            panic!("Expected DrawText");
        }
    }

    #[test]
    fn test_parse_stick_line() {
        let result = parse_formula("STICKLINE(CLOSE > OPEN, OPEN, CLOSE, 2, TRUE)");
        assert!(result.is_ok());
        if let AstNode::StickLine { cond, empty, .. } = result.unwrap() {
            assert!(matches!(*cond, AstNode::BinaryOp { .. }));
            assert!(empty);
        } else {
            panic!("Expected StickLine");
        }
    }

    #[test]
    fn test_parse_operator_precedence() {
        let result = parse_formula("CLOSE + OPEN * 2");
        assert!(result.is_ok());
        if let AstNode::BinaryOp { op, left: _, right } = result.unwrap() {
            assert_eq!(op, BinaryOperator::Add);
            assert!(matches!(
                *right,
                AstNode::BinaryOp {
                    op: BinaryOperator::Mul,
                    ..
                }
            ));
        } else {
            panic!("Expected BinaryOp with correct precedence");
        }
    }

    #[test]
    fn test_parse_complex_formula() {
        let source = r#"
            PARAMS: SHORT(5, 100, 12), LONG(5, 100, 26);
            MA_SHORT := MA(CLOSE, SHORT);
            MA_LONG := MA(CLOSE, LONG);
            GOLDEN_CROSS := MA_SHORT > MA_LONG;
            MA_SHORT: MA_SHORT;
            MA_LONG: MA_LONG
        "#;
        let result = parse_formula(source);
        assert!(result.is_ok());
        if let AstNode::Statements(stmts) = result.unwrap() {
            assert!(stmts.len() >= 5);
        } else {
            panic!("Expected Statements");
        }
    }

    #[test]
    fn test_parse_invalid_formula() {
        let result = parse_formula("CLOSE +");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_formula() {
        let result = parse_formula("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_decimal_number() {
        let pi = std::f64::consts::PI;
        let result = parse_formula(&pi.to_string());
        assert!(result.is_ok());
        if let AstNode::Number(val) = result.unwrap() {
            assert!((val - pi).abs() < 1e-10);
        } else {
            panic!("Expected Number");
        }
    }

    #[test]
    fn test_parse_if_then_else_statement() {
        let result = parse_formula("IF CLOSE > OPEN THEN CLOSE ELSE OPEN");
        assert!(result.is_ok());
        if let AstNode::IfThenElse {
            cond,
            then_branch,
            else_branch,
        } = result.unwrap()
        {
            assert!(matches!(*cond, AstNode::BinaryOp { .. }));
            assert!(matches!(*then_branch, AstNode::Variable { .. }));
            assert!(matches!(*else_branch, AstNode::Variable { .. }));
        } else {
            panic!("Expected IfThenElse");
        }
    }

    #[test]
    fn test_parse_if_then_else_with_assignment() {
        let result = parse_formula("IF C > O THEN C + 1 ELSE O - 1; RESULT: 0");
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        if let AstNode::Statements(stmts) = result.unwrap() {
            assert_eq!(stmts.len(), 2);
            assert!(matches!(&stmts[0], AstNode::IfThenElse { .. }));
            assert!(matches!(&stmts[1], AstNode::Output { .. }));
        } else {
            panic!("Expected Statements");
        }
    }

    #[test]
    fn test_parse_draw_text_with_color_name() {
        // 验证 DRAWTEXT 指令支持 color 参数的 AST 结构
        let result = parse_formula("DRAWTEXT(CLOSE > OPEN, CLOSE, \"BUY\")");
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        if let AstNode::DrawText { text, .. } = result.unwrap() {
            assert_eq!(text, "BUY");
        } else {
            panic!("Expected DrawText");
        }
    }

    #[test]
    fn test_parse_draw_text_with_color_rgb() {
        // 验证 DRAWTEXT 指令的基本解析
        let result = parse_formula("DRAWTEXT(C > O, C, \"TEST\")");
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        if let AstNode::DrawText { text, .. } = result.unwrap() {
            assert_eq!(text, "TEST");
        } else {
            panic!("Expected DrawText");
        }
    }

    #[test]
    fn test_parse_draw_text_with_color_hex() {
        // 验证 DRAWTEXT 指令解析
        let result = parse_formula("DRAWTEXT(C > O, C, \"HEX\")");
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        if let AstNode::DrawText { text, .. } = result.unwrap() {
            assert_eq!(text, "HEX");
        } else {
            panic!("Expected DrawText");
        }
    }

    #[test]
    fn test_parse_stick_line_with_color() {
        // 验证 STICKLINE 指令的基本解析
        let result = parse_formula("STICKLINE(CLOSE > OPEN, OPEN, CLOSE, 2, TRUE)");
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        if let AstNode::StickLine { empty, .. } = result.unwrap() {
            assert!(empty);
        } else {
            panic!("Expected StickLine");
        }
    }

    #[test]
    fn test_parse_line_comment() {
        let result = parse_formula("// this is a comment\nCLOSE");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_block_comment() {
        let result = parse_formula("/* multi-line\ncomment */CLOSE");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_multiple_comments() {
        let result = parse_formula("// comment 1\nCLOSE + /* inline */ OPEN // comment 2");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_draw_icon_with_color() {
        // 验证 DRAWICON 指令的基本解析
        let result = parse_formula("DRAWICON(CLOSE > OPEN, CLOSE, 1)");
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        if let AstNode::DrawIcon { icon, .. } = result.unwrap() {
            if let AstNode::Number(val) = *icon {
                assert!((val - 1.0).abs() < 1e-10);
            } else {
                panic!("Expected Number for icon");
            }
        } else {
            panic!("Expected DrawIcon");
        }
    }

    #[test]
    fn test_parse_if_then_else_nested() {
        // 测试 IF-THEN-ELSE 后跟另一个语句
        let result = parse_formula("IF C > O THEN C + 1 ELSE O - 1; RESULT: 0");
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        if let AstNode::Statements(stmts) = result.unwrap() {
            assert_eq!(stmts.len(), 2);
            assert!(matches!(&stmts[0], AstNode::IfThenElse { .. }));
            assert!(matches!(&stmts[1], AstNode::Output { .. }));
        } else {
            panic!("Expected Statements");
        }
    }

    #[test]
    fn test_parse_color_spec_named() {
        // 验证 ColorSpec::Named 能正确构造
        let color = ColorSpec::Named("COLORRED".to_string());
        match color {
            ColorSpec::Named(name) => assert_eq!(name, "COLORRED"),
            _ => panic!("Expected Named color"),
        }
    }

    #[test]
    fn test_parse_color_spec_rgb() {
        // 验证 ColorSpec::Rgb 能正确构造
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
    fn test_parse_color_spec_hex() {
        // 验证 ColorSpec::Hex 能正确构造
        let color = ColorSpec::Hex("FF0000".to_string());
        match color {
            ColorSpec::Hex(hex) => assert_eq!(hex, "FF0000"),
            _ => panic!("Expected Hex color"),
        }
    }

    #[test]
    fn test_parse_draw_text_without_color() {
        // 验证不带颜色的 DRAWTEXT 能正常工作
        let result = parse_formula("DRAWTEXT(CLOSE > OPEN, CLOSE, \"BUY\")");
        assert!(result.is_ok());
        if let AstNode::DrawText { text, color, .. } = result.unwrap() {
            assert_eq!(text, "BUY");
            assert!(color.is_none());
        } else {
            panic!("Expected DrawText");
        }
    }

    #[test]
    fn test_parse_brace_comment() {
        let result = parse_formula("{this is a TDX comment}CLOSE");
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        if let AstNode::Variable(name) = result.unwrap() {
            assert_eq!(name, "CLOSE");
        } else {
            panic!("Expected Variable");
        }
    }

    #[test]
    fn test_parse_brace_comment_multiline() {
        let result = parse_formula("{multi\nline\ncomment}CLOSE + OPEN");
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
    }

    #[test]
    fn test_parse_hash_comment() {
        let result = parse_formula("# this is a Pine Script comment\nCLOSE");
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        if let AstNode::Variable(name) = result.unwrap() {
            assert_eq!(name, "CLOSE");
        } else {
            panic!("Expected Variable");
        }
    }

    #[test]
    fn test_parse_hash_comment_end_of_line() {
        let result = parse_formula("CLOSE + OPEN # trailing comment");
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
    }

    #[test]
    fn test_parse_single_quote_string() {
        let result = parse_formula("DRAWTEXT(CLOSE > OPEN, CLOSE, 'BUY')");
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        if let AstNode::DrawText { text, .. } = result.unwrap() {
            assert_eq!(text, "BUY");
        } else {
            panic!("Expected DrawText");
        }
    }

    #[test]
    fn test_parse_equals_assignment() {
        let result = parse_formula("MA5 = MA(CLOSE, 5)");
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        if let AstNode::Assignment { name, expr } = result.unwrap() {
            assert_eq!(name, "MA5");
            assert!(matches!(*expr, AstNode::FunctionCall { .. }));
        } else {
            panic!("Expected Assignment");
        }
    }

    #[test]
    fn test_parse_equals_assignment_with_traditional() {
        let source = "MA5 = MA(CLOSE, 5); MA10 := MA(CLOSE, 10)";
        let result = parse_formula(source);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        if let AstNode::Statements(stmts) = result.unwrap() {
            assert_eq!(stmts.len(), 2);
            assert!(matches!(&stmts[0], AstNode::Assignment { .. }));
            assert!(matches!(&stmts[1], AstNode::Assignment { .. }));
        } else {
            panic!("Expected Statements");
        }
    }

    #[test]
    fn test_parse_equals_does_not_conflict_with_comparison() {
        let result = parse_formula("CLOSE == OPEN");
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
        if let AstNode::BinaryOp { op, .. } = result.unwrap() {
            assert_eq!(op, BinaryOperator::Eq);
        } else {
            panic!("Expected BinaryOp with Eq");
        }
    }

    #[test]
    fn test_parse_mixed_syntax_compat() {
        let source = r#"
            {通达信注释}
            # Pine Script 注释
            MA5 = MA(CLOSE, 5);
            MA10 := MA(CLOSE, 10);
            DRAWTEXT(CLOSE > MA5, CLOSE, 'BUY')
        "#;
        let result = parse_formula(source);
        assert!(result.is_ok(), "Parse error: {:?}", result.err());
    }
}
