//! Pine Script v5 parser — converts source text into `PineAst`.

use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

/// Pine Script AST root.
#[derive(Debug, Clone)]
pub struct PineAst {
    pub version: Option<u32>,
    pub items: Vec<PineAstNode>,
}

/// Pine Script AST node.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PineAstNode {
  /// //@version=N
  VersionAnnotation(u32),
  /// indicator("title", overlay=true, ...)
  IndicatorDecl {
    title: String,
    args: Vec<(String, PineAstNode)>,
  },
  /// study() — legacy alias
  StudyDecl {
    title: String,
    args: Vec<(String, PineAstNode)>,
  },
  /// var / varip declaration
  VarDecl {
    is_varip: bool,
    type_qualifier: Option<PineType>,
    name: String,
    init: Box<PineAstNode>,
  },
  /// input(...) declaration
  InputDecl {
    input_type: Option<String>,
    default: Box<PineAstNode>,
    args: Vec<PineAstNode>,
  },
  /// fn(x) => body
  FunctionDecl {
    name: String,
    params: Vec<String>,
    body: FunctionBody,
  },
  /// x = expr  or  x := expr
  Assignment {
    name: String,
    is_reassign: bool,
    expr: Box<PineAstNode>,
  },
  /// [a, b, ...] = expr  — tuple destructuring assignment
  TupleAssign {
    names: Vec<String>,
    expr: Box<PineAstNode>,
  },
  /// if / else block
  IfStmt {
    cond: Box<PineAstNode>,
    then_body: Vec<PineAstNode>,
    else_body: Option<Vec<PineAstNode>>,
  },
  /// for i = start to end [by step]
  ForStmt {
    var: String,
    start: Box<PineAstNode>,
    end: Box<PineAstNode>,
    step: Option<Box<PineAstNode>>,
    body: Vec<PineAstNode>,
  },
  /// while cond
  WhileStmt {
    cond: Box<PineAstNode>,
    body: Vec<PineAstNode>,
  },
  /// plot(...)
  PlotCall {
    value: Box<PineAstNode>,
    args: Vec<(Option<String>, PineAstNode)>,
  },
  /// hline(...)
  HlineCall {
    price: Box<PineAstNode>,
    args: Vec<(Option<String>, PineAstNode)>,
  },
  /// fill(plot1, plot2, ...)
  FillCall {
    plot1: Box<PineAstNode>,
    plot2: Box<PineAstNode>,
    args: Vec<(Option<String>, PineAstNode)>,
  },
  /// Expression statement
  Expr(Box<PineAstNode>),
  // --- expressions ---
  Number(f64),
  StringLit(String),
  NaLiteral,
  Identifier(String),
  BinaryOp {
    op: PineBinaryOp,
    left: Box<PineAstNode>,
    right: Box<PineAstNode>,
  },
  UnaryOp {
    op: PineUnaryOp,
    expr: Box<PineAstNode>,
  },
  Ternary {
    cond: Box<PineAstNode>,
    then_expr: Box<PineAstNode>,
    else_expr: Box<PineAstNode>,
  },
  FunctionCall {
    namespace: Option<String>,
    name: String,
    args: Vec<(Option<String>, PineAstNode)>,
  },
  IndexAccess {
    array: Box<PineAstNode>,
    index: Box<PineAstNode>,
  },
  /// barstate.isconfirmed, barstate.islast, barstate.isnew
  BarstateAccess {
    field: String,
  },
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FunctionBody {
  Expr(Box<PineAstNode>),
  Block(Vec<PineAstNode>),
}

/// Pine built-in type qualifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PineType {
  Series,
  Simple,
  Const,
  Input,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PineBinaryOp {
  Add,
  Sub,
  Mul,
  Div,
  Mod,
  Eq,
  Ne,
  Gt,
  Lt,
  Gte,
  Lte,
  And,
  Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PineUnaryOp {
  Not,
  Neg,
}

/// Parse error with source location.
#[derive(Debug, Clone)]
pub struct PineError {
  pub message: String,
  pub line: usize,
  pub column: usize,
}

impl std::fmt::Display for PineError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "Pine parse error at line {}, column {}: {}",
      self.line, self.column, self.message
    )
  }
}

impl std::error::Error for PineError {}

#[derive(Parser)]
#[grammar = "formula/pine/grammar.pest"]
struct PineGrammar;

/// Parse Pine Script v5 source into AST.
pub fn parse_pine(source: &str) -> Result<PineAst, PineError> {
  let program_pairs = PineGrammar::parse(Rule::program, source).map_err(|e| {
    let (line, col) = line_col_from_pest_error(source, &e);
    PineError {
      message: e.to_string(),
      line,
      column: col,
    }
  })?;

  let mut version = None;
  let mut items = Vec::new();

  for pair in program_pairs {
    if pair.as_rule() != Rule::program {
      continue;
    }
    for inner in pair.into_inner() {
      match inner.as_rule() {
        Rule::program_item => {
          for item in inner.into_inner() {
            match item.as_rule() {
              Rule::version_annotation => {
                let v = parse_version(item)?;
                version = Some(v);
                items.push(PineAstNode::VersionAnnotation(v));
              }
              Rule::declaration => {
                items.push(parse_declaration(item)?);
              }
              Rule::statement => {
                items.push(parse_statement(item)?);
              }
              _ => {}
            }
          }
        }
        _ => {}
      }
    }
  }

  Ok(PineAst { version, items })
}

fn line_col_from_pest_error(_source: &str, err: &pest::error::Error<Rule>) -> (usize, usize) {
  match &err.line_col {
    pest::error::LineColLocation::Pos((line, col)) => (*line, *col),
    pest::error::LineColLocation::Span((start_line, start_col), _) => (*start_line, *start_col),
  }
}

fn parse_version(pair: Pair<Rule>) -> Result<u32, PineError> {
  let num = pair
    .into_inner()
    .find(|p| p.as_rule() == Rule::version_num)
    .map(|p| p.as_str())
    .unwrap_or("5");
  num.parse::<u32>().map_err(|e| PineError {
    message: format!("Invalid version number: {}", e),
    line: 1,
    column: 1,
  })
}

fn parse_declaration(pair: Pair<Rule>) -> Result<PineAstNode, PineError> {
  let inner = pair.into_inner().next().expect("empty declaration");
  match inner.as_rule() {
    Rule::indicator_decl => parse_indicator(inner, false),
    Rule::study_decl => parse_indicator(inner, true),
    Rule::var_decl => parse_var_decl(inner),
    Rule::input_decl => parse_input_decl(inner),
    Rule::function_decl => parse_function_decl(inner),
    _ => Err(PineError {
      message: format!("Unknown declaration: {:?}", inner.as_rule()),
      line: 1,
      column: 1,
    }),
  }
}

fn parse_indicator(pair: Pair<Rule>, is_study: bool) -> Result<PineAstNode, PineError> {
  let mut inner = pair.into_inner();
  let title_pair = inner.next().expect("indicator title");
  let title = match parse_string_lit(title_pair)? {
    PineAstNode::StringLit(s) => s,
    _ => {
      return Err(PineError {
        message: "indicator title must be a string literal".to_string(),
        line: 1,
        column: 1,
      });
    }
  };

  let mut args = Vec::new();
  for arg in inner {
    if arg.as_rule() == Rule::indicator_arg {
      let mut ai = arg.into_inner();
      let name = ai.next().expect("arg name").as_str().to_string();
      let expr = parse_expression(ai.next().expect("arg expr"))?;
      args.push((name, expr));
    }
  }

  if is_study {
    Ok(PineAstNode::StudyDecl { title, args })
  } else {
    Ok(PineAstNode::IndicatorDecl { title, args })
  }
}

fn parse_var_decl(pair: Pair<Rule>) -> Result<PineAstNode, PineError> {
  let mut inner = pair.into_inner();
  let var_kw = inner.next().expect("var_kw").as_str();
  let is_varip = var_kw == "varip";

  let mut type_qualifier = None;
  let mut name_pair = inner.next().expect("name or type");
  if name_pair.as_rule() == Rule::type_qualifier {
    type_qualifier = Some(parse_type_qualifier(name_pair.as_str()));
    name_pair = inner.next().expect("name");
  }

  let name = name_pair.as_str().to_string();
  let init = parse_expression(inner.next().expect("init"))?;

  Ok(PineAstNode::VarDecl {
    is_varip,
    type_qualifier,
    name,
    init: Box::new(init),
  })
}

fn parse_type_qualifier(s: &str) -> PineType {
  match s {
    "series" => PineType::Series,
    "simple" => PineType::Simple,
    "const" => PineType::Const,
    "input" => PineType::Input,
    _ => PineType::Series,
  }
}

fn parse_input_decl(pair: Pair<Rule>) -> Result<PineAstNode, PineError> {
  let mut inner = pair.into_inner();
  let first = inner.next().expect("input decl");

  match first.as_rule() {
    Rule::input_typed_decl => {
      let mut ti = first.into_inner();
      let input_type = ti
        .next()
        .filter(|p| p.as_rule() == Rule::input_type)
        .map(|p| p.as_str().to_string());
      let default = parse_expression(ti.next().expect("default"))?;
      let mut args = Vec::new();
      for arg in ti {
        if arg.as_rule() == Rule::input_arg {
          args.push(parse_input_arg(arg)?);
        }
      }
      Ok(PineAstNode::InputDecl {
        input_type,
        default: Box::new(default),
        args,
      })
    }
    Rule::expression => {
      let default = parse_expression(first)?;
      let mut args = Vec::new();
      for arg in inner {
        if arg.as_rule() == Rule::input_arg {
          args.push(parse_input_arg(arg)?);
        }
      }
      Ok(PineAstNode::InputDecl {
        input_type: None,
        default: Box::new(default),
        args,
      })
    }
    _ => {
      let mut input_type = None;
      let mut default_pair = first;
      if default_pair.as_rule() == Rule::input_type {
        input_type = Some(default_pair.as_str().to_string());
        default_pair = inner.next().expect("default expr");
      }
      let default = parse_expression(default_pair)?;
      let mut args = Vec::new();
      for arg in inner {
        if arg.as_rule() == Rule::input_arg {
          args.push(parse_input_arg(arg)?);
        }
      }
      Ok(PineAstNode::InputDecl {
        input_type,
        default: Box::new(default),
        args,
      })
    }
  }
}

fn parse_input_arg(pair: Pair<Rule>) -> Result<PineAstNode, PineError> {
  let inner = pair.into_inner().next().expect("input arg");
  match inner.as_rule() {
    Rule::string => parse_string_lit(inner),
    Rule::expression => parse_expression(inner),
    _ => {
      let mut ii = inner.into_inner();
      let expr = parse_expression(ii.next().expect("named arg expr"))?;
      Ok(expr)
    }
  }
}

fn parse_function_decl(pair: Pair<Rule>) -> Result<PineAstNode, PineError> {
  let mut inner = pair.into_inner();
  let name = inner.next().expect("fn name").as_str().to_string();

  let mut params = Vec::new();
  if let Some(pl) = inner.peek() {
    if pl.as_rule() == Rule::param_list {
      let pl = inner.next().unwrap();
      for p in pl.into_inner() {
        params.push(p.as_str().to_string());
      }
    }
  }

  let body_pair = inner.next().expect("fn body");
  let body = match body_pair.as_rule() {
    Rule::expression => FunctionBody::Expr(Box::new(parse_expression(body_pair)?)),
    Rule::block => FunctionBody::Block(parse_block(body_pair)?),
    _ => FunctionBody::Expr(Box::new(parse_expression(body_pair)?)),
  };

  Ok(PineAstNode::FunctionDecl {
    name,
    params,
    body,
  })
}

fn parse_block(pair: Pair<Rule>) -> Result<Vec<PineAstNode>, PineError> {
  let mut stmts = Vec::new();
  for stmt in pair.into_inner() {
    if stmt.as_rule() == Rule::statement {
      stmts.push(parse_statement(stmt)?);
    }
  }
  Ok(stmts)
}

fn parse_statement(pair: Pair<Rule>) -> Result<PineAstNode, PineError> {
  let inner = pair.into_inner().next().expect("empty statement");
  match inner.as_rule() {
    Rule::assignment => parse_assignment(inner),
    Rule::if_stmt => parse_if_stmt(inner),
    Rule::for_stmt => parse_for_stmt(inner),
    Rule::while_stmt => parse_while_stmt(inner),
    Rule::plot_call => parse_plot_call(inner),
    Rule::hline_call => parse_hline_call(inner),
    Rule::fill_call => parse_fill_call(inner),
    Rule::function_call_stmt => {
      let call = inner.into_inner().next().expect("call");
      let node = parse_qualified_call(call)?;
      Ok(PineAstNode::Expr(Box::new(node)))
    }
    Rule::expression => Ok(PineAstNode::Expr(Box::new(parse_expression(inner)?))),
    _ => Err(PineError {
      message: format!("Unknown statement: {:?}", inner.as_rule()),
      line: 1,
      column: 1,
    }),
  }
}

fn parse_assignment(pair: Pair<Rule>) -> Result<PineAstNode, PineError> {
  // Collect inner pairs; leading `identifier` tokens are the LHS target(s),
  // followed by `assign_op`, then the RHS `expression`. A tuple assignment
  // like `[a, b] = expr` yields multiple leading identifiers.
  let parts: Vec<Pair<Rule>> = pair.into_inner().collect();
  let mut names = Vec::new();
  let mut i = 0;
  while i < parts.len() && parts[i].as_rule() == Rule::identifier {
    names.push(parts[i].as_str().to_string());
    i += 1;
  }
  let op = parts
    .get(i)
    .map(|p| p.as_str().to_string())
    .unwrap_or_else(|| "=".to_string());
  let is_reassign = op == ":=";
  i += 1;
  let expr_pair = parts
    .into_iter()
    .nth(i)
    .expect("assignment RHS expression");
  let expr = parse_expression(expr_pair)?;
  if names.len() <= 1 {
    Ok(PineAstNode::Assignment {
      name: names.into_iter().next().unwrap_or_default(),
      is_reassign,
      expr: Box::new(expr),
    })
  } else {
    Ok(PineAstNode::TupleAssign {
      names,
      expr: Box::new(expr),
    })
  }
}

fn parse_if_stmt(pair: Pair<Rule>) -> Result<PineAstNode, PineError> {
  let mut inner = pair.into_inner();
  let cond = parse_expression(inner.next().expect("cond"))?;
  let then_body = parse_block(inner.next().expect("then"))?;
  let else_body = inner.next().map(|p| parse_block(p)).transpose()?;
  Ok(PineAstNode::IfStmt {
    cond: Box::new(cond),
    then_body,
    else_body,
  })
}

fn parse_for_stmt(pair: Pair<Rule>) -> Result<PineAstNode, PineError> {
  let mut inner = pair.into_inner();
  let var = inner.next().expect("var").as_str().to_string();
  let start = parse_expression(inner.next().expect("start"))?;
  let end = parse_expression(inner.next().expect("end"))?;
  let step = inner
    .find(|p| p.as_rule() == Rule::expression)
    .map(|p| parse_expression(p))
    .transpose()?;
  let body_pair = inner.into_iter().find(|p| p.as_rule() == Rule::block);
  let body = match body_pair {
    Some(p) => parse_block(p)?,
    None => Vec::new(),
  };
  Ok(PineAstNode::ForStmt {
    var,
    start: Box::new(start),
    end: Box::new(end),
    step: step.map(Box::new),
    body,
  })
}

fn parse_while_stmt(pair: Pair<Rule>) -> Result<PineAstNode, PineError> {
  let mut inner = pair.into_inner();
  let cond = parse_expression(inner.next().expect("cond"))?;
  let body = parse_block(inner.next().expect("body"))?;
  Ok(PineAstNode::WhileStmt {
    cond: Box::new(cond),
    body,
  })
}

fn parse_plot_call(pair: Pair<Rule>) -> Result<PineAstNode, PineError> {
  let args_pair = pair.into_inner().next().expect("plot args");
  let (value, args) = parse_named_args(args_pair)?;
  Ok(PineAstNode::PlotCall {
    value: Box::new(value),
    args,
  })
}

fn parse_hline_call(pair: Pair<Rule>) -> Result<PineAstNode, PineError> {
  let args_pair = pair.into_inner().next().expect("hline args");
  let (price, args) = parse_named_args(args_pair)?;
  Ok(PineAstNode::HlineCall {
    price: Box::new(price),
    args,
  })
}

fn parse_fill_call(pair: Pair<Rule>) -> Result<PineAstNode, PineError> {
  let args_pair = pair.into_inner().next().expect("fill args");
  let mut inner = args_pair.into_inner();
  let plot1 = parse_expression(inner.next().expect("plot1"))?;
  let plot2 = parse_expression(inner.next().expect("plot2"))?;
  let mut args = Vec::new();
  for arg in inner {
    match arg.as_rule() {
      Rule::fill_arg => {
        let mut ai = arg.into_inner();
        let first = ai.next().expect("fill arg item");
      if first.as_rule() == Rule::identifier || first.as_rule() == Rule::namespace || first.as_rule() == Rule::arg_name {
        let name = named_arg_name(first);
        let expr = parse_expression(ai.next().expect("fill named expr"))?;
        args.push((Some(name), expr));
      } else {
          args.push((None, parse_expression(first)?));
        }
      }
      _ => {}
    }
  }
  Ok(PineAstNode::FillCall {
    plot1: Box::new(plot1),
    plot2: Box::new(plot2),
    args,
  })
}

/// Extract the bare name string from a named-argument LHS node, which may be an
/// `identifier`, `namespace`, or (after the grammar refactor) an `arg_name`
/// wrapper around one of those.
fn named_arg_name(item: Pair<Rule>) -> String {
  if item.as_rule() == Rule::arg_name {
    item.into_inner().next().expect("arg name").as_str().to_string()
  } else {
    item.as_str().to_string()
  }
}

fn parse_named_args(pair: Pair<Rule>) -> Result<(PineAstNode, Vec<(Option<String>, PineAstNode)>), PineError> {
  let rule = pair.as_rule();
  let mut inner = pair.into_inner();
  let first = inner.next().expect("first arg");
  let value = parse_expression(first)?;
  let mut args = Vec::new();

  let arg_rule = match rule {
    Rule::plot_args => Rule::plot_arg,
    Rule::hline_args => Rule::hline_arg,
    _ => Rule::plot_arg,
  };

  for arg in inner {
    if arg.as_rule() == arg_rule {
      let mut ai = arg.into_inner();
      let item = ai.next().expect("arg item");
      if item.as_rule() == Rule::identifier || item.as_rule() == Rule::namespace || item.as_rule() == Rule::arg_name {
        let name = named_arg_name(item);
        let expr = parse_expression(ai.next().expect("named expr"))?;
        args.push((Some(name), expr));
      } else {
        args.push((None, parse_expression(item)?));
      }
    }
  }
  Ok((value, args))
}

fn parse_expression(pair: Pair<Rule>) -> Result<PineAstNode, PineError> {
  let rule = pair.as_rule();
  match rule {
    Rule::postfix => parse_expr_node(pair),
    Rule::expression | Rule::ternary | Rule::logical_or | Rule::logical_and | Rule::comparison
    | Rule::addition | Rule::multiplication | Rule::unary => {
      let inner = pair.into_inner().next();
      if inner.is_none() {
        return Ok(PineAstNode::Number(0.0));
      }
      let inner = inner.unwrap();
      if inner.as_rule() == rule {
        return parse_expression(inner);
      }
      parse_expr_node(inner)
    }
    _ => parse_expr_node(pair),
  }
}

fn parse_expr_node(pair: Pair<Rule>) -> Result<PineAstNode, PineError> {
  match pair.as_rule() {
    Rule::ternary => {
      let mut inner = pair.into_inner();
      let cond = parse_expression(inner.next().expect("cond"))?;
      if let Some(then_p) = inner.next() {
        let then_expr = parse_expression(then_p)?;
        let else_expr = parse_expression(inner.next().expect("else"))?;
        Ok(PineAstNode::Ternary {
          cond: Box::new(cond),
          then_expr: Box::new(then_expr),
          else_expr: Box::new(else_expr),
        })
      } else {
        Ok(cond)
      }
    }
    Rule::logical_or => parse_binary_chain(pair, "or", PineBinaryOp::Or),
    Rule::logical_and => parse_binary_chain(pair, "and", PineBinaryOp::And),
    Rule::comparison => {
      let mut inner = pair.into_inner();
      let left = parse_expression(inner.next().expect("left"))?;
      if let Some(op_pair) = inner.next() {
        let op_str = op_pair.as_str();
        let right = parse_expression(inner.next().expect("right"))?;
        let op = match op_str {
          "==" => PineBinaryOp::Eq,
          "!=" => PineBinaryOp::Ne,
          ">" => PineBinaryOp::Gt,
          "<" => PineBinaryOp::Lt,
          ">=" => PineBinaryOp::Gte,
          "<=" => PineBinaryOp::Lte,
          _ => PineBinaryOp::Eq,
        };
        Ok(PineAstNode::BinaryOp {
          op,
          left: Box::new(left),
          right: Box::new(right),
        })
      } else {
        Ok(left)
      }
    }
    Rule::addition => {
      let mut inner = pair.into_inner();
      let first = parse_expression(inner.next().expect("first"))?;
      let mut result = first;
      while let Some(op_pair) = inner.next() {
        if op_pair.as_rule() == Rule::add_op {
          let op = match op_pair.as_str() {
            "+" => PineBinaryOp::Add,
            "-" => PineBinaryOp::Sub,
            _ => PineBinaryOp::Add,
          };
          let right = parse_expression(inner.next().expect("right"))?;
          result = PineAstNode::BinaryOp {
            op,
            left: Box::new(result),
            right: Box::new(right),
          };
        }
      }
      Ok(result)
    }
    Rule::multiplication => {
      let mut inner = pair.into_inner();
      let first = parse_expression(inner.next().expect("first"))?;
      let mut result = first;
      while let Some(op_pair) = inner.next() {
        if op_pair.as_rule() == Rule::mul_op {
          let op_str = op_pair.as_str();
          let right = parse_expression(inner.next().expect("right"))?;
          let op = match op_str {
            "*" => PineBinaryOp::Mul,
            "/" => PineBinaryOp::Div,
            "%" => PineBinaryOp::Mod,
            _ => PineBinaryOp::Mul,
          };
          result = PineAstNode::BinaryOp {
            op,
            left: Box::new(result),
            right: Box::new(right),
          };
        }
      }
      Ok(result)
    }
    Rule::unary => {
      let mut inner = pair.into_inner();
      let first = inner.next().expect("unary");
      match first.as_str() {
        "not" => {
          let expr = parse_expression(inner.next().expect("not expr"))?;
          Ok(PineAstNode::UnaryOp {
            op: PineUnaryOp::Not,
            expr: Box::new(expr),
          })
        }
        "-" => {
          let expr = parse_expression(inner.next().expect("neg expr"))?;
          Ok(PineAstNode::UnaryOp {
            op: PineUnaryOp::Neg,
            expr: Box::new(expr),
          })
        }
        _ => parse_expression(first),
      }
    }
    Rule::postfix => {
      let mut inner = pair.into_inner();
      let primary = parse_expr_node(inner.next().expect("primary"))?;
      let mut result = primary;
      for access in inner {
        if access.as_rule() == Rule::index_access {
          let idx = parse_expression(access.into_inner().next().expect("idx"))?;
          result = PineAstNode::IndexAccess {
            array: Box::new(result),
            index: Box::new(idx),
          };
        }
      }
      Ok(result)
    }
    Rule::number => {
      let n = pair.as_str().parse::<f64>().map_err(|e| PineError {
        message: format!("Invalid number: {}", e),
        line: 1,
        column: 1,
      })?;
      Ok(PineAstNode::Number(n))
    }
    Rule::string => parse_string_lit(pair),
    Rule::bool_literal => {
      let v = pair.as_str() == "true";
      Ok(PineAstNode::Number(if v { 1.0 } else { 0.0 }))
    }
    Rule::na_literal => Ok(PineAstNode::NaLiteral),
    Rule::variable => {
      let inner = pair.into_inner().next().expect("var");
      match inner.as_rule() {
        Rule::barstate_var => {
          let field = inner.into_inner()
            .find(|p| p.as_rule() == Rule::barstate_field)
            .map(|p| p.as_str().to_string())
            .unwrap_or_default();
          Ok(PineAstNode::BarstateAccess { field })
        }
        _ => Ok(PineAstNode::Identifier(inner.as_str().to_string())),
      }
    }
    Rule::namespace_access => {
      // e.g. `color.blue`, `syminfo.tickerid` — surface as a dotted identifier.
      // Plot/draw color args are ignored by the mapper; `syminfo.tickerid`
      // becomes a Variable node for `request.security` argument mapping.
      Ok(PineAstNode::Identifier(pair.as_str().to_string()))
    }
    Rule::function_call => parse_function_call(pair),
    Rule::primary => {
      let inner = pair.into_inner().next().expect("primary inner");
      parse_expr_node(inner)
    }
    Rule::expression => parse_expression(pair),
    _ => Err(PineError {
      message: format!("Unexpected expression node: {:?}", pair.as_rule()),
      line: 1,
      column: 1,
    }),
  }
}

fn parse_binary_chain(
  pair: Pair<Rule>,
  op_token: &str,
  default_op: PineBinaryOp,
) -> Result<PineAstNode, PineError> {
  let mut inner = pair.into_inner();
  let first = parse_expression(inner.next().expect("first"))?;
  let mut result = first;
  for item in inner {
    if item.as_str() == op_token {
      continue;
    }
    let right = parse_expression(item)?;
    result = PineAstNode::BinaryOp {
      op: default_op,
      left: Box::new(result),
      right: Box::new(right),
    };
  }
  Ok(result)
}

fn parse_function_call(pair: Pair<Rule>) -> Result<PineAstNode, PineError> {
  let inner = pair.into_inner().next().expect("call type");
  match inner.as_rule() {
    Rule::qualified_call => parse_qualified_call(inner),
    Rule::simple_call => {
      let mut si = inner.into_inner();
      let name = si.next().expect("name").as_str().to_string();
      let args = si
        .next()
        .map(|al| parse_arg_list(al))
        .transpose()?
        .unwrap_or_default();
      Ok(PineAstNode::FunctionCall {
        namespace: None,
        name,
        args,
      })
    }
    _ => parse_qualified_call(inner),
  }
}

fn parse_qualified_call(pair: Pair<Rule>) -> Result<PineAstNode, PineError> {
  let mut inner = pair.into_inner();
  let ns = inner.next().map(|p| p.as_str().to_string());
  let name = inner.next().expect("fn name").as_str().to_string();
  let args = inner
    .next()
    .map(|al| parse_arg_list(al))
    .transpose()?
    .unwrap_or_default();

  // request.security is a two-part namespace
  let (namespace, name) = if ns == Some("request".to_string()) && name == "security" {
    (Some("request".to_string()), "security".to_string())
  } else if let Some(ns_str) = ns {
    (Some(ns_str), name)
  } else {
    (None, name)
  };

  Ok(PineAstNode::FunctionCall {
    namespace,
    name,
    args,
  })
}

fn parse_arg_list(pair: Pair<Rule>) -> Result<Vec<(Option<String>, PineAstNode)>, PineError> {
  let mut args = Vec::new();
  for item in pair.into_inner() {
    if item.as_rule() == Rule::arg_item {
      let mut ai = item.into_inner();
      let first = ai.next().expect("arg");
      if (first.as_rule() == Rule::identifier || first.as_rule() == Rule::namespace || first.as_rule() == Rule::arg_name) && ai.peek().is_some() {
        let name = named_arg_name(first);
        let expr = parse_expression(ai.next().expect("named"))?;
        args.push((Some(name), expr));
      } else {
        args.push((None, parse_expression(first)?));
      }
    } else if item.as_rule() == Rule::expression {
      args.push((None, parse_expression(item)?));
    }
  }
  Ok(args)
}

fn parse_string_lit(pair: Pair<Rule>) -> Result<PineAstNode, PineError> {
  let s = pair.as_str();
  let inner = s.trim_matches('"');
  Ok(PineAstNode::StringLit(inner.to_string()))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_version_annotation() {
    let ast = parse_pine("//@version=5\n").unwrap();
    assert_eq!(ast.version, Some(5));
  }

  #[test]
  fn test_parse_indicator() {
    let src = "//@version=5\nindicator(\"RSI\", overlay=false)\n";
    let ast = parse_pine(src).unwrap();
    assert!(ast.items.iter().any(|n| matches!(n, PineAstNode::IndicatorDecl { .. })));
  }

  #[test]
  fn test_parse_assignment_and_plot() {
    let src = "//@version=5\nindicator(\"Test\")\nval = close\nplot(val)\n";
    let ast = parse_pine(src).unwrap();
    assert!(ast.items.len() >= 2);
  }

  #[test]
  fn test_parse_ternary() {
    let src = "//@version=5\nindicator(\"T\")\nx = close > open ? high : low\n";
    let ast = parse_pine(src).unwrap();
    assert!(ast.items.iter().any(|n| {
      matches!(n, PineAstNode::Assignment { .. })
    }));
  }

  #[test]
  fn test_parse_ta_call() {
    let src = "//@version=5\nindicator(\"MA\")\nma = ta.sma(close, 20)\nplot(ma)\n";
    let ast = parse_pine(src).unwrap();
    let has_call = ast.items.iter().any(|n| {
      if let PineAstNode::Assignment { expr, .. } = n {
        if let PineAstNode::FunctionCall { namespace, name, .. } = &**expr {
          return namespace.as_deref() == Some("ta") && name == "sma";
        }
      }
      false
    });
    assert!(has_call);
  }

  #[test]
  fn test_error_has_line() {
    let err = parse_pine("//@version=5\nindicator(\n").unwrap_err();
    assert!(err.line > 0);
  }

  #[test]
  fn test_parse_history_operator() {
    let src = "//@version=5\nindicator(\"T\")\nprev = close[1]\n";
    let ast = parse_pine(src).unwrap();
    let has_index = ast.items.iter().any(|n| {
      if let PineAstNode::Assignment { expr, .. } = n {
        matches!(&**expr, PineAstNode::IndexAccess { .. })
      } else {
        false
      }
    });
    assert!(has_index);
  }

  #[test]
  fn test_parse_barstate() {
    let src = "//@version=5\nindicator(\"T\")\nx = barstate.isconfirmed\n";
    let ast = parse_pine(src).unwrap();
    let has_barstate = ast.items.iter().any(|n| {
      if let PineAstNode::Assignment { expr, .. } = n {
        matches!(&**expr, PineAstNode::BarstateAccess { field } if field == "isconfirmed")
      } else {
        false
      }
    });
    assert!(has_barstate);
  }
}
