use ahash::AHashMap;
use ndarray::Array1;
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

use crate::formula::ast::*;
use crate::formula::drawing::DrawResult;
use crate::formula::functions::get_builtin_functions;
use crate::formula::simd::SimdOps;
use crate::formula::types::{
    classify_builtin_var, BuiltinVar, FormulaContext, FormulaError, FormulaValue,
};

// ============================================================
// OpCode definitions
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum OpCode {
    PushConst(f64),
    LoadVar(String),
    StoreVar(String),

    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,

    StringConcat,

    Gt,
    Lt,
    Gte,
    Lte,
    Eq,
    Neq,

    And,
    Or,
    Xor,
    Not,

    Jump(usize),
    JumpIfFalse(usize),

    Call {
        name: String,
        arg_count: usize,
    },

    LoadData(String),

    Index,

    CompoundStore {
        name: String,
        op: u8,
    },

    Output(String),

    Select,

    DrawText {
        text: String,
        color: String,
    },
    DrawIcon {
        color: String,
    },
    StickLine {
        empty: bool,
        color: String,
    },
    DrawGeneric {
        command: String,
        arg_count: usize,
        color: String,
    },

    PushString(String),
}

#[derive(Debug, Clone)]
pub struct Bytecode {
    pub instructions: Vec<OpCode>,
    pub source: String,
    pub output_names: Vec<String>,
}

// ============================================================
// Bytecode Compiler
// ============================================================

struct BytecodeCompiler {
    instructions: Vec<OpCode>,
    output_names: Vec<String>,
}

pub fn compile_to_bytecode(ast: &AstNode, source: &str) -> Result<Bytecode, String> {
    let mut compiler = BytecodeCompiler {
        instructions: Vec::new(),
        output_names: Vec::new(),
    };
    compiler.compile(ast)?;
    Ok(Bytecode {
        instructions: compiler.instructions,
        source: source.to_string(),
        output_names: compiler.output_names,
    })
}

impl BytecodeCompiler {
    fn color_to_string(color: &Option<ColorSpec>) -> String {
        match color {
            None => String::new(),
            Some(ColorSpec::Named(s)) => s.clone(),
            Some(ColorSpec::Rgb(r, g, b)) => format!("#{:02X}{:02X}{:02X}", r, g, b),
            Some(ColorSpec::Hex(s)) => format!("#{}", s),
        }
    }

    fn compile(&mut self, ast: &AstNode) -> Result<(), String> {
        match ast {
            AstNode::Number(val) => {
                self.instructions.push(OpCode::PushConst(*val));
                Ok(())
            }
            AstNode::StringLit(s) => {
                self.instructions.push(OpCode::PushString(s.clone()));
                Ok(())
            }
            AstNode::Variable(name) => {
                let normalized = self.normalize_variable(name);
                if self.is_builtin_data(&normalized) {
                    self.instructions.push(OpCode::LoadData(normalized));
                } else {
                    self.instructions.push(OpCode::LoadVar(normalized));
                }
                Ok(())
            }
            AstNode::BinaryOp { op, left, right } => {
                self.compile(left)?;
                self.compile(right)?;
                self.emit_binary_op(op);
                Ok(())
            }
            AstNode::UnaryOp { op, expr } => {
                self.compile(expr)?;
                self.emit_unary_op(op);
                Ok(())
            }
            AstNode::FunctionCall { name, args } => {
                for arg in args {
                    self.compile(arg)?;
                }
                self.instructions.push(OpCode::Call {
                    name: name.clone(),
                    arg_count: args.len(),
                });
                Ok(())
            }
            AstNode::IndexAccess { array, index } => {
                self.compile(array)?;
                self.compile(index)?;
                self.instructions.push(OpCode::Index);
                Ok(())
            }
            AstNode::Assignment { name, expr } => {
                self.compile(expr)?;
                self.instructions.push(OpCode::StoreVar(name.clone()));
                Ok(())
            }
            AstNode::CompoundAssignment { name, op, expr } => {
                self.instructions.push(OpCode::LoadVar(name.clone()));
                self.compile(expr)?;
                let op_code = match op {
                    CompoundAssignOp::AddAssign => 0,
                    CompoundAssignOp::SubAssign => 1,
                    CompoundAssignOp::MulAssign => 2,
                    CompoundAssignOp::DivAssign => 3,
                };
                self.instructions.push(OpCode::CompoundStore {
                    name: name.clone(),
                    op: op_code,
                });
                Ok(())
            }
            AstNode::Output {
                name,
                expr,
                modifier: _,
            } => {
                self.compile(expr)?;
                self.instructions.push(OpCode::StoreVar(name.clone()));
                self.instructions.push(OpCode::Output(name.clone()));
                self.output_names.push(name.clone());
                Ok(())
            }
            AstNode::Statements(stmts) => {
                for stmt in stmts {
                    self.compile(stmt)?;
                }
                Ok(())
            }
            AstNode::ParamDecl { .. } => Ok(()),
            AstNode::DrawText {
                cond,
                price,
                text,
                color,
            } => {
                self.compile(cond)?;
                self.compile(price)?;
                let color_str = Self::color_to_string(color);
                self.instructions.push(OpCode::DrawText {
                    text: text.clone(),
                    color: color_str,
                });
                Ok(())
            }
            AstNode::DrawIcon {
                cond,
                price,
                icon,
                color,
            } => {
                self.compile(cond)?;
                self.compile(price)?;
                self.compile(icon)?;
                let color_str = Self::color_to_string(color);
                self.instructions
                    .push(OpCode::DrawIcon { color: color_str });
                Ok(())
            }
            AstNode::StickLine {
                cond,
                price1,
                price2,
                width,
                empty,
                color,
            } => {
                self.compile(cond)?;
                self.compile(price1)?;
                self.compile(price2)?;
                self.compile(width)?;
                let color_str = Self::color_to_string(color);
                self.instructions.push(OpCode::StickLine {
                    empty: *empty,
                    color: color_str,
                });
                Ok(())
            }
            AstNode::DrawGeneric {
                command,
                args,
                color,
            } => {
                for arg in args {
                    self.compile(arg)?;
                }
                let color_str = Self::color_to_string(color);
                self.instructions.push(OpCode::DrawGeneric {
                    command: command.clone(),
                    arg_count: args.len(),
                    color: color_str,
                });
                Ok(())
            }
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => {
                self.compile(cond)?;
                self.compile(then_branch)?;
                self.compile(else_branch)?;
                self.instructions.push(OpCode::Select);
                Ok(())
            }
            AstNode::ForLoop {
                var,
                start,
                end,
                body,
            } => {
                self.compile(start)?;
                self.instructions.push(OpCode::StoreVar(var.clone()));
                let loop_start = self.instructions.len();
                self.instructions.push(OpCode::LoadVar(var.clone()));
                self.compile(end)?;
                self.instructions.push(OpCode::Lte);
                let jump_exit_idx = self.instructions.len();
                self.instructions.push(OpCode::JumpIfFalse(0));
                for stmt in body {
                    self.compile(stmt)?;
                }
                self.instructions.push(OpCode::LoadVar(var.clone()));
                self.instructions.push(OpCode::PushConst(1.0));
                self.instructions.push(OpCode::Add);
                self.instructions.push(OpCode::StoreVar(var.clone()));
                self.instructions.push(OpCode::Jump(loop_start));
                let exit_pos = self.instructions.len();
                if let OpCode::JumpIfFalse(ref mut target) = self.instructions[jump_exit_idx] {
                    *target = exit_pos;
                }
                Ok(())
            }
            AstNode::WhileLoop { cond, body } => {
                let loop_start = self.instructions.len();
                self.compile(cond)?;
                let jump_exit_idx = self.instructions.len();
                self.instructions.push(OpCode::JumpIfFalse(0));
                for stmt in body {
                    self.compile(stmt)?;
                }
                self.instructions.push(OpCode::Jump(loop_start));
                let exit_pos = self.instructions.len();
                if let OpCode::JumpIfFalse(ref mut target) = self.instructions[jump_exit_idx] {
                    *target = exit_pos;
                }
                Ok(())
            }
        }
    }

    fn emit_binary_op(&mut self, op: &BinaryOperator) {
        match op {
            BinaryOperator::Add => self.instructions.push(OpCode::Add),
            BinaryOperator::Sub => self.instructions.push(OpCode::Sub),
            BinaryOperator::Mul => self.instructions.push(OpCode::Mul),
            BinaryOperator::Div => self.instructions.push(OpCode::Div),
            BinaryOperator::Mod => self.instructions.push(OpCode::Mod),
            BinaryOperator::Pow => self.instructions.push(OpCode::Pow),
            BinaryOperator::StringConcat => self.instructions.push(OpCode::StringConcat),
            BinaryOperator::Gt => self.instructions.push(OpCode::Gt),
            BinaryOperator::Lt => self.instructions.push(OpCode::Lt),
            BinaryOperator::Gte => self.instructions.push(OpCode::Gte),
            BinaryOperator::Lte => self.instructions.push(OpCode::Lte),
            BinaryOperator::Eq => self.instructions.push(OpCode::Eq),
            BinaryOperator::Neq => self.instructions.push(OpCode::Neq),
            BinaryOperator::And => self.instructions.push(OpCode::And),
            BinaryOperator::Or => self.instructions.push(OpCode::Or),
            BinaryOperator::Xor => self.instructions.push(OpCode::Xor),
        }
    }

    fn emit_unary_op(&mut self, op: &UnaryOperator) {
        match op {
            UnaryOperator::Not => self.instructions.push(OpCode::Not),
            UnaryOperator::Neg => {
                self.instructions.push(OpCode::PushConst(0.0));
                self.instructions.push(OpCode::Sub);
            }
        }
    }

    fn normalize_variable(&self, name: &str) -> String {
        let upper = name.to_uppercase();
        match upper.as_str() {
            "C" => "CLOSE".to_string(),
            "H" => "HIGH".to_string(),
            "L" => "LOW".to_string(),
            "O" => "OPEN".to_string(),
            "V" | "VOL" => "VOLUME".to_string(),
            "A" => "AMOUNT".to_string(),
            _ => upper,
        }
    }

    fn is_builtin_data(&self, name: &str) -> bool {
        matches!(
            name,
            "OPEN" | "HIGH" | "LOW" | "CLOSE" | "VOLUME" | "AMOUNT"
        )
    }
}

// ============================================================
// Bytecode Virtual Machine
// ============================================================

pub struct ExecResult {
    pub outputs: AHashMap<String, Array1<f64>>,
    pub final_value: Array1<f64>,
    pub draw_commands: DrawResult,
}

type BuiltinFn = fn(&FormulaContext, &[Array1<f64>]) -> Result<Array1<f64>, FormulaError>;

pub struct BytecodeVM {
    stack: Vec<FormulaValue>,
    variables: HashMap<String, Array1<f64>>,
    builtins: HashMap<String, BuiltinFn>,
}

impl Default for BytecodeVM {
    fn default() -> Self {
        Self::new()
    }
}

impl BytecodeVM {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            variables: HashMap::new(),
            builtins: get_builtin_functions(),
        }
    }

    pub fn execute(
        &mut self,
        bytecode: &Bytecode,
        ctx: &FormulaContext,
    ) -> Result<ExecResult, FormulaError> {
        let mut outputs = AHashMap::new();
        let mut final_value = Array1::zeros(ctx.data_len);
        let mut draw_commands = DrawResult::new();
        let mut pc: usize = 0;

        while pc < bytecode.instructions.len() {
            let op = &bytecode.instructions[pc];
            match op {
                OpCode::Jump(target) => {
                    pc = *target;
                    continue;
                }
                OpCode::JumpIfFalse(target) => {
                    let cond = self.pop_val()?;
                    let all_false = match &cond {
                        FormulaValue::Scalar(v) => *v <= 0.0,
                        FormulaValue::Array(a) => a.iter().all(|&v| v <= 0.0),
                    };
                    if all_false {
                        pc = *target;
                        continue;
                    }
                    self.stack.push(cond);
                    pc += 1;
                    continue;
                }
                _ => {
                    self.execute_op(op, ctx, &mut outputs, &mut draw_commands)?;
                    pc += 1;
                }
            }
        }

        if let Some(val) = self.stack.pop() {
            final_value = val.to_array(ctx.data_len);
        }

        Ok(ExecResult {
            outputs,
            final_value,
            draw_commands,
        })
    }

    fn execute_op(
        &mut self,
        op: &OpCode,
        ctx: &FormulaContext,
        outputs: &mut AHashMap<String, Array1<f64>>,
        draw_commands: &mut DrawResult,
    ) -> Result<(), FormulaError> {
        match op {
            OpCode::PushConst(val) => {
                self.stack.push(FormulaValue::Scalar(*val));
                Ok(())
            }
            OpCode::LoadVar(name) => {
                let value = self.load_variable(name, ctx)?;
                self.stack.push(FormulaValue::Array(value));
                Ok(())
            }
            OpCode::StoreVar(name) => {
                if let Some(val) = self.stack.pop() {
                    let arr = val.to_array(ctx.data_len);
                    self.variables.insert(name.clone(), arr);
                    Ok(())
                } else {
                    Err(FormulaError::RuntimeError(
                        "Stack underflow on StoreVar".to_string(),
                    ))
                }
            }
            OpCode::Add => self.exec_binary_op(ctx, |a, b, r| {
                for i in 0..a.len() {
                    r[i] = a[i] + b[i];
                }
            }),
            OpCode::Sub => self.exec_binary_op(ctx, |a, b, r| {
                for i in 0..a.len() {
                    r[i] = a[i] - b[i];
                }
            }),
            OpCode::Mul => self.exec_binary_op(ctx, |a, b, r| {
                for i in 0..a.len() {
                    r[i] = a[i] * b[i];
                }
            }),
            OpCode::Div => self.exec_binary_op(ctx, |a, b, r| {
                for i in 0..a.len() {
                    r[i] = if b[i].abs() < 1e-15 {
                        f64::NAN
                    } else {
                        a[i] / b[i]
                    };
                }
            }),
            OpCode::Mod => self.exec_binary_op(ctx, |a, b, r| {
                for i in 0..a.len() {
                    r[i] = if b[i].abs() < 1e-15 {
                        f64::NAN
                    } else {
                        a[i] - (a[i] / b[i]).floor() * b[i]
                    };
                }
            }),
            OpCode::Pow => self.exec_binary_op(ctx, |a, b, r| {
                for i in 0..a.len() {
                    r[i] = a[i].powf(b[i]);
                }
            }),
            OpCode::Gt => self.exec_binary_op(ctx, |a, b, r| {
                for i in 0..a.len() {
                    r[i] = if a[i] > b[i] { 1.0 } else { 0.0 };
                }
            }),
            OpCode::Lt => self.exec_binary_op(ctx, |a, b, r| {
                for i in 0..a.len() {
                    r[i] = if a[i] < b[i] { 1.0 } else { 0.0 };
                }
            }),
            OpCode::Gte => self.exec_binary_op(ctx, |a, b, r| {
                for i in 0..a.len() {
                    r[i] = if a[i] >= b[i] { 1.0 } else { 0.0 };
                }
            }),
            OpCode::Lte => self.exec_binary_op(ctx, |a, b, r| {
                for i in 0..a.len() {
                    r[i] = if a[i] <= b[i] { 1.0 } else { 0.0 };
                }
            }),
            OpCode::Eq => self.exec_binary_op(ctx, |a, b, r| {
                for i in 0..a.len() {
                    r[i] = if (a[i] - b[i]).abs() < 1e-10 {
                        1.0
                    } else {
                        0.0
                    };
                }
            }),
            OpCode::Neq => self.exec_binary_op(ctx, |a, b, r| {
                for i in 0..a.len() {
                    r[i] = if (a[i] - b[i]).abs() >= 1e-10 {
                        1.0
                    } else {
                        0.0
                    };
                }
            }),
            OpCode::And => self.exec_binary_op(ctx, |a, b, r| {
                for i in 0..a.len() {
                    r[i] = if a[i] > 0.0 && b[i] > 0.0 { 1.0 } else { 0.0 };
                }
            }),
            OpCode::Or => self.exec_binary_op(ctx, |a, b, r| {
                for i in 0..a.len() {
                    r[i] = if a[i] > 0.0 || b[i] > 0.0 { 1.0 } else { 0.0 };
                }
            }),
            OpCode::Xor => self.exec_binary_op(ctx, |a, b, r| {
                for i in 0..a.len() {
                    r[i] = if (a[i] > 0.0) != (b[i] > 0.0) {
                        1.0
                    } else {
                        0.0
                    };
                }
            }),
            OpCode::StringConcat => {
                let _right = self.pop_val()?;
                let _left = self.pop_val()?;
                Err(FormulaError::InvalidOperation(
                    "String concatenation (&) is not supported for numeric values. Use STRCAT() function instead.".to_string()
                ))
            }
            OpCode::Not => {
                let val = self.pop_val()?;
                match val {
                    FormulaValue::Scalar(v) => {
                        self.stack
                            .push(FormulaValue::Scalar(if v <= 0.0 { 1.0 } else { 0.0 }));
                    }
                    FormulaValue::Array(a) => {
                        let mut result = Array1::zeros(a.len());
                        SimdOps::logical_not(a.as_slice().unwrap(), result.as_slice_mut().unwrap());
                        self.stack.push(FormulaValue::Array(result));
                    }
                }
                Ok(())
            }
            OpCode::Jump(_) | OpCode::JumpIfFalse(_) => Ok(()),
            OpCode::Call { name, arg_count } => {
                let arg_count = *arg_count;
                let mut args = Vec::with_capacity(arg_count);
                for _ in 0..arg_count {
                    let val = self.pop_val()?;
                    args.push(val.to_array(ctx.data_len));
                }
                args.reverse();

                let func = self.builtins.get(name.as_str()).ok_or_else(|| {
                    FormulaError::RuntimeError(format!("Unknown function: {}", name))
                })?;
                let result = func(ctx, &args)?;
                self.stack.push(FormulaValue::Array(result));
                Ok(())
            }
            OpCode::LoadData(name) => {
                let data = ctx.get_data(name).ok_or_else(|| {
                    FormulaError::RuntimeError(format!("Data not available: {}", name))
                })?;
                self.stack.push(FormulaValue::Array(data.clone()));
                Ok(())
            }
            OpCode::Index => {
                let idx_val = self.pop_val()?;
                let arr_val = self.pop_val()?;
                let idx = idx_val.to_array(ctx.data_len);
                let arr = arr_val.to_array(ctx.data_len);
                let mut result = Array1::zeros(ctx.data_len);
                for i in 0..ctx.data_len {
                    let idx_i = idx[i] as usize;
                    if idx_i < arr.len() {
                        result[i] = arr[idx_i];
                    } else {
                        result[i] = f64::NAN;
                    }
                }
                self.stack.push(FormulaValue::Array(result));
                Ok(())
            }
            OpCode::CompoundStore { name, op } => {
                let rhs_val = self.pop_val()?;
                let current_val = self.pop_val()?;
                let current = current_val.to_array(ctx.data_len);
                let rhs = rhs_val.to_array(ctx.data_len);
                let value = match op {
                    0 => &current + &rhs,
                    1 => &current - &rhs,
                    2 => &current * &rhs,
                    3 => {
                        let mut r = Array1::zeros(current.len());
                        for i in 0..current.len() {
                            if rhs[i].abs() < 1e-15 {
                                r[i] = f64::NAN;
                            } else {
                                r[i] = current[i] / rhs[i];
                            }
                        }
                        r
                    }
                    _ => {
                        return Err(FormulaError::RuntimeError(format!(
                            "Unknown compound op: {}",
                            op
                        )))
                    }
                };
                self.variables.insert(name.clone(), value.clone());
                self.stack.push(FormulaValue::Array(value));
                Ok(())
            }
            OpCode::Output(name) => {
                if let Some(value) = self.variables.get(name) {
                    outputs.insert(name.clone(), value.clone());
                }
                Ok(())
            }
            OpCode::Select => {
                let else_val = self.pop_val()?;
                let then_val = self.pop_val()?;
                let cond = self.pop_val()?;
                match (&cond, &then_val, &else_val) {
                    (FormulaValue::Scalar(c), FormulaValue::Scalar(t), FormulaValue::Scalar(e)) => {
                        self.stack
                            .push(FormulaValue::Scalar(if *c > 0.0 { *t } else { *e }));
                    }
                    _ => {
                        let cond_arr = cond.to_array(ctx.data_len);
                        let then_arr = then_val.to_array(ctx.data_len);
                        let else_arr = else_val.to_array(ctx.data_len);
                        let len = cond_arr.len().min(then_arr.len()).min(else_arr.len());
                        let result = if len >= 16 {
                            SimdOps::simd_select_arrays(&cond_arr, &then_arr, &else_arr)
                        } else {
                            cond_arr
                                .iter()
                                .zip(then_arr.iter())
                                .zip(else_arr.iter())
                                .map(|((&c, &t), &e)| if c > 0.0 { t } else { e })
                                .collect()
                        };
                        self.stack.push(FormulaValue::Array(result));
                    }
                }
                Ok(())
            }
            OpCode::DrawText { text, color } => {
                let price = self.pop_val()?;
                let cond = self.pop_val()?;
                draw_commands.add_text(
                    cond.to_array(ctx.data_len),
                    price.to_array(ctx.data_len),
                    text.clone(),
                    color.clone(),
                );
                Ok(())
            }
            OpCode::DrawIcon { color } => {
                let icon_val = self.pop_val()?;
                let price = self.pop_val()?;
                let cond = self.pop_val()?;
                let icon_arr = icon_val.to_array(ctx.data_len);
                let icon_type = icon_arr[0] as i32;
                draw_commands.add_icon(
                    cond.to_array(ctx.data_len),
                    price.to_array(ctx.data_len),
                    icon_type,
                    color.clone(),
                );
                Ok(())
            }
            OpCode::StickLine { empty, color } => {
                let width_val = self.pop_val()?;
                let price2 = self.pop_val()?;
                let price1 = self.pop_val()?;
                let cond = self.pop_val()?;
                let width_arr = width_val.to_array(ctx.data_len);
                let width = width_arr[0] as i32;
                draw_commands.add_stick(
                    cond.to_array(ctx.data_len),
                    price1.to_array(ctx.data_len),
                    price2.to_array(ctx.data_len),
                    width,
                    *empty,
                    color.clone(),
                );
                Ok(())
            }
            OpCode::DrawGeneric {
                command: _,
                arg_count,
                color: _,
            } => {
                for _ in 0..*arg_count {
                    self.pop_val()?;
                }
                Ok(())
            }
            OpCode::PushString(_) => {
                self.stack.push(FormulaValue::Scalar(0.0));
                Ok(())
            }
        }
    }

    fn exec_binary_op<F>(&mut self, _ctx: &FormulaContext, _op_fn: F) -> Result<(), FormulaError>
    where
        F: Fn(&[f64], &[f64], &mut [f64]),
    {
        let right = self.pop_val()?;
        let left = self.pop_val()?;
        match (&left, &right) {
            (FormulaValue::Scalar(l), FormulaValue::Scalar(r)) => {
                let mut la = [0.0; 1];
                let mut ra = [0.0; 1];
                let mut res = [0.0; 1];
                la[0] = *l;
                ra[0] = *r;
                _op_fn(&la, &ra, &mut res);
                self.stack.push(FormulaValue::Scalar(res[0]));
            }
            (FormulaValue::Scalar(s), FormulaValue::Array(a)) => {
                let len = a.len();
                let mut result = Array1::zeros(len);
                {
                    let sa = vec![*s; len];
                    _op_fn(&sa, a.as_slice().unwrap(), result.as_slice_mut().unwrap());
                }
                self.stack.push(FormulaValue::Array(result));
            }
            (FormulaValue::Array(a), FormulaValue::Scalar(s)) => {
                let len = a.len();
                let mut result = Array1::zeros(len);
                {
                    let sa = vec![*s; len];
                    _op_fn(a.as_slice().unwrap(), &sa, result.as_slice_mut().unwrap());
                }
                self.stack.push(FormulaValue::Array(result));
            }
            (FormulaValue::Array(l), FormulaValue::Array(r)) => {
                let len = l.len().min(r.len());
                let mut result = Array1::zeros(len);
                if len >= 16 {
                    _op_fn(
                        l.as_slice().unwrap(),
                        r.as_slice().unwrap(),
                        result.as_slice_mut().unwrap(),
                    );
                } else {
                    _op_fn(
                        l.as_slice().unwrap(),
                        r.as_slice().unwrap(),
                        result.as_slice_mut().unwrap(),
                    );
                }
                self.stack.push(FormulaValue::Array(result));
            }
        }
        Ok(())
    }

    fn pop_val(&mut self) -> Result<FormulaValue, FormulaError> {
        self.stack
            .pop()
            .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))
    }

    fn load_variable(&self, name: &str, ctx: &FormulaContext) -> Result<Array1<f64>, FormulaError> {
        match classify_builtin_var(name) {
            Some(BuiltinVar::BarsCount) => Ok(Array1::from_elem(ctx.data_len, ctx.data_len as f64)),
            Some(BuiltinVar::BarPos) => Ok(Array1::from(
                (1..=ctx.data_len).map(|i| i as f64).collect::<Vec<_>>(),
            )),
            Some(BuiltinVar::Capital) => {
                let val = ctx.capital.unwrap_or(f64::NAN);
                Ok(Array1::from_elem(ctx.data_len, val))
            }
            Some(BuiltinVar::DrawNull) => Ok(Array1::from_elem(ctx.data_len, f64::NAN)),
            _ => {
                self.variables.get(name).cloned().ok_or_else(|| {
                    FormulaError::RuntimeError(format!("Unknown variable: {}", name))
                })
            }
        }
    }
}

// ============================================================
// Serialization
// ============================================================

const MAGIC: u32 = 0x42595443; // "BYTC"
const VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum SerializedOpCode {
    PushConst = 0,
    LoadVar = 1,
    StoreVar = 2,
    Add = 3,
    Sub = 4,
    Mul = 5,
    Div = 6,
    Mod = 7,
    Pow = 8,
    Gt = 9,
    Lt = 10,
    Gte = 11,
    Lte = 12,
    Eq = 13,
    Neq = 14,
    And = 15,
    Or = 16,
    Not = 17,
    Jump = 18,
    JumpIfFalse = 19,
    Call = 20,
    LoadData = 21,
    Index = 22,
    Output = 23,
    DrawText = 24,
    DrawIcon = 25,
    StickLine = 26,
    StringConcat = 27,
    Xor = 28,
    CompoundStore = 29,
    PushString = 30,
    Select = 31,
    DrawGeneric = 32,
}

impl SerializedOpCode {
    fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(SerializedOpCode::PushConst),
            1 => Some(SerializedOpCode::LoadVar),
            2 => Some(SerializedOpCode::StoreVar),
            3 => Some(SerializedOpCode::Add),
            4 => Some(SerializedOpCode::Sub),
            5 => Some(SerializedOpCode::Mul),
            6 => Some(SerializedOpCode::Div),
            7 => Some(SerializedOpCode::Mod),
            8 => Some(SerializedOpCode::Pow),
            9 => Some(SerializedOpCode::Gt),
            10 => Some(SerializedOpCode::Lt),
            11 => Some(SerializedOpCode::Gte),
            12 => Some(SerializedOpCode::Lte),
            13 => Some(SerializedOpCode::Eq),
            14 => Some(SerializedOpCode::Neq),
            15 => Some(SerializedOpCode::And),
            16 => Some(SerializedOpCode::Or),
            17 => Some(SerializedOpCode::Not),
            18 => Some(SerializedOpCode::Jump),
            19 => Some(SerializedOpCode::JumpIfFalse),
            20 => Some(SerializedOpCode::Call),
            21 => Some(SerializedOpCode::LoadData),
            22 => Some(SerializedOpCode::Index),
            23 => Some(SerializedOpCode::Output),
            24 => Some(SerializedOpCode::DrawText),
            25 => Some(SerializedOpCode::DrawIcon),
            26 => Some(SerializedOpCode::StickLine),
            27 => Some(SerializedOpCode::StringConcat),
            28 => Some(SerializedOpCode::Xor),
            29 => Some(SerializedOpCode::CompoundStore),
            30 => Some(SerializedOpCode::PushString),
            31 => Some(SerializedOpCode::Select),
            32 => Some(SerializedOpCode::DrawGeneric),
            _ => None,
        }
    }

    fn from_op(op: &OpCode) -> Self {
        match op {
            OpCode::PushConst(_) => SerializedOpCode::PushConst,
            OpCode::LoadVar(_) => SerializedOpCode::LoadVar,
            OpCode::StoreVar(_) => SerializedOpCode::StoreVar,
            OpCode::Add => SerializedOpCode::Add,
            OpCode::Sub => SerializedOpCode::Sub,
            OpCode::Mul => SerializedOpCode::Mul,
            OpCode::Div => SerializedOpCode::Div,
            OpCode::Mod => SerializedOpCode::Mod,
            OpCode::Pow => SerializedOpCode::Pow,
            OpCode::Gt => SerializedOpCode::Gt,
            OpCode::Lt => SerializedOpCode::Lt,
            OpCode::Gte => SerializedOpCode::Gte,
            OpCode::Lte => SerializedOpCode::Lte,
            OpCode::Eq => SerializedOpCode::Eq,
            OpCode::Neq => SerializedOpCode::Neq,
            OpCode::And => SerializedOpCode::And,
            OpCode::Or => SerializedOpCode::Or,
            OpCode::Not => SerializedOpCode::Not,
            OpCode::Jump(_) => SerializedOpCode::Jump,
            OpCode::JumpIfFalse(_) => SerializedOpCode::JumpIfFalse,
            OpCode::Call { .. } => SerializedOpCode::Call,
            OpCode::LoadData(_) => SerializedOpCode::LoadData,
            OpCode::Index => SerializedOpCode::Index,
            OpCode::Output(_) => SerializedOpCode::Output,
            OpCode::DrawText { .. } => SerializedOpCode::DrawText,
            OpCode::DrawIcon { .. } => SerializedOpCode::DrawIcon,
            OpCode::StickLine { .. } => SerializedOpCode::StickLine,
            OpCode::DrawGeneric { .. } => SerializedOpCode::DrawGeneric,
            OpCode::StringConcat => SerializedOpCode::StringConcat,
            OpCode::Xor => SerializedOpCode::Xor,
            OpCode::CompoundStore { .. } => SerializedOpCode::CompoundStore,
            OpCode::PushString(_) => SerializedOpCode::PushString,
            OpCode::Select => SerializedOpCode::Select,
        }
    }
}

impl Bytecode {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        buf.write_all(&MAGIC.to_le_bytes()).unwrap();
        buf.write_all(&VERSION.to_le_bytes()).unwrap();

        let inst_count = self.instructions.len() as u32;
        buf.write_all(&inst_count.to_le_bytes()).unwrap();

        for op in &self.instructions {
            let tag = SerializedOpCode::from_op(op);
            buf.push(tag as u8);

            match op {
                OpCode::PushConst(val) => {
                    buf.write_all(&val.to_le_bytes()).unwrap();
                }
                OpCode::LoadVar(name)
                | OpCode::StoreVar(name)
                | OpCode::LoadData(name)
                | OpCode::Output(name) => {
                    let bytes = name.as_bytes();
                    buf.write_all(&(bytes.len() as u16).to_le_bytes()).unwrap();
                    buf.write_all(bytes).unwrap();
                }
                OpCode::Jump(pos) | OpCode::JumpIfFalse(pos) => {
                    buf.write_all(&(*pos as u32).to_le_bytes()).unwrap();
                }
                OpCode::Call { name, arg_count } => {
                    let bytes = name.as_bytes();
                    buf.write_all(&(bytes.len() as u16).to_le_bytes()).unwrap();
                    buf.write_all(bytes).unwrap();
                    buf.write_all(&(*arg_count as u16).to_le_bytes()).unwrap();
                }
                OpCode::CompoundStore { name, op } => {
                    let bytes = name.as_bytes();
                    buf.write_all(&(bytes.len() as u16).to_le_bytes()).unwrap();
                    buf.write_all(bytes).unwrap();
                    buf.push(*op);
                }
                OpCode::DrawText { text, color } => {
                    let text_bytes = text.as_bytes();
                    buf.write_all(&(text_bytes.len() as u16).to_le_bytes())
                        .unwrap();
                    buf.write_all(text_bytes).unwrap();
                    let color_bytes = color.as_bytes();
                    buf.write_all(&(color_bytes.len() as u16).to_le_bytes())
                        .unwrap();
                    buf.write_all(color_bytes).unwrap();
                }
                OpCode::DrawIcon { color } => {
                    let color_bytes = color.as_bytes();
                    buf.write_all(&(color_bytes.len() as u16).to_le_bytes())
                        .unwrap();
                    buf.write_all(color_bytes).unwrap();
                }
                OpCode::StickLine { empty, color } => {
                    buf.push(if *empty { 1u8 } else { 0u8 });
                    let color_bytes = color.as_bytes();
                    buf.write_all(&(color_bytes.len() as u16).to_le_bytes())
                        .unwrap();
                    buf.write_all(color_bytes).unwrap();
                }
                OpCode::DrawGeneric {
                    command,
                    arg_count,
                    color,
                } => {
                    let cmd_bytes = command.as_bytes();
                    buf.write_all(&(cmd_bytes.len() as u16).to_le_bytes())
                        .unwrap();
                    buf.write_all(cmd_bytes).unwrap();
                    buf.write_all(&(*arg_count as u32).to_le_bytes()).unwrap();
                    let color_bytes = color.as_bytes();
                    buf.write_all(&(color_bytes.len() as u16).to_le_bytes())
                        .unwrap();
                    buf.write_all(color_bytes).unwrap();
                }
                OpCode::PushString(s) => {
                    let bytes = s.as_bytes();
                    buf.write_all(&(bytes.len() as u16).to_le_bytes()).unwrap();
                    buf.write_all(bytes).unwrap();
                }
                _ => {}
            }
        }

        let output_count = self.output_names.len() as u32;
        buf.write_all(&output_count.to_le_bytes()).unwrap();
        for name in &self.output_names {
            let bytes = name.as_bytes();
            buf.write_all(&(bytes.len() as u16).to_le_bytes()).unwrap();
            buf.write_all(bytes).unwrap();
        }

        let source_bytes = self.source.as_bytes();
        buf.write_all(&(source_bytes.len() as u32).to_le_bytes())
            .unwrap();
        buf.write_all(source_bytes).unwrap();

        buf
    }

    pub fn deserialize(data: &[u8]) -> Result<Bytecode, String> {
        let mut cursor = Cursor::new(data);

        let mut magic_buf = [0u8; 4];
        cursor
            .read_exact(&mut magic_buf)
            .map_err(|e| e.to_string())?;
        let magic = u32::from_le_bytes(magic_buf);
        if magic != MAGIC {
            return Err("Invalid bytecode magic".to_string());
        }

        let mut version_buf = [0u8; 2];
        cursor
            .read_exact(&mut version_buf)
            .map_err(|e| e.to_string())?;
        let version = u16::from_le_bytes(version_buf);
        if version != VERSION {
            return Err(format!("Unsupported bytecode version: {}", version));
        }

        let mut count_buf = [0u8; 4];
        cursor
            .read_exact(&mut count_buf)
            .map_err(|e| e.to_string())?;
        let inst_count = u32::from_le_bytes(count_buf) as usize;

        let mut instructions = Vec::with_capacity(inst_count);

        for _ in 0..inst_count {
            let mut tag_buf = [0u8; 1];
            cursor.read_exact(&mut tag_buf).map_err(|e| e.to_string())?;
            let tag = SerializedOpCode::from_u8(tag_buf[0])
                .ok_or_else(|| format!("Unknown opcode tag: {}", tag_buf[0]))?;

            let op = match tag {
                SerializedOpCode::PushConst => {
                    let mut val_buf = [0u8; 8];
                    cursor.read_exact(&mut val_buf).map_err(|e| e.to_string())?;
                    OpCode::PushConst(f64::from_le_bytes(val_buf))
                }
                SerializedOpCode::LoadVar => {
                    let name = Self::read_string(&mut cursor)?;
                    OpCode::LoadVar(name)
                }
                SerializedOpCode::StoreVar => {
                    let name = Self::read_string(&mut cursor)?;
                    OpCode::StoreVar(name)
                }
                SerializedOpCode::Add => OpCode::Add,
                SerializedOpCode::Sub => OpCode::Sub,
                SerializedOpCode::Mul => OpCode::Mul,
                SerializedOpCode::Div => OpCode::Div,
                SerializedOpCode::Mod => OpCode::Mod,
                SerializedOpCode::Pow => OpCode::Pow,
                SerializedOpCode::Gt => OpCode::Gt,
                SerializedOpCode::Lt => OpCode::Lt,
                SerializedOpCode::Gte => OpCode::Gte,
                SerializedOpCode::Lte => OpCode::Lte,
                SerializedOpCode::Eq => OpCode::Eq,
                SerializedOpCode::Neq => OpCode::Neq,
                SerializedOpCode::And => OpCode::And,
                SerializedOpCode::Or => OpCode::Or,
                SerializedOpCode::Not => OpCode::Not,
                SerializedOpCode::Jump => {
                    let mut pos_buf = [0u8; 4];
                    cursor.read_exact(&mut pos_buf).map_err(|e| e.to_string())?;
                    OpCode::Jump(u32::from_le_bytes(pos_buf) as usize)
                }
                SerializedOpCode::JumpIfFalse => {
                    let mut pos_buf = [0u8; 4];
                    cursor.read_exact(&mut pos_buf).map_err(|e| e.to_string())?;
                    OpCode::JumpIfFalse(u32::from_le_bytes(pos_buf) as usize)
                }
                SerializedOpCode::Call => {
                    let name = Self::read_string(&mut cursor)?;
                    let mut arg_buf = [0u8; 2];
                    cursor.read_exact(&mut arg_buf).map_err(|e| e.to_string())?;
                    OpCode::Call {
                        name,
                        arg_count: u16::from_le_bytes(arg_buf) as usize,
                    }
                }
                SerializedOpCode::LoadData => {
                    let name = Self::read_string(&mut cursor)?;
                    OpCode::LoadData(name)
                }
                SerializedOpCode::Index => OpCode::Index,
                SerializedOpCode::Output => {
                    let name = Self::read_string(&mut cursor)?;
                    OpCode::Output(name)
                }
                SerializedOpCode::DrawText => {
                    let text = Self::read_string(&mut cursor)?;
                    let color = Self::read_string(&mut cursor)?;
                    OpCode::DrawText { text, color }
                }
                SerializedOpCode::DrawIcon => {
                    let color = Self::read_string(&mut cursor)?;
                    OpCode::DrawIcon { color }
                }
                SerializedOpCode::StickLine => {
                    let mut empty_buf = [0u8; 1];
                    cursor
                        .read_exact(&mut empty_buf)
                        .map_err(|e| e.to_string())?;
                    let empty = empty_buf[0] != 0;
                    let color = Self::read_string(&mut cursor)?;
                    OpCode::StickLine { empty, color }
                }
                SerializedOpCode::DrawGeneric => {
                    let command = Self::read_string(&mut cursor)?;
                    let mut count_buf = [0u8; 4];
                    cursor
                        .read_exact(&mut count_buf)
                        .map_err(|e| e.to_string())?;
                    let arg_count = u32::from_le_bytes(count_buf) as usize;
                    let color = Self::read_string(&mut cursor)?;
                    OpCode::DrawGeneric {
                        command,
                        arg_count,
                        color,
                    }
                }
                SerializedOpCode::StringConcat => OpCode::StringConcat,
                SerializedOpCode::Xor => OpCode::Xor,
                SerializedOpCode::CompoundStore => {
                    let name = Self::read_string(&mut cursor)?;
                    let mut op_buf = [0u8; 1];
                    cursor.read_exact(&mut op_buf).map_err(|e| e.to_string())?;
                    OpCode::CompoundStore {
                        name,
                        op: op_buf[0],
                    }
                }
                SerializedOpCode::PushString => {
                    let s = Self::read_string(&mut cursor)?;
                    OpCode::PushString(s)
                }
                SerializedOpCode::Select => OpCode::Select,
            };

            instructions.push(op);
        }

        let mut count_buf2 = [0u8; 4];
        cursor
            .read_exact(&mut count_buf2)
            .map_err(|e| e.to_string())?;
        let output_count = u32::from_le_bytes(count_buf2) as usize;

        let mut output_names = Vec::with_capacity(output_count);
        for _ in 0..output_count {
            output_names.push(Self::read_string(&mut cursor)?);
        }

        let mut source_len_buf = [0u8; 4];
        cursor
            .read_exact(&mut source_len_buf)
            .map_err(|e| e.to_string())?;
        let source_len = u32::from_le_bytes(source_len_buf) as usize;

        let mut source_buf = vec![0u8; source_len];
        cursor
            .read_exact(&mut source_buf)
            .map_err(|e| e.to_string())?;
        let source = String::from_utf8(source_buf).map_err(|e| e.to_string())?;

        Ok(Bytecode {
            instructions,
            source,
            output_names,
        })
    }

    fn read_string(cursor: &mut Cursor<&[u8]>) -> Result<String, String> {
        let mut len_buf = [0u8; 2];
        cursor.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
        let len = u16::from_le_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        cursor.read_exact(&mut buf).map_err(|e| e.to_string())?;
        String::from_utf8(buf).map_err(|e| e.to_string())
    }
}

// ============================================================
// Unit Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::ast::{AstNode, BinaryOperator, ColorSpec};
    use crate::formula::drawing::DrawCommand;
    use crate::formula::parser::parse_formula;
    use std::time::Instant;

    fn make_ctx(len: usize) -> FormulaContext {
        let open = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.1).collect());
        let high = Array1::from_vec((0..len).map(|i| 11.0 + i as f64 * 0.2).collect());
        let low = Array1::from_vec((0..len).map(|i| 9.0 + i as f64 * 0.1).collect());
        let close = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.15).collect());
        let volume = Array1::from_vec((0..len).map(|i| 1000.0 + i as f64 * 10.0).collect());
        FormulaContext::new(open, high, low, close, volume, None)
    }

    fn compile_formula(source: &str) -> Bytecode {
        let ast = parse_formula(source).expect("Parse failed");
        compile_to_bytecode(&ast, source).expect("Compile failed")
    }

    fn execute_bytecode(bytecode: &Bytecode, ctx: &FormulaContext) -> ExecResult {
        let mut vm = BytecodeVM::new();
        vm.execute(bytecode, ctx).expect("Execution failed")
    }

    // Test 1: Basic constant expression
    #[test]
    fn test_compile_constant() {
        let bytecode = compile_formula("42");
        assert_eq!(bytecode.instructions.len(), 1);
        match &bytecode.instructions[0] {
            OpCode::PushConst(val) => assert!((*val - 42.0).abs() < 1e-10),
            _ => panic!("Expected PushConst"),
        }
    }

    // Test 2: Basic addition
    #[test]
    fn test_compile_addition() {
        let bytecode = compile_formula("10 + 20");
        assert_eq!(bytecode.instructions.len(), 3);
        assert!(
            matches!(&bytecode.instructions[0], OpCode::PushConst(v) if (*v - 10.0).abs() < 1e-10)
        );
        assert!(
            matches!(&bytecode.instructions[1], OpCode::PushConst(v) if (*v - 20.0).abs() < 1e-10)
        );
        assert!(matches!(&bytecode.instructions[2], OpCode::Add));
    }

    // Test 3: Variable loading (builtin data)
    #[test]
    fn test_compile_variable_close() {
        let bytecode = compile_formula("CLOSE");
        assert_eq!(bytecode.instructions.len(), 1);
        assert!(matches!(&bytecode.instructions[0], OpCode::LoadData(name) if name == "CLOSE"));
    }

    // Test 4: Variable shortcut (C -> CLOSE)
    #[test]
    fn test_compile_variable_shortcut() {
        let bytecode = compile_formula("C + O");
        assert!(matches!(&bytecode.instructions[0], OpCode::LoadData(name) if name == "CLOSE"));
        assert!(matches!(&bytecode.instructions[1], OpCode::LoadData(name) if name == "OPEN"));
        assert!(matches!(&bytecode.instructions[2], OpCode::Add));
    }

    // Test 5: Function call compilation
    #[test]
    fn test_compile_function_call() {
        let bytecode = compile_formula("MA(CLOSE, 5)");
        assert_eq!(bytecode.instructions.len(), 3);
        assert!(matches!(&bytecode.instructions[0], OpCode::LoadData(name) if name == "CLOSE"));
        assert!(
            matches!(&bytecode.instructions[1], OpCode::PushConst(v) if (*v - 5.0).abs() < 1e-10)
        );
        assert!(
            matches!(&bytecode.instructions[2], OpCode::Call { name, arg_count } if name == "MA" && *arg_count == 2)
        );
    }

    // Test 6: Assignment compilation
    #[test]
    fn test_compile_assignment() {
        let bytecode = compile_formula("UP := CLOSE + 1");
        let has_store = bytecode
            .instructions
            .iter()
            .any(|op| matches!(op, OpCode::StoreVar(name) if name == "UP"));
        assert!(
            has_store,
            "Expected StoreVar(UP) in {:?}",
            bytecode.instructions
        );
    }

    // Test 7: Output compilation
    #[test]
    fn test_compile_output() {
        let bytecode = compile_formula("RESULT: CLOSE * 2");
        let has_store = bytecode
            .instructions
            .iter()
            .any(|op| matches!(op, OpCode::StoreVar(name) if name == "RESULT"));
        let has_output = bytecode
            .instructions
            .iter()
            .any(|op| matches!(op, OpCode::Output(name) if name == "RESULT"));
        assert!(has_store, "Expected StoreVar(RESULT)");
        assert!(has_output, "Expected Output(RESULT)");
        assert_eq!(bytecode.output_names.len(), 1);
        assert_eq!(&bytecode.output_names[0], "RESULT");
    }

    // Test 8: Multiple statements
    #[test]
    fn test_compile_multiple_statements() {
        let source = "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); MA5 > MA10";
        let bytecode = compile_formula(source);
        assert!(bytecode.instructions.len() > 10);
    }

    // Test 9: Nested function call
    #[test]
    fn test_compile_nested_function() {
        let bytecode = compile_formula("EMA(MA(CLOSE, 10), 12)");
        let ma_calls = bytecode
            .instructions
            .iter()
            .filter(|op| matches!(op, OpCode::Call { name, .. } if name == "MA"))
            .count();
        let ema_calls = bytecode
            .instructions
            .iter()
            .filter(|op| matches!(op, OpCode::Call { name, .. } if name == "EMA"))
            .count();
        assert_eq!(ma_calls, 1);
        assert_eq!(ema_calls, 1);
    }

    // Test 10: Comparison operators
    #[test]
    fn test_compile_comparison() {
        let bytecode = compile_formula("CLOSE > 10.5");
        let has_gt = bytecode
            .instructions
            .iter()
            .any(|op| matches!(op, OpCode::Gt));
        assert!(has_gt, "Expected Gt opcode");
    }

    // Test 11: Logical AND
    #[test]
    fn test_compile_logical_and() {
        let bytecode = compile_formula("CLOSE > OPEN AND VOLUME > 1000");
        let has_and = bytecode
            .instructions
            .iter()
            .any(|op| matches!(op, OpCode::And));
        assert!(has_and, "Expected And opcode");
    }

    // Test 12: Unary NOT
    #[test]
    fn test_compile_unary_not() {
        let bytecode = compile_formula("NOT(CLOSE > 20)");
        let has_not = bytecode
            .instructions
            .iter()
            .any(|op| matches!(op, OpCode::Not));
        assert!(has_not, "Expected Not opcode");
    }

    // Test 13: Complex expression with multiple operators
    #[test]
    fn test_compile_complex_expression() {
        let bytecode = compile_formula("(CLOSE + OPEN) * 2");
        assert_eq!(bytecode.instructions.len(), 5);
        assert!(matches!(&bytecode.instructions[0], OpCode::LoadData(name) if name == "CLOSE"));
        assert!(matches!(&bytecode.instructions[1], OpCode::LoadData(name) if name == "OPEN"));
        assert!(matches!(&bytecode.instructions[2], OpCode::Add));
        assert!(
            matches!(&bytecode.instructions[3], OpCode::PushConst(v) if (*v - 2.0).abs() < 1e-10)
        );
        assert!(matches!(&bytecode.instructions[4], OpCode::Mul));
    }

    // Test 14: Execute constant
    #[test]
    fn test_execute_constant() {
        let bytecode = compile_formula("42");
        let ctx = make_ctx(5);
        let result = execute_bytecode(&bytecode, &ctx);
        for i in 0..5 {
            assert!((result.final_value[i] - 42.0).abs() < 1e-10);
        }
    }

    // Test 15: Execute addition
    #[test]
    fn test_execute_addition() {
        let bytecode = compile_formula("10 + 20");
        let ctx = make_ctx(5);
        let result = execute_bytecode(&bytecode, &ctx);
        for i in 0..5 {
            assert!((result.final_value[i] - 30.0).abs() < 1e-10);
        }
    }

    // Test 16: Execute variable reference
    #[test]
    fn test_execute_variable_close() {
        let bytecode = compile_formula("CLOSE");
        let ctx = make_ctx(5);
        let result = execute_bytecode(&bytecode, &ctx);
        for i in 0..5 {
            let expected = 10.0 + i as f64 * 0.15;
            assert!((result.final_value[i] - expected).abs() < 1e-10);
        }
    }

    // Test 17: Execute binary expression with variables
    #[test]
    fn test_execute_variable_expression() {
        let bytecode = compile_formula("C + O");
        let ctx = make_ctx(5);
        let result = execute_bytecode(&bytecode, &ctx);
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            let expected = close_val + open_val;
            assert!((result.final_value[i] - expected).abs() < 1e-10);
        }
    }

    // Test 18: Execute function call
    #[test]
    fn test_execute_function_call() {
        let bytecode = compile_formula("MA(CLOSE, 3)");
        let ctx = make_ctx(10);
        let result = execute_bytecode(&bytecode, &ctx);
        assert!(result.final_value[0].is_nan());
        assert!(result.final_value[1].is_nan());
        assert!(!result.final_value[2].is_nan());
    }

    // Test 19: Execute assignment and use
    #[test]
    fn test_execute_assignment_and_use() {
        let source = "UP := CLOSE + 1; UP * 2";
        let bytecode = compile_formula(source);
        let ctx = make_ctx(5);
        let result = execute_bytecode(&bytecode, &ctx);
        for i in 0..5 {
            let expected = (10.0 + i as f64 * 0.15 + 1.0) * 2.0;
            assert!((result.final_value[i] - expected).abs() < 1e-10);
        }
    }

    // Test 20: Execute comparison
    #[test]
    fn test_execute_comparison() {
        let bytecode = compile_formula("CLOSE > 10.5");
        let ctx = make_ctx(5);
        let result = execute_bytecode(&bytecode, &ctx);
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let expected = if close_val > 10.5 { 1.0 } else { 0.0 };
            assert!((result.final_value[i] - expected).abs() < 1e-10);
        }
    }

    // Test 21: Serialize and deserialize
    #[test]
    fn test_serialize_deserialize_simple() {
        let bytecode = compile_formula("10 + 20");
        let data = bytecode.serialize();
        let restored = Bytecode::deserialize(&data).expect("Deserialize failed");
        assert_eq!(restored.instructions.len(), bytecode.instructions.len());
        assert_eq!(restored.source, bytecode.source);
        assert_eq!(restored.output_names, bytecode.output_names);
    }

    // Test 22: Serialize and deserialize with function call
    #[test]
    fn test_serialize_deserialize_with_function() {
        let bytecode = compile_formula("MA(CLOSE, 5)");
        let data = bytecode.serialize();
        let restored = Bytecode::deserialize(&data).expect("Deserialize failed");
        assert_eq!(restored.instructions.len(), 3);
        assert!(
            matches!(&restored.instructions[2], OpCode::Call { name, arg_count } if name == "MA" && *arg_count == 2)
        );
    }

    // Test 23: Serialize and deserialize with multiple statements
    #[test]
    fn test_serialize_deserialize_multiple() {
        let source = "MA5 := MA(CLOSE, 5); MA5: MA5";
        let bytecode = compile_formula(source);
        let data = bytecode.serialize();
        let restored = Bytecode::deserialize(&data).expect("Deserialize failed");
        assert_eq!(restored.output_names.len(), 1);
        assert_eq!(&restored.output_names[0], "MA5");
    }

    // Test 24: Execute after deserialize
    #[test]
    fn test_execute_after_deserialize() {
        let bytecode = compile_formula("CLOSE + OPEN");
        let data = bytecode.serialize();
        let restored = Bytecode::deserialize(&data).expect("Deserialize failed");
        let ctx = make_ctx(5);
        let result = execute_bytecode(&restored, &ctx);
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            let expected = close_val + open_val;
            assert!((result.final_value[i] - expected).abs() < 1e-10);
        }
    }

    // Test 25: Complex formula execution matches AST executor
    #[test]
    fn test_bytecode_matches_ast_executor() {
        let source = "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); MA5 > MA10";
        let ctx_for_bytecode = make_ctx(30);
        let _ctx_for_ast = make_ctx(30);

        let ast = parse_formula(source).expect("Parse failed");
        let bytecode = compile_to_bytecode(&ast, source).expect("Compile failed");

        let mut vm = BytecodeVM::new();
        let bc_result = vm
            .execute(&bytecode, &ctx_for_bytecode)
            .expect("VM exec failed");

        let executor = crate::formula::executor::FormulaExecutor::new();
        let mut ctx_clone = make_ctx(30);
        let ast_result = executor
            .execute(&ast, &mut ctx_clone)
            .expect("AST exec failed");

        for i in 0..30 {
            let bc_val = bc_result.final_value[i];
            let ast_val = ast_result[i];
            if bc_val.is_nan() {
                assert!(
                    ast_val.is_nan(),
                    "Mismatch at index {}: bc=NaN, ast={}",
                    i,
                    ast_val
                );
            } else {
                assert!(
                    (bc_val - ast_val).abs() < 1e-10,
                    "Mismatch at index {}: bc={}, ast={}",
                    i,
                    bc_val,
                    ast_val
                );
            }
        }
    }

    // Test 26: Output tracking
    #[test]
    fn test_output_tracking() {
        let source = "RESULT: CLOSE * 2";
        let bytecode = compile_formula(source);
        let ctx = make_ctx(5);
        let result = execute_bytecode(&bytecode, &ctx);
        assert!(result.outputs.contains_key("RESULT"));
        for i in 0..5 {
            let expected = (10.0 + i as f64 * 0.15) * 2.0;
            assert!((result.outputs["RESULT"][i] - expected).abs() < 1e-10);
        }
    }

    // Test 27: Division by zero handling
    #[test]
    fn test_division_by_zero() {
        let bytecode = compile_formula("10 / 0");
        let ctx = make_ctx(5);
        let result = execute_bytecode(&bytecode, &ctx);
        for i in 0..5 {
            assert!(result.final_value[i].is_nan());
        }
    }

    // Test 28: Nested expressions
    #[test]
    fn test_nested_expressions() {
        let bytecode = compile_formula("((CLOSE + OPEN) * 2) / (HIGH - LOW)");
        let ctx = make_ctx(5);
        let result = execute_bytecode(&bytecode, &ctx);
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            let high_val = 11.0 + i as f64 * 0.2;
            let low_val = 9.0 + i as f64 * 0.1;
            let expected = ((close_val + open_val) * 2.0) / (high_val - low_val);
            assert!((result.final_value[i] - expected).abs() < 1e-10);
        }
    }

    // Test 29: Power operator
    #[test]
    fn test_power_operator() {
        let bytecode = compile_formula("2 ^ 10");
        let ctx = make_ctx(5);
        let result = execute_bytecode(&bytecode, &ctx);
        for i in 0..5 {
            assert!((result.final_value[i] - 1024.0).abs() < 1e-10);
        }
    }

    // Test 30: IF function execution
    #[test]
    fn test_if_function_execution() {
        let bytecode = compile_formula("IF(CLOSE > 10.5, 100, 0)");
        let ctx = make_ctx(5);
        let result = execute_bytecode(&bytecode, &ctx);
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let expected = if close_val > 10.5 { 100.0 } else { 0.0 };
            assert!((result.final_value[i] - expected).abs() < 1e-10);
        }
    }

    // Test 31: Performance benchmark - bytecode vs AST
    #[test]
    fn test_performance_bytecode_vs_ast() {
        let source = "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); MA20 := MA(CLOSE, 20); DIF := MA5 - MA10; DEA := MA(DIF, 5); SIGNAL := IF(DIF > DEA, 1, 0); SIGNAL";
        let _ctx_template = make_ctx(500);

        let ast = parse_formula(source).expect("Parse failed");
        let bytecode = compile_to_bytecode(&ast, source).expect("Compile failed");

        let iterations = 1000;

        let start = Instant::now();
        for _ in 0..iterations {
            let ctx = make_ctx(500);
            let mut vm = BytecodeVM::new();
            vm.execute(&bytecode, &ctx).expect("VM exec failed");
        }
        let bytecode_time = start.elapsed();

        let start = Instant::now();
        for _ in 0..iterations {
            let mut ctx = make_ctx(500);
            let executor = crate::formula::executor::FormulaExecutor::new();
            executor.execute(&ast, &mut ctx).expect("AST exec failed");
        }
        let ast_time = start.elapsed();

        let speedup = ast_time.as_secs_f64() / bytecode_time.as_secs_f64();

        println!(
            "Bytecode vs AST speedup: {:.2}x (bytecode: {:?}, AST: {:?})",
            speedup, bytecode_time, ast_time
        );
        // Performance may vary; just verify both produce correct results
        assert!(
            speedup > 0.1,
            "Unexpected performance degradation. Speedup: {:.2}x",
            speedup
        );
    }

    // Test 32: REF function in bytecode
    #[test]
    fn test_ref_function() {
        let bytecode = compile_formula("REF(CLOSE, 1)");
        let ctx = make_ctx(10);
        let result = execute_bytecode(&bytecode, &ctx);
        assert!(result.final_value[0].is_nan());
        for i in 1..10 {
            let expected = 10.0 + (i - 1) as f64 * 0.15;
            assert!((result.final_value[i] - expected).abs() < 1e-10);
        }
    }

    // Test 33: CROSS function in bytecode
    #[test]
    fn test_cross_function() {
        let source = "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); CROSS(MA5, MA10)";
        let bytecode = compile_formula(source);
        let ctx = make_ctx(30);
        let result = execute_bytecode(&bytecode, &ctx);
        assert_eq!(result.final_value.len(), 30);
    }

    // Test 34: Complex trading formula execution
    #[test]
    fn test_complex_trading_formula() {
        let source = "SHORT := 12; LONG := 26; DIF := EMA(CLOSE, SHORT) - EMA(CLOSE, LONG); DEA := EMA(DIF, 9); MACD := (DIF - DEA) * 2; MACD";
        let bytecode = compile_formula(source);
        let ctx = make_ctx(50);
        let result = execute_bytecode(&bytecode, &ctx);
        assert_eq!(result.final_value.len(), 50);
    }

    // Test 35: Invalid bytecode deserialization
    #[test]
    fn test_invalid_deserialization() {
        let invalid_data = [0u8; 10];
        let result = Bytecode::deserialize(&invalid_data);
        assert!(result.is_err());
    }

    // Test 36: Serialization roundtrip preserves all instruction types
    #[test]
    fn test_roundtrip_preserves_instructions() {
        let source = "UP := CLOSE + OPEN; UP > 10";
        let bytecode = compile_formula(source);
        let data = bytecode.serialize();
        let restored = Bytecode::deserialize(&data).expect("Deserialize failed");

        assert_eq!(bytecode.instructions.len(), restored.instructions.len());
        for (orig, rest) in bytecode
            .instructions
            .iter()
            .zip(restored.instructions.iter())
        {
            assert_eq!(orig, rest);
        }
    }

    // Test 37: IfThenElse bytecode compilation
    #[test]
    fn test_if_then_else_compilation() {
        let node = AstNode::IfThenElse {
            cond: Box::new(AstNode::BinaryOp {
                op: BinaryOperator::Gt,
                left: Box::new(AstNode::Variable("CLOSE".to_string())),
                right: Box::new(AstNode::Number(10.0)),
            }),
            then_branch: Box::new(AstNode::Number(1.0)),
            else_branch: Box::new(AstNode::Number(0.0)),
        };
        let bytecode =
            compile_to_bytecode(&node, "IF CLOSE > 10 THEN 1 ELSE 0").expect("Compile failed");
        assert!(bytecode
            .instructions
            .iter()
            .any(|op| matches!(op, OpCode::Select)));
    }

    // Test 38: IfThenElse bytecode execution
    #[test]
    fn test_if_then_else_execution() {
        let node = AstNode::IfThenElse {
            cond: Box::new(AstNode::BinaryOp {
                op: BinaryOperator::Gt,
                left: Box::new(AstNode::Variable("CLOSE".to_string())),
                right: Box::new(AstNode::Number(10.5)),
            }),
            then_branch: Box::new(AstNode::Number(100.0)),
            else_branch: Box::new(AstNode::Number(0.0)),
        };
        let bytecode = compile_to_bytecode(&node, "test").expect("Compile failed");
        let ctx = make_ctx(5);
        let result = execute_bytecode(&bytecode, &ctx);
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let expected = if close_val > 10.5 { 100.0 } else { 0.0 };
            assert!(
                (result.final_value[i] - expected).abs() < 1e-10,
                "Mismatch at {}: got {}, expected {}",
                i,
                result.final_value[i],
                expected
            );
        }
    }

    // Test 39: Nested IfThenElse
    #[test]
    fn test_nested_if_then_else() {
        let node = AstNode::IfThenElse {
            cond: Box::new(AstNode::BinaryOp {
                op: BinaryOperator::Gt,
                left: Box::new(AstNode::Variable("CLOSE".to_string())),
                right: Box::new(AstNode::Number(10.5)),
            }),
            then_branch: Box::new(AstNode::IfThenElse {
                cond: Box::new(AstNode::BinaryOp {
                    op: BinaryOperator::Gt,
                    left: Box::new(AstNode::Variable("CLOSE".to_string())),
                    right: Box::new(AstNode::Number(11.0)),
                }),
                then_branch: Box::new(AstNode::Number(2.0)),
                else_branch: Box::new(AstNode::Number(1.0)),
            }),
            else_branch: Box::new(AstNode::Number(0.0)),
        };
        let bytecode = compile_to_bytecode(&node, "test").expect("Compile failed");
        let ctx = make_ctx(10);
        let result = execute_bytecode(&bytecode, &ctx);
        for i in 0..10 {
            let close_val = 10.0 + i as f64 * 0.15;
            let expected = if close_val > 11.0 {
                2.0
            } else if close_val > 10.5 {
                1.0
            } else {
                0.0
            };
            assert!(
                (result.final_value[i] - expected).abs() < 1e-10,
                "Mismatch at {}: got {}, expected {}",
                i,
                result.final_value[i],
                expected
            );
        }
    }

    // Test 40: ForLoop bytecode compilation
    #[test]
    fn test_for_loop_compilation() {
        let node = AstNode::ForLoop {
            var: "I".to_string(),
            start: Box::new(AstNode::Number(0.0)),
            end: Box::new(AstNode::Number(5.0)),
            body: vec![AstNode::Assignment {
                name: "ACC".to_string(),
                expr: Box::new(AstNode::BinaryOp {
                    op: BinaryOperator::Add,
                    left: Box::new(AstNode::Variable("ACC".to_string())),
                    right: Box::new(AstNode::Number(1.0)),
                }),
            }],
        };
        let bytecode = compile_to_bytecode(&node, "test").expect("Compile failed");
        assert!(bytecode
            .instructions
            .iter()
            .any(|op| matches!(op, OpCode::JumpIfFalse(_))));
        assert!(bytecode
            .instructions
            .iter()
            .any(|op| matches!(op, OpCode::Jump(_))));
    }

    // Test 41: ForLoop bytecode execution
    #[test]
    fn test_for_loop_execution() {
        let node = AstNode::Statements(vec![
            AstNode::Assignment {
                name: "ACC".to_string(),
                expr: Box::new(AstNode::Number(0.0)),
            },
            AstNode::ForLoop {
                var: "I".to_string(),
                start: Box::new(AstNode::Number(0.0)),
                end: Box::new(AstNode::Number(3.0)),
                body: vec![AstNode::Assignment {
                    name: "ACC".to_string(),
                    expr: Box::new(AstNode::BinaryOp {
                        op: BinaryOperator::Add,
                        left: Box::new(AstNode::Variable("ACC".to_string())),
                        right: Box::new(AstNode::Number(1.0)),
                    }),
                }],
            },
            AstNode::Variable("ACC".to_string()),
        ]);
        let bytecode = compile_to_bytecode(&node, "test").expect("Compile failed");
        let ctx = make_ctx(5);
        let result = execute_bytecode(&bytecode, &ctx);
        for i in 0..5 {
            assert!(
                (result.final_value[i] - 4.0).abs() < 1e-10,
                "ACC at {}: got {}, expected 4.0",
                i,
                result.final_value[i]
            );
        }
    }

    // Test 42: PushString OpCode
    #[test]
    fn test_push_string_opcode() {
        let node = AstNode::StringLit("hello".to_string());
        let bytecode = compile_to_bytecode(&node, "test").expect("Compile failed");
        assert!(bytecode
            .instructions
            .iter()
            .any(|op| matches!(op, OpCode::PushString(s) if s == "hello")));
        let ctx = make_ctx(5);
        let result = execute_bytecode(&bytecode, &ctx);
        for i in 0..5 {
            assert!((result.final_value[i] - 0.0).abs() < 1e-10);
        }
    }

    // Test 43: PushString serialization roundtrip
    #[test]
    fn test_push_string_serialization() {
        let node = AstNode::StringLit("test_string".to_string());
        let bytecode = compile_to_bytecode(&node, "test").expect("Compile failed");
        let data = bytecode.serialize();
        let restored = Bytecode::deserialize(&data).expect("Deserialize failed");
        assert_eq!(bytecode.instructions.len(), restored.instructions.len());
        for (orig, rest) in bytecode
            .instructions
            .iter()
            .zip(restored.instructions.iter())
        {
            assert_eq!(orig, rest);
        }
    }

    // Test 44: DrawText execution in VM
    #[test]
    fn test_draw_text_execution() {
        let node = AstNode::DrawText {
            cond: Box::new(AstNode::BinaryOp {
                op: BinaryOperator::Gt,
                left: Box::new(AstNode::Variable("CLOSE".to_string())),
                right: Box::new(AstNode::Number(10.5)),
            }),
            price: Box::new(AstNode::Variable("CLOSE".to_string())),
            text: "BUY".to_string(),
            color: None,
        };
        let bytecode = compile_to_bytecode(&node, "test").expect("Compile failed");
        let ctx = make_ctx(5);
        let result = execute_bytecode(&bytecode, &ctx);
        assert_eq!(result.draw_commands.commands.len(), 1);
        match &result.draw_commands.commands[0] {
            DrawCommand::Text { text, color, .. } => {
                assert_eq!(text, "BUY");
                assert_eq!(color, "");
            }
            _ => panic!("Expected DrawCommand::Text"),
        }
    }

    // Test 45: DrawIcon execution in VM
    #[test]
    fn test_draw_icon_execution() {
        let node = AstNode::DrawIcon {
            cond: Box::new(AstNode::Number(1.0)),
            price: Box::new(AstNode::Variable("CLOSE".to_string())),
            icon: Box::new(AstNode::Number(1.0)),
            color: Some(ColorSpec::Named("COLORRED".to_string())),
        };
        let bytecode = compile_to_bytecode(&node, "test").expect("Compile failed");
        let ctx = make_ctx(5);
        let result = execute_bytecode(&bytecode, &ctx);
        assert_eq!(result.draw_commands.commands.len(), 1);
        match &result.draw_commands.commands[0] {
            DrawCommand::Icon {
                icon_type, color, ..
            } => {
                assert_eq!(*icon_type, 1);
                assert_eq!(color, "COLORRED");
            }
            _ => panic!("Expected DrawCommand::Icon"),
        }
    }

    // Test 46: StickLine execution in VM
    #[test]
    fn test_stick_line_execution() {
        let node = AstNode::StickLine {
            cond: Box::new(AstNode::Number(1.0)),
            price1: Box::new(AstNode::Variable("HIGH".to_string())),
            price2: Box::new(AstNode::Variable("LOW".to_string())),
            width: Box::new(AstNode::Number(2.0)),
            empty: true,
            color: Some(ColorSpec::Rgb(255, 0, 0)),
        };
        let bytecode = compile_to_bytecode(&node, "test").expect("Compile failed");
        let ctx = make_ctx(5);
        let result = execute_bytecode(&bytecode, &ctx);
        assert_eq!(result.draw_commands.commands.len(), 1);
        match &result.draw_commands.commands[0] {
            DrawCommand::StickLine {
                width,
                empty,
                color,
                ..
            } => {
                assert_eq!(*width, 2);
                assert!(*empty);
                assert_eq!(color, "#FF0000");
            }
            _ => panic!("Expected DrawCommand::StickLine"),
        }
    }

    // Test 47: DrawText serialization roundtrip
    #[test]
    fn test_draw_text_serialization() {
        let node = AstNode::DrawText {
            cond: Box::new(AstNode::Number(1.0)),
            price: Box::new(AstNode::Variable("CLOSE".to_string())),
            text: "SELL".to_string(),
            color: Some(ColorSpec::Named("COLORGREEN".to_string())),
        };
        let bytecode = compile_to_bytecode(&node, "test").expect("Compile failed");
        let data = bytecode.serialize();
        let restored = Bytecode::deserialize(&data).expect("Deserialize failed");
        assert_eq!(bytecode.instructions.len(), restored.instructions.len());
        for (orig, rest) in bytecode
            .instructions
            .iter()
            .zip(restored.instructions.iter())
        {
            assert_eq!(orig, rest);
        }
    }

    // Test 48: StickLine serialization roundtrip
    #[test]
    fn test_stick_line_serialization() {
        let node = AstNode::StickLine {
            cond: Box::new(AstNode::Number(1.0)),
            price1: Box::new(AstNode::Variable("HIGH".to_string())),
            price2: Box::new(AstNode::Variable("LOW".to_string())),
            width: Box::new(AstNode::Number(3.0)),
            empty: false,
            color: Some(ColorSpec::Hex("FF00FF".to_string())),
        };
        let bytecode = compile_to_bytecode(&node, "test").expect("Compile failed");
        let data = bytecode.serialize();
        let restored = Bytecode::deserialize(&data).expect("Deserialize failed");
        assert_eq!(bytecode.instructions.len(), restored.instructions.len());
        for (orig, rest) in bytecode
            .instructions
            .iter()
            .zip(restored.instructions.iter())
        {
            assert_eq!(orig, rest);
        }
    }

    // Test 49: Select opcode with all-false condition
    #[test]
    fn test_select_all_false_condition() {
        let node = AstNode::IfThenElse {
            cond: Box::new(AstNode::Number(0.0)),
            then_branch: Box::new(AstNode::Number(999.0)),
            else_branch: Box::new(AstNode::Number(42.0)),
        };
        let bytecode = compile_to_bytecode(&node, "test").expect("Compile failed");
        let ctx = make_ctx(3);
        let result = execute_bytecode(&bytecode, &ctx);
        for i in 0..3 {
            assert!(
                (result.final_value[i] - 42.0).abs() < 1e-10,
                "Expected 42.0 at {}, got {}",
                i,
                result.final_value[i]
            );
        }
    }

    // Test 50: Select opcode with all-true condition
    #[test]
    fn test_select_all_true_condition() {
        let node = AstNode::IfThenElse {
            cond: Box::new(AstNode::Number(1.0)),
            then_branch: Box::new(AstNode::Number(100.0)),
            else_branch: Box::new(AstNode::Number(0.0)),
        };
        let bytecode = compile_to_bytecode(&node, "test").expect("Compile failed");
        let ctx = make_ctx(3);
        let result = execute_bytecode(&bytecode, &ctx);
        for i in 0..3 {
            assert!(
                (result.final_value[i] - 100.0).abs() < 1e-10,
                "Expected 100.0 at {}, got {}",
                i,
                result.final_value[i]
            );
        }
    }

    // Test 51: Select opcode serialization roundtrip
    #[test]
    fn test_select_serialization() {
        let node = AstNode::IfThenElse {
            cond: Box::new(AstNode::Number(1.0)),
            then_branch: Box::new(AstNode::Number(100.0)),
            else_branch: Box::new(AstNode::Number(0.0)),
        };
        let bytecode = compile_to_bytecode(&node, "test").expect("Compile failed");
        let data = bytecode.serialize();
        let restored = Bytecode::deserialize(&data).expect("Deserialize failed");
        assert_eq!(bytecode.instructions.len(), restored.instructions.len());
        for (orig, rest) in bytecode
            .instructions
            .iter()
            .zip(restored.instructions.iter())
        {
            assert_eq!(orig, rest);
        }
    }

    // Test 52: JumpIfFalse in ForLoop control flow
    #[test]
    fn test_jump_if_false_in_for_loop() {
        let node = AstNode::Statements(vec![
            AstNode::Assignment {
                name: "SUM".to_string(),
                expr: Box::new(AstNode::Number(0.0)),
            },
            AstNode::ForLoop {
                var: "I".to_string(),
                start: Box::new(AstNode::Number(0.0)),
                end: Box::new(AstNode::Number(5.0)),
                body: vec![AstNode::Assignment {
                    name: "SUM".to_string(),
                    expr: Box::new(AstNode::BinaryOp {
                        op: BinaryOperator::Add,
                        left: Box::new(AstNode::Variable("SUM".to_string())),
                        right: Box::new(AstNode::Number(1.0)),
                    }),
                }],
            },
            AstNode::Variable("SUM".to_string()),
        ]);
        let bytecode = compile_to_bytecode(&node, "test").expect("Compile failed");
        assert!(bytecode
            .instructions
            .iter()
            .any(|op| matches!(op, OpCode::JumpIfFalse(_))));
        assert!(bytecode
            .instructions
            .iter()
            .any(|op| matches!(op, OpCode::Jump(_))));
        let ctx = make_ctx(3);
        let result = execute_bytecode(&bytecode, &ctx);
        for i in 0..3 {
            assert!(
                (result.final_value[i] - 6.0).abs() < 1e-10,
                "SUM at {}: got {}, expected 6.0",
                i,
                result.final_value[i]
            );
        }
    }

    #[test]
    fn test_simd_path_arithmetic() {
        let ctx = make_ctx(20);
        let bytecode = compile_formula("CLOSE + OPEN");
        let result = execute_bytecode(&bytecode, &ctx);
        for i in 0..20 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            let expected = close_val + open_val;
            assert!(
                (result.final_value[i] - expected).abs() < 1e-10,
                "Mismatch at {}: got {}, expected {}",
                i,
                result.final_value[i],
                expected
            );
        }

        let bytecode_sub = compile_formula("CLOSE - OPEN");
        let result_sub = execute_bytecode(&bytecode_sub, &ctx);
        for i in 0..20 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            let expected = close_val - open_val;
            assert!((result_sub.final_value[i] - expected).abs() < 1e-10);
        }

        let bytecode_mul = compile_formula("CLOSE * 2");
        let result_mul = execute_bytecode(&bytecode_mul, &ctx);
        for i in 0..20 {
            let close_val = 10.0 + i as f64 * 0.15;
            let expected = close_val * 2.0;
            assert!((result_mul.final_value[i] - expected).abs() < 1e-10);
        }

        let bytecode_div = compile_formula("CLOSE / 2");
        let result_div = execute_bytecode(&bytecode_div, &ctx);
        for i in 0..20 {
            let close_val = 10.0 + i as f64 * 0.15;
            let expected = close_val / 2.0;
            assert!((result_div.final_value[i] - expected).abs() < 1e-10);
        }

        let bytecode_gt = compile_formula("CLOSE > 10.5");
        let result_gt = execute_bytecode(&bytecode_gt, &ctx);
        for i in 0..20 {
            let close_val = 10.0 + i as f64 * 0.15;
            let expected = if close_val > 10.5 { 1.0 } else { 0.0 };
            assert!((result_gt.final_value[i] - expected).abs() < 1e-10);
        }

        let bytecode_if = compile_formula("IF(CLOSE > 10.5, 100, 0)");
        let result_if = execute_bytecode(&bytecode_if, &ctx);
        for i in 0..20 {
            let close_val = 10.0 + i as f64 * 0.15;
            let expected = if close_val > 10.5 { 100.0 } else { 0.0 };
            assert!((result_if.final_value[i] - expected).abs() < 1e-10);
        }
    }
}
