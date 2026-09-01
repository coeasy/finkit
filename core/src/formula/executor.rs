use ndarray::Array1;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use crate::formula::ast::*;
use crate::formula::functions::get_builtin_functions;
use crate::formula::memory_pool::BufferPool;
use crate::formula::simd::SimdOps;
use crate::formula::types::*;

type FormulaFn = fn(&FormulaContext, &[Array1<f64>]) -> Result<Array1<f64>, FormulaError>;

fn color_to_string(color: &Option<ColorSpec>) -> String {
    match color {
        None => String::new(),
        Some(ColorSpec::Named(s)) => s.to_string(),
        Some(ColorSpec::Rgb(r, g, b)) => format!("#{:02X}{:02X}{:02X}", r, g, b),
        Some(ColorSpec::Hex(s)) => format!("#{}", s),
    }
}

pub struct FormulaExecutor {
    functions: HashMap<String, FormulaFn>,
    buffer_pool: RefCell<BufferPool>,
    /// Reused across `execute_zero_copy_cached` calls to avoid re-allocating
    /// the per-execution `VarNameCache` and the common-name entries it pre-fills.
    name_cache: RefCell<VarNameCache>,
}

impl Default for FormulaExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl FormulaExecutor {
    pub fn new() -> Self {
        let mut name_cache = VarNameCache::new();
        name_cache.pre_cache_common();
        Self {
            functions: get_builtin_functions(),
            buffer_pool: RefCell::new(BufferPool::new(10000, 8)),
            name_cache: RefCell::new(name_cache),
        }
    }

    pub fn execute(
        &self,
        ast: &AstNode,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        ctx.reset_sandbox();
        let val = self.execute_val(ast, ctx)?;
        Ok(val.to_array(ctx.data_len))
    }

    pub fn execute_val(
        &self,
        ast: &AstNode,
        ctx: &mut FormulaContext,
    ) -> Result<FormulaValue, FormulaError> {
        let sandbox_cfg = ctx.sandbox;
        let entering_at_top = ctx.sandbox_state.borrow().recursion_depth() == 0;
        crate::formula::sandbox::sandbox_push(&sandbox_cfg, &ctx.sandbox_state)?;
        let result = self.execute_val_inner(ast, ctx);
        crate::formula::sandbox::sandbox_pop(&ctx.sandbox_state);
        if result.is_ok() && entering_at_top {
            if let Err(e) = crate::formula::sandbox::sandbox_track_bytes(
                &sandbox_cfg,
                &ctx.sandbox_state,
                ctx.data_len * 8,
            ) {
                return Err(e);
            }
        }
        result
    }

    fn execute_val_inner(
        &self,
        ast: &AstNode,
        ctx: &mut FormulaContext,
    ) -> Result<FormulaValue, FormulaError> {
        match ast {
            AstNode::Statements(stmts) => {
                let mut result = FormulaValue::Scalar(0.0);
                for stmt in stmts {
                    result = self.execute_val(stmt, ctx)?;
                }
                Ok(result)
            }
            AstNode::Assignment { name, expr } => {
                let value = self.execute_val(expr, ctx)?;
                let name_arc: VarName = Arc::from(name.clone());
                match &value {
                    FormulaValue::Scalar(v) => {
                        ctx.variables
                            .insert(name_arc, Array1::from_elem(ctx.data_len, *v));
                    }
                    FormulaValue::Array(arr) => {
                        ctx.variables.insert(name_arc, arr.clone());
                    }
                }
                Ok(value)
            }
            AstNode::CompoundAssignment { name, op, expr } => {
                let current = self.resolve_variable(name, ctx)?;
                let rhs_val = self.execute_val(expr, ctx)?;
                let rhs = rhs_val.to_array(ctx.data_len);
                let value = self.apply_compound_assign(op, &current, &rhs)?;
                ctx.variables.insert(
                    Arc::from(name.to_string()),
                    FormulaContext::copy_array(&value),
                );
                Ok(FormulaValue::Array(value))
            }
            AstNode::Output {
                name,
                expr,
                modifier,
            } => {
                let value = self.execute_val(expr, ctx)?;
                let name_arc: VarName = Arc::from(name.clone());
                match &value {
                    FormulaValue::Scalar(v) => {
                        ctx.variables
                            .insert(name_arc, Array1::from_elem(ctx.data_len, *v));
                    }
                    FormulaValue::Array(arr) => {
                        ctx.variables.insert(name_arc, arr.clone());
                    }
                }
                if let Some(modifier) = modifier {
                    ctx.output_modifiers
                        .insert(name.to_string(), modifier.clone());
                }
                Ok(value)
            }
            AstNode::Variable(name) => self.resolve_variable_val(name, ctx),
            AstNode::Number(val) => Ok(FormulaValue::Scalar(*val)),
            AstNode::BinaryOp { op, left, right } => {
                let left_val = self.execute_val(left, ctx)?;
                let right_val = self.execute_val(right, ctx)?;
                self.apply_binary_op_val(op, left_val, right_val, ctx.data_len)
            }
            AstNode::UnaryOp { op, expr } => {
                let val = self.execute_val(expr, ctx)?;
                self.apply_unary_op_val(op, val, ctx.data_len)
            }
            AstNode::FunctionCall { name, args } => {
                let arg_values: Result<Vec<Array1<f64>>, FormulaError> = args
                    .iter()
                    .map(|a| {
                        let v = self.execute_val(a, ctx)?;
                        Ok(v.to_array(ctx.data_len))
                    })
                    .collect();
                let arg_values = arg_values?;
                let result = self.call_function(name, ctx, &arg_values)?;
                Ok(FormulaValue::Array(result))
            }
            AstNode::IndexAccess { array, index } => {
                let arr_val = self.execute_val(array, ctx)?;
                let idx_val = self.execute_val(index, ctx)?;
                let arr = arr_val.to_array(ctx.data_len);
                let idx = idx_val.to_array(ctx.data_len);
                let mut result = Array1::zeros(ctx.data_len);
                for i in 0..ctx.data_len {
                    let idx_i = idx[i] as usize;
                    if idx_i < arr.len() {
                        result[i] = arr[idx_i];
                    } else {
                        result[i] = f64::NAN;
                    }
                }
                Ok(FormulaValue::Array(result))
            }
            AstNode::StringLit(s) => {
                let idx = ctx.string_table.len();
                ctx.string_table.push(s.clone());
                Ok(FormulaValue::Scalar(idx as f64))
            }
            AstNode::ParamDecl { .. } => Err(FormulaError::RuntimeError(
                "ParamDecl should be handled at parse time".to_string(),
            )),
            AstNode::DrawText {
                cond,
                price,
                text,
                color,
            } => {
                let cond_val = self.execute_val(cond, ctx)?;
                let price_val = self.execute_val(price, ctx)?;
                let cond_arr = cond_val.to_array(ctx.data_len);
                let price_arr = price_val.to_array(ctx.data_len);
                let color_str = color_to_string(color);
                ctx.draw_commands.borrow_mut().add_text(
                    cond_arr,
                    price_arr,
                    text.to_string(),
                    color_str,
                );
                Ok(FormulaValue::Scalar(0.0))
            }
            AstNode::DrawIcon {
                cond,
                price,
                icon,
                color,
            } => {
                let cond_val = self.execute_val(cond, ctx)?;
                let price_val = self.execute_val(price, ctx)?;
                let icon_val = self.execute_val(icon, ctx)?;
                let cond_arr = cond_val.to_array(ctx.data_len);
                let price_arr = price_val.to_array(ctx.data_len);
                let icon_arr = icon_val.to_array(ctx.data_len);
                let icon_type = icon_arr[0] as i32;
                let color_str = color_to_string(color);
                ctx.draw_commands
                    .borrow_mut()
                    .add_icon(cond_arr, price_arr, icon_type, color_str);
                Ok(FormulaValue::Scalar(0.0))
            }
            AstNode::StickLine {
                cond,
                price1,
                price2,
                width,
                empty,
                color,
            } => {
                let cond_val = self.execute_val(cond, ctx)?;
                let price1_val = self.execute_val(price1, ctx)?;
                let price2_val = self.execute_val(price2, ctx)?;
                let width_val = self.execute_val(width, ctx)?;
                let cond_arr = cond_val.to_array(ctx.data_len);
                let price1_arr = price1_val.to_array(ctx.data_len);
                let price2_arr = price2_val.to_array(ctx.data_len);
                let width_arr = width_val.to_array(ctx.data_len);
                let width_int = width_arr[0] as i32;
                let color_str = color_to_string(color);
                ctx.draw_commands.borrow_mut().add_stick(
                    cond_arr, price1_arr, price2_arr, width_int, *empty, color_str,
                );
                Ok(FormulaValue::Scalar(0.0))
            }
            AstNode::DrawGeneric {
                command,
                args,
                color,
            } => {
                let mut evaluated = Vec::with_capacity(args.len());
                for arg in args {
                    let v = self.execute_val(arg, ctx)?;
                    evaluated.push(v.to_array(ctx.data_len));
                }
                let color_str = color_to_string(color);
                match (command.as_str(), evaluated.len()) {
                    ("DRAWLINE", n) if n >= 5 => {
                        let expand = evaluated[4][0] as i32;
                        ctx.draw_commands.borrow_mut().add_line(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            expand,
                            color_str,
                        );
                    }
                    ("DRAWBAND", n) if n >= 2 => {
                        ctx.draw_commands.borrow_mut().add_band(
                            evaluated.remove(0),
                            color_str.clone(),
                            evaluated.remove(0),
                            color_str,
                        );
                    }
                    ("DRAWKLINE", n) if n >= 4 => {
                        ctx.draw_commands.borrow_mut().add_kline(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                        );
                    }
                    ("DRAWRECTREL", n) if n >= 4 => {
                        ctx.draw_commands.borrow_mut().add_rect(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            color_str,
                        );
                    }
                    ("FILLRGN", n) if n >= 3 => {
                        ctx.draw_commands.borrow_mut().add_fill_rgn(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            color_str,
                        );
                    }
                    ("PARTLINE", n) if n >= 2 => {
                        ctx.draw_commands.borrow_mut().add_part_line(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            color_str,
                        );
                    }
                    ("POLYLINE", n) if n >= 2 => {
                        ctx.draw_commands.borrow_mut().add_poly_line(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            color_str,
                        );
                    }
                    ("DRAWGBK", n) if n >= 1 => {
                        ctx.draw_commands
                            .borrow_mut()
                            .add_background(evaluated.remove(0), color_str);
                    }
                    ("DRAWSL", n) if n >= 4 => {
                        ctx.draw_commands.borrow_mut().add_slope_line(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            color_str,
                        );
                    }
                    ("DRAWTEXT_FIX", n) if n >= 3 => {
                        let x = evaluated[0][0];
                        let y = evaluated[1][0];
                        ctx.draw_commands
                            .borrow_mut()
                            .add_text_fix(x, y, String::new(), color_str);
                    }
                    ("DRAWNUMBER", n) if n >= 4 => {
                        let precision = evaluated[3][0] as i32;
                        ctx.draw_commands.borrow_mut().add_number(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            precision,
                            color_str,
                        );
                    }
                    ("VERTLINE", n) if n >= 1 => {
                        ctx.draw_commands
                            .borrow_mut()
                            .add_vert_line(evaluated.remove(0), color_str);
                    }
                    _ => {}
                }
                Ok(FormulaValue::Scalar(0.0))
            }
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond_val = self.execute_val(cond, ctx)?;
                let then_val = self.execute_val(then_branch, ctx)?;
                let else_val = self.execute_val(else_branch, ctx)?;
                match (&cond_val, &then_val, &else_val) {
                    (FormulaValue::Scalar(c), FormulaValue::Scalar(t), FormulaValue::Scalar(e)) => {
                        Ok(FormulaValue::Scalar(if *c > 0.0 { *t } else { *e }))
                    }
                    _ => {
                        let cond_arr = cond_val.to_array(ctx.data_len);
                        let then_arr = then_val.to_array(ctx.data_len);
                        let else_arr = else_val.to_array(ctx.data_len);
                        let result = if cond_arr.len() >= 16 {
                            SimdOps::simd_select_arrays(&cond_arr, &then_arr, &else_arr)
                        } else {
                            cond_arr
                                .iter()
                                .zip(then_arr.iter())
                                .zip(else_arr.iter())
                                .map(|((&c, &t), &e)| if c > 0.0 { t } else { e })
                                .collect()
                        };
                        Ok(FormulaValue::Array(result))
                    }
                }
            }
            AstNode::ForLoop {
                var,
                start,
                end,
                body,
            } => {
                let start_val = self.execute_val(start, ctx)?;
                let end_val = self.execute_val(end, ctx)?;
                let start_i = start_val.as_scalar().unwrap_or_else(|| {
                    let arr = start_val.to_array(ctx.data_len);
                    arr[0]
                }) as i64;
                let end_i = end_val.as_scalar().unwrap_or_else(|| {
                    let arr = end_val.to_array(ctx.data_len);
                    arr[0]
                }) as i64;
                let mut result = FormulaValue::Scalar(0.0);
                let max_iterations = 10000i64;
                for (count, i) in (start_i..=end_i).enumerate() {
                    if count as i64 >= max_iterations {
                        return Err(FormulaError::RuntimeError(format!(
                            "FOR loop exceeded maximum iterations ({})",
                            max_iterations
                        )));
                    }
                    ctx.variables.insert(
                        Arc::from(var.to_string()),
                        Array1::from_elem(ctx.data_len, i as f64),
                    );
                    for stmt in body {
                        result = self.execute_val(stmt, ctx)?;
                    }
                }
                Ok(result)
            }
            AstNode::WhileLoop { cond, body } => {
                let mut result = FormulaValue::Scalar(0.0);
                let max_iterations = 10000usize;
                for _ in 0..max_iterations {
                    let cond_val = self.execute_val(cond, ctx)?;
                    let cond_arr = cond_val.to_array(ctx.data_len);
                    if !cond_arr.iter().any(|&v| v > 0.0) {
                        break;
                    }
                    for stmt in body {
                        result = self.execute_val(stmt, ctx)?;
                    }
                }
                Ok(result)
            }
        }
    }

    fn resolve_variable_val(
        &self,
        name: &str,
        ctx: &FormulaContext,
    ) -> Result<FormulaValue, FormulaError> {
        match classify_builtin_var(name) {
            Some(BuiltinVar::Close) => Ok(FormulaValue::Array(ctx.close_view().to_owned())),
            Some(BuiltinVar::High) => Ok(FormulaValue::Array(ctx.high_view().to_owned())),
            Some(BuiltinVar::Low) => Ok(FormulaValue::Array(ctx.low_view().to_owned())),
            Some(BuiltinVar::Open) => Ok(FormulaValue::Array(ctx.open_view().to_owned())),
            Some(BuiltinVar::Volume) => Ok(FormulaValue::Array(ctx.volume_view().to_owned())),
            Some(BuiltinVar::Amount) => ctx
                .amount
                .as_ref()
                .map(|a| FormulaValue::Array(FormulaContext::copy_array(a)))
                .ok_or_else(|| FormulaError::RuntimeError("AMOUNT data not available".to_string())),
            Some(BuiltinVar::BarsCount) => Ok(FormulaValue::Scalar(ctx.data_len as f64)),
            Some(BuiltinVar::BarPos) => Ok(FormulaValue::Array(Array1::from(
                (1..=ctx.data_len).map(|i| i as f64).collect::<Vec<_>>(),
            ))),
            Some(BuiltinVar::Capital) => Ok(FormulaValue::Scalar(ctx.capital.unwrap_or(f64::NAN))),
            Some(BuiltinVar::DrawNull) => Ok(FormulaValue::Scalar(f64::NAN)),
            None => ctx
                .variables
                .get(name)
                .map(|v| FormulaValue::Array(FormulaContext::copy_array(v)))
                .ok_or_else(|| FormulaError::RuntimeError(format!("Unknown variable: {}", name))),
        }
    }

    fn resolve_variable(
        &self,
        name: &str,
        ctx: &FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let val = self.resolve_variable_val(name, ctx)?;
        Ok(val.to_array(ctx.data_len))
    }

    fn apply_binary_op_val(
        &self,
        op: &BinaryOperator,
        left: FormulaValue,
        right: FormulaValue,
        data_len: usize,
    ) -> Result<FormulaValue, FormulaError> {
        match (&left, &right) {
            (FormulaValue::Scalar(l), FormulaValue::Scalar(r)) => {
                let result = apply_scalar_op(op, *l, *r)?;
                Ok(FormulaValue::Scalar(result))
            }
            (FormulaValue::Scalar(s), FormulaValue::Array(a)) => {
                let mut result = Array1::zeros(data_len);
                apply_scalar_array_op(op, *s, a, &mut result)?;
                Ok(FormulaValue::Array(result))
            }
            (FormulaValue::Array(a), FormulaValue::Scalar(s)) => {
                let mut result = Array1::zeros(data_len);
                apply_array_scalar_op(op, a, *s, &mut result)?;
                Ok(FormulaValue::Array(result))
            }
            (FormulaValue::Array(l), FormulaValue::Array(r)) => {
                let result = self.apply_binary_op(op, l, r)?;
                Ok(FormulaValue::Array(result))
            }
        }
    }

    fn apply_unary_op_val(
        &self,
        op: &UnaryOperator,
        val: FormulaValue,
        _data_len: usize,
    ) -> Result<FormulaValue, FormulaError> {
        match (&val, op) {
            (FormulaValue::Scalar(v), UnaryOperator::Not) => {
                Ok(FormulaValue::Scalar(if *v <= 0.0 { 1.0 } else { 0.0 }))
            }
            (FormulaValue::Scalar(v), UnaryOperator::Neg) => Ok(FormulaValue::Scalar(-*v)),
            (FormulaValue::Array(a), UnaryOperator::Not) => {
                let result = self.apply_unary_op(op, a)?;
                Ok(FormulaValue::Array(result))
            }
            (FormulaValue::Array(a), UnaryOperator::Neg) => {
                let result = self.apply_unary_op(op, a)?;
                Ok(FormulaValue::Array(result))
            }
        }
    }
}

fn apply_scalar_op(op: &BinaryOperator, l: f64, r: f64) -> Result<f64, FormulaError> {
    match op {
        BinaryOperator::Add => Ok(l + r),
        BinaryOperator::Sub => Ok(l - r),
        BinaryOperator::Mul => Ok(l * r),
        BinaryOperator::Div => {
            if r.abs() < 1e-15 {
                Ok(f64::NAN)
            } else {
                Ok(l / r)
            }
        }
        BinaryOperator::Mod => {
            if r.abs() < 1e-15 {
                Ok(f64::NAN)
            } else {
                Ok(l - (l / r).floor() * r)
            }
        }
        BinaryOperator::Pow => Ok(l.powf(r)),
        BinaryOperator::Gt => Ok(if l > r { 1.0 } else { 0.0 }),
        BinaryOperator::Lt => Ok(if l < r { 1.0 } else { 0.0 }),
        BinaryOperator::Gte => Ok(if l >= r { 1.0 } else { 0.0 }),
        BinaryOperator::Lte => Ok(if l <= r { 1.0 } else { 0.0 }),
        BinaryOperator::Eq => Ok(if (l - r).abs() < 1e-10 { 1.0 } else { 0.0 }),
        BinaryOperator::Neq => Ok(if (l - r).abs() >= 1e-10 { 1.0 } else { 0.0 }),
        BinaryOperator::And => Ok(if l > 0.0 && r > 0.0 { 1.0 } else { 0.0 }),
        BinaryOperator::Or => Ok(if l > 0.0 || r > 0.0 { 1.0 } else { 0.0 }),
        BinaryOperator::Xor => Ok(if (l > 0.0) != (r > 0.0) { 1.0 } else { 0.0 }),
        BinaryOperator::StringConcat => Err(FormulaError::InvalidOperation(
            "String concatenation (&) is not supported for numeric values. Use STRCAT() function instead.".to_string()
        )),
    }
}

fn apply_scalar_array_op(
    op: &BinaryOperator,
    scalar: f64,
    arr: &Array1<f64>,
    result: &mut Array1<f64>,
) -> Result<(), FormulaError> {
    let len = arr.len();
    match op {
        BinaryOperator::Add => {
            for i in 0..len {
                result[i] = scalar + arr[i];
            }
        }
        BinaryOperator::Sub => {
            for i in 0..len {
                result[i] = scalar - arr[i];
            }
        }
        BinaryOperator::Mul => {
            for i in 0..len {
                result[i] = scalar * arr[i];
            }
        }
        BinaryOperator::Div => {
            for i in 0..len {
                result[i] = if arr[i].abs() < 1e-15 {
                    f64::NAN
                } else {
                    scalar / arr[i]
                };
            }
        }
        BinaryOperator::Mod => {
            for i in 0..len {
                result[i] = if arr[i].abs() < 1e-15 {
                    f64::NAN
                } else {
                    scalar - (scalar / arr[i]).floor() * arr[i]
                };
            }
        }
        BinaryOperator::Pow => {
            for i in 0..len {
                result[i] = scalar.powf(arr[i]);
            }
        }
        BinaryOperator::Gt => {
            for i in 0..len {
                result[i] = if scalar > arr[i] { 1.0 } else { 0.0 };
            }
        }
        BinaryOperator::Lt => {
            for i in 0..len {
                result[i] = if scalar < arr[i] { 1.0 } else { 0.0 };
            }
        }
        BinaryOperator::Gte => {
            for i in 0..len {
                result[i] = if scalar >= arr[i] { 1.0 } else { 0.0 };
            }
        }
        BinaryOperator::Lte => {
            for i in 0..len {
                result[i] = if scalar <= arr[i] { 1.0 } else { 0.0 };
            }
        }
        BinaryOperator::Eq => {
            for i in 0..len {
                result[i] = if (scalar - arr[i]).abs() < 1e-10 {
                    1.0
                } else {
                    0.0
                };
            }
        }
        BinaryOperator::Neq => {
            for i in 0..len {
                result[i] = if (scalar - arr[i]).abs() >= 1e-10 {
                    1.0
                } else {
                    0.0
                };
            }
        }
        BinaryOperator::And => {
            for i in 0..len {
                result[i] = if scalar > 0.0 && arr[i] > 0.0 {
                    1.0
                } else {
                    0.0
                };
            }
        }
        BinaryOperator::Or => {
            for i in 0..len {
                result[i] = if scalar > 0.0 || arr[i] > 0.0 {
                    1.0
                } else {
                    0.0
                };
            }
        }
        BinaryOperator::Xor => {
            for i in 0..len {
                result[i] = if (scalar > 0.0) != (arr[i] > 0.0) {
                    1.0
                } else {
                    0.0
                };
            }
        }
        BinaryOperator::StringConcat => {
            return Err(FormulaError::InvalidOperation(
                "String concatenation (&) is not supported for numeric values. Use STRCAT() function instead.".to_string()
            ));
        }
    }
    Ok(())
}

fn apply_array_scalar_op(
    op: &BinaryOperator,
    arr: &Array1<f64>,
    scalar: f64,
    result: &mut Array1<f64>,
) -> Result<(), FormulaError> {
    let len = arr.len();
    match op {
        BinaryOperator::Add => {
            for i in 0..len {
                result[i] = arr[i] + scalar;
            }
        }
        BinaryOperator::Sub => {
            for i in 0..len {
                result[i] = arr[i] - scalar;
            }
        }
        BinaryOperator::Mul => {
            for i in 0..len {
                result[i] = arr[i] * scalar;
            }
        }
        BinaryOperator::Div => {
            if scalar.abs() < 1e-15 {
                for i in 0..len {
                    result[i] = f64::NAN;
                }
            } else {
                for i in 0..len {
                    result[i] = arr[i] / scalar;
                }
            }
        }
        BinaryOperator::Mod => {
            if scalar.abs() < 1e-15 {
                for i in 0..len {
                    result[i] = f64::NAN;
                }
            } else {
                for i in 0..len {
                    result[i] = arr[i] - (arr[i] / scalar).floor() * scalar;
                }
            }
        }
        BinaryOperator::Pow => {
            for i in 0..len {
                result[i] = arr[i].powf(scalar);
            }
        }
        BinaryOperator::Gt => {
            for i in 0..len {
                result[i] = if arr[i] > scalar { 1.0 } else { 0.0 };
            }
        }
        BinaryOperator::Lt => {
            for i in 0..len {
                result[i] = if arr[i] < scalar { 1.0 } else { 0.0 };
            }
        }
        BinaryOperator::Gte => {
            for i in 0..len {
                result[i] = if arr[i] >= scalar { 1.0 } else { 0.0 };
            }
        }
        BinaryOperator::Lte => {
            for i in 0..len {
                result[i] = if arr[i] <= scalar { 1.0 } else { 0.0 };
            }
        }
        BinaryOperator::Eq => {
            for i in 0..len {
                result[i] = if (arr[i] - scalar).abs() < 1e-10 {
                    1.0
                } else {
                    0.0
                };
            }
        }
        BinaryOperator::Neq => {
            for i in 0..len {
                result[i] = if (arr[i] - scalar).abs() >= 1e-10 {
                    1.0
                } else {
                    0.0
                };
            }
        }
        BinaryOperator::And => {
            for i in 0..len {
                result[i] = if arr[i] > 0.0 && scalar > 0.0 {
                    1.0
                } else {
                    0.0
                };
            }
        }
        BinaryOperator::Or => {
            for i in 0..len {
                result[i] = if arr[i] > 0.0 || scalar > 0.0 {
                    1.0
                } else {
                    0.0
                };
            }
        }
        BinaryOperator::Xor => {
            for i in 0..len {
                result[i] = if (arr[i] > 0.0) != (scalar > 0.0) {
                    1.0
                } else {
                    0.0
                };
            }
        }
        BinaryOperator::StringConcat => {
            return Err(FormulaError::InvalidOperation(
                "String concatenation (&) is not supported for numeric values. Use STRCAT() function instead.".to_string()
            ));
        }
    }
    Ok(())
}

impl FormulaExecutor {
    fn apply_binary_op(
        &self,
        op: &BinaryOperator,
        left: &Array1<f64>,
        right: &Array1<f64>,
    ) -> Result<Array1<f64>, FormulaError> {
        match op {
            BinaryOperator::Add => Ok(SimdOps::simd_add_arrays(left, right)),
            BinaryOperator::Sub => Ok(SimdOps::simd_sub_arrays(left, right)),
            BinaryOperator::Mul => Ok(SimdOps::simd_mul_arrays(left, right)),
            BinaryOperator::Div => Ok(SimdOps::simd_div_arrays(left, right)),
            BinaryOperator::Mod => Ok(SimdOps::simd_mod_arrays(left, right)),
            BinaryOperator::Pow => Ok(SimdOps::simd_pow_arrays(left, right)),
            BinaryOperator::Gt => Ok(SimdOps::simd_gt_arrays(left, right)),
            BinaryOperator::Lt => Ok(SimdOps::simd_lt_arrays(left, right)),
            BinaryOperator::Gte => Ok(SimdOps::simd_gte_arrays(left, right)),
            BinaryOperator::Lte => Ok(SimdOps::simd_lte_arrays(left, right)),
            BinaryOperator::Eq => Ok(SimdOps::simd_eq_arrays(left, right)),
            BinaryOperator::Neq => Ok(SimdOps::simd_neq_arrays(left, right)),
            BinaryOperator::And => {
                let len = left.len();
                let mut result = Array1::zeros(len);
                SimdOps::logical_and(
                    left.as_slice().unwrap(),
                    right.as_slice().unwrap(),
                    result.as_slice_mut().unwrap(),
                );
                Ok(result)
            }
            BinaryOperator::Or => {
                let len = left.len();
                let mut result = Array1::zeros(len);
                SimdOps::logical_or(
                    left.as_slice().unwrap(),
                    right.as_slice().unwrap(),
                    result.as_slice_mut().unwrap(),
                );
                Ok(result)
            }
            BinaryOperator::Xor => {
                let len = left.len();
                let mut result = Array1::zeros(len);
                SimdOps::logical_xor(
                    left.as_slice().unwrap(),
                    right.as_slice().unwrap(),
                    result.as_slice_mut().unwrap(),
                );
                Ok(result)
            }
            BinaryOperator::StringConcat => {
                Err(FormulaError::InvalidOperation(
                    "String concatenation (&) is not supported for numeric values. Use STRCAT() function instead.".to_string()
                ))
            }
        }
    }

    fn apply_unary_op(
        &self,
        op: &UnaryOperator,
        val: &Array1<f64>,
    ) -> Result<Array1<f64>, FormulaError> {
        match op {
            UnaryOperator::Not => {
                let len = val.len();
                let mut result = Array1::zeros(len);
                SimdOps::logical_not(val.as_slice().unwrap(), result.as_slice_mut().unwrap());
                Ok(result)
            }
            UnaryOperator::Neg => Ok(-val),
        }
    }

    fn apply_compound_assign(
        &self,
        op: &CompoundAssignOp,
        current: &Array1<f64>,
        rhs: &Array1<f64>,
    ) -> Result<Array1<f64>, FormulaError> {
        match op {
            CompoundAssignOp::AddAssign => Ok(SimdOps::simd_add_arrays(current, rhs)),
            CompoundAssignOp::SubAssign => Ok(SimdOps::simd_sub_arrays(current, rhs)),
            CompoundAssignOp::MulAssign => Ok(SimdOps::simd_mul_arrays(current, rhs)),
            CompoundAssignOp::DivAssign => Ok(SimdOps::simd_div_arrays(current, rhs)),
        }
    }

    fn call_function(
        &self,
        name: &str,
        ctx: &FormulaContext,
        args: &[Array1<f64>],
    ) -> Result<Array1<f64>, FormulaError> {
        let func = self
            .functions
            .get(name)
            .ok_or_else(|| FormulaError::UnsupportedFunction(format!(
                "Function '{}' is not implemented. Check spelling or ensure this function is available.",
                name
            )))?;
        func(ctx, args)
    }

    pub fn execute_zero_copy(
        &self,
        ast: &AstNode,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let mut pool = self.buffer_pool.borrow_mut();
        self.execute_with_pool(ast, ctx, &mut pool)
    }

    /// 使用 VarNameCache 的零拷贝执行路径，避免重复创建 Arc<str>
    pub fn execute_zero_copy_cached(
        &self,
        ast: &AstNode,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let mut pool = self.buffer_pool.borrow_mut();
        let mut name_cache = self.name_cache.borrow_mut();
        self.execute_with_pool_cached(ast, ctx, &mut pool, &mut name_cache)
    }

    /// 零分配热路径：复用 caller 预分配的 `output` buffer。
    ///
    /// 内部走带缓存的 pooled execution 路径,但最后一根 buffer 通过 `assign` 写回到
    /// caller 提供的 `output`,**不再产生任何 Array1 分配**。
    ///
    /// # 零分配保证
    /// - 中间 buffer 来自 `BufferPool` (复用)
    /// - 最终结果 `assign` 到 caller 的 `output` (不分配)
    ///
    /// # 兼容性
    /// 旧 `execute()` API 行为不变。`output` 长度必须等于 `ctx.data_len`。
    pub fn eval_into(
        &self,
        ast: &AstNode,
        ctx: &mut FormulaContext,
        output: &mut Array1<f64>,
    ) -> Result<(), FormulaError> {
        if output.len() != ctx.data_len {
            return Err(FormulaError::InvalidParameter(format!(
                "output length {} != ctx.data_len {}",
                output.len(),
                ctx.data_len
            )));
        }
        let mut pool = self.buffer_pool.borrow_mut();
        let mut name_cache = self.name_cache.borrow_mut();
        let result = self.execute_with_pool_cached(ast, ctx, &mut pool, &mut name_cache)?;
        // 关键：直接 assign 到 caller 提供的 buffer,无 clone
        output.assign(&result);
        // The result is no longer needed after the copy. Returning it to the
        // pool makes repeated eval_into calls allocation-free after warm-up.
        pool.return_buffer(result);
        Ok(())
    }

    /// 零拷贝热路径:返回的 Array1 与 ctx 内部 buffer 共享内存(不复制)。
    ///
    /// 与 `execute_zero_copy` 等价,提供更直观的命名。
    pub fn eval_borrowed(
        &self,
        ast: &AstNode,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        self.execute_zero_copy(ast, ctx)
    }

    fn execute_with_pool_cached(
        &self,
        ast: &AstNode,
        ctx: &mut FormulaContext,
        pool: &mut BufferPool,
        name_cache: &mut VarNameCache,
    ) -> Result<Array1<f64>, FormulaError> {
        match ast {
            AstNode::Statements(stmts) => {
                let mut result = pool.get_buffer(ctx.data_len);
                for stmt in stmts {
                    let new_result = self.execute_with_pool_cached(stmt, ctx, pool, name_cache)?;
                    pool.return_buffer(result);
                    result = new_result;
                }
                Ok(result)
            }
            AstNode::Assignment { name, expr } => {
                let value = self.execute_with_pool_cached(expr, ctx, pool, name_cache)?;
                let name_arc = name_cache.get_or_create(name);
                ctx.assign_var_no_copy(name_arc, value.clone());
                Ok(value)
            }
            AstNode::CompoundAssignment { name, op, expr } => {
                let current = self.resolve_variable_zero_copy(name, ctx, pool)?;
                let rhs = self.execute_with_pool_cached(expr, ctx, pool, name_cache)?;
                let value = self.apply_compound_assign_pooled(op, &current, &rhs, pool)?;
                pool.return_buffer(current);
                pool.return_buffer(rhs);
                let name_arc = name_cache.get_or_create(name);
                ctx.assign_var_no_copy(name_arc, value.clone());
                Ok(value)
            }
            AstNode::Output {
                name,
                expr,
                modifier,
            } => {
                let value = self.execute_with_pool_cached(expr, ctx, pool, name_cache)?;
                let name_arc = name_cache.get_or_create(name);
                ctx.assign_var_no_copy(name_arc, value.clone());
                if let Some(modifier) = modifier {
                    ctx.output_modifiers
                        .insert(name.to_string(), modifier.clone());
                }
                Ok(value)
            }
            AstNode::Variable(name) => self.resolve_variable_zero_copy(name, ctx, pool),
            AstNode::Number(val) => {
                let mut buf = pool.get_buffer(ctx.data_len);
                for i in 0..ctx.data_len {
                    buf[i] = *val;
                }
                Ok(buf)
            }
            AstNode::BinaryOp { op, left, right } => {
                let left_val = self.execute_with_pool_cached(left, ctx, pool, name_cache)?;
                let right_val = self.execute_with_pool_cached(right, ctx, pool, name_cache)?;
                let result = self.apply_binary_op_pooled(op, &left_val, &right_val, pool)?;
                pool.return_buffer(left_val);
                pool.return_buffer(right_val);
                Ok(result)
            }
            AstNode::UnaryOp { op, expr } => {
                let val = self.execute_with_pool_cached(expr, ctx, pool, name_cache)?;
                let result = self.apply_unary_op_pooled(op, &val, pool)?;
                pool.return_buffer(val);
                Ok(result)
            }
            AstNode::FunctionCall { name, args } => {
                let arg_values: Result<Vec<Array1<f64>>, FormulaError> = args
                    .iter()
                    .map(|a| self.execute_with_pool_cached(a, ctx, pool, name_cache))
                    .collect();
                let arg_values = arg_values?;
                let result = self.call_function(name, ctx, &arg_values)?;
                for arg in arg_values {
                    pool.return_buffer(arg);
                }
                Ok(result)
            }
            AstNode::IndexAccess { array, index } => {
                let arr_val = self.execute_with_pool_cached(array, ctx, pool, name_cache)?;
                let idx_val = self.execute_with_pool_cached(index, ctx, pool, name_cache)?;
                let mut result = pool.get_buffer(ctx.data_len);
                for i in 0..ctx.data_len {
                    let idx = idx_val[i] as usize;
                    if idx < arr_val.len() {
                        result[i] = arr_val[idx];
                    } else {
                        result[i] = f64::NAN;
                    }
                }
                pool.return_buffer(arr_val);
                pool.return_buffer(idx_val);
                Ok(result)
            }
            AstNode::StringLit(s) => {
                let idx = ctx.string_table.len();
                ctx.string_table.push(s.clone());
                Ok(Array1::from_elem(ctx.data_len, idx as f64))
            }
            AstNode::ParamDecl { .. } => Err(FormulaError::RuntimeError(
                "ParamDecl should be handled at parse time".to_string(),
            )),
            AstNode::DrawText {
                cond,
                price,
                text,
                color,
            } => {
                let cond_val = self.execute_with_pool_cached(cond, ctx, pool, name_cache)?;
                let price_val = self.execute_with_pool_cached(price, ctx, pool, name_cache)?;
                let color_str = color_to_string(color);
                ctx.draw_commands.borrow_mut().add_text(
                    cond_val,
                    price_val,
                    text.to_string(),
                    color_str,
                );
                let result = pool.get_buffer(ctx.data_len);
                Ok(result)
            }
            AstNode::DrawIcon {
                cond,
                price,
                icon,
                color,
            } => {
                let cond_val = self.execute_with_pool_cached(cond, ctx, pool, name_cache)?;
                let price_val = self.execute_with_pool_cached(price, ctx, pool, name_cache)?;
                let icon_val = self.execute_with_pool_cached(icon, ctx, pool, name_cache)?;
                let icon_type = icon_val[0] as i32;
                let color_str = color_to_string(color);
                ctx.draw_commands
                    .borrow_mut()
                    .add_icon(cond_val, price_val, icon_type, color_str);
                pool.return_buffer(icon_val);
                let result = pool.get_buffer(ctx.data_len);
                Ok(result)
            }
            AstNode::StickLine {
                cond,
                price1,
                price2,
                width,
                empty,
                color,
            } => {
                let cond_val = self.execute_with_pool_cached(cond, ctx, pool, name_cache)?;
                let price1_val = self.execute_with_pool_cached(price1, ctx, pool, name_cache)?;
                let price2_val = self.execute_with_pool_cached(price2, ctx, pool, name_cache)?;
                let width_val = self.execute_with_pool_cached(width, ctx, pool, name_cache)?;
                let width_int = width_val[0] as i32;
                let color_str = color_to_string(color);
                ctx.draw_commands.borrow_mut().add_stick(
                    cond_val, price1_val, price2_val, width_int, *empty, color_str,
                );
                pool.return_buffer(width_val);
                let result = pool.get_buffer(ctx.data_len);
                Ok(result)
            }
            AstNode::DrawGeneric {
                command,
                args,
                color,
            } => {
                let mut evaluated = Vec::with_capacity(args.len());
                for arg in args {
                    evaluated.push(self.execute_with_pool_cached(arg, ctx, pool, name_cache)?);
                }
                let color_str = color_to_string(color);
                match command.as_str() {
                    "DRAWLINE" if evaluated.len() >= 5 => {
                        let expand = evaluated[4][0] as i32;
                        pool.return_buffer(evaluated.remove(4));
                        ctx.draw_commands.borrow_mut().add_line(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            expand,
                            color_str,
                        );
                    }
                    "FILLRGN" if evaluated.len() >= 3 => {
                        ctx.draw_commands.borrow_mut().add_fill_rgn(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            color_str,
                        );
                    }
                    "PARTLINE" if evaluated.len() >= 2 => {
                        ctx.draw_commands.borrow_mut().add_part_line(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            color_str,
                        );
                    }
                    "POLYLINE" if evaluated.len() >= 2 => {
                        ctx.draw_commands.borrow_mut().add_poly_line(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            color_str,
                        );
                    }
                    "DRAWGBK" if !evaluated.is_empty() => {
                        ctx.draw_commands
                            .borrow_mut()
                            .add_background(evaluated.remove(0), color_str);
                    }
                    "DRAWSL" if evaluated.len() >= 4 => {
                        ctx.draw_commands.borrow_mut().add_slope_line(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            color_str,
                        );
                    }
                    "DRAWTEXT_FIX" if evaluated.len() >= 3 => {
                        let x = evaluated[0][0];
                        let y = evaluated[1][0];
                        pool.return_buffer(evaluated.remove(0));
                        pool.return_buffer(evaluated.remove(0));
                        pool.return_buffer(evaluated.remove(0));
                        ctx.draw_commands
                            .borrow_mut()
                            .add_text_fix(x, y, String::new(), color_str);
                    }
                    "DRAWNUMBER" if evaluated.len() >= 4 => {
                        let precision = evaluated[3][0] as i32;
                        pool.return_buffer(evaluated.remove(3));
                        ctx.draw_commands.borrow_mut().add_number(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            precision,
                            color_str,
                        );
                    }
                    "VERTLINE" if !evaluated.is_empty() => {
                        ctx.draw_commands
                            .borrow_mut()
                            .add_vert_line(evaluated.remove(0), color_str);
                    }
                    _ => {
                        for buf in evaluated {
                            pool.return_buffer(buf);
                        }
                    }
                }
                let result = pool.get_buffer(ctx.data_len);
                Ok(result)
            }
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond_val = self.execute_with_pool_cached(cond, ctx, pool, name_cache)?;
                let then_val = self.execute_with_pool_cached(then_branch, ctx, pool, name_cache)?;
                let else_val = self.execute_with_pool_cached(else_branch, ctx, pool, name_cache)?;
                let mut result = pool.get_buffer(ctx.data_len);
                for i in 0..ctx.data_len {
                    if cond_val[i] > 0.0 {
                        result[i] = then_val[i];
                    } else {
                        result[i] = else_val[i];
                    }
                }
                pool.return_buffer(cond_val);
                pool.return_buffer(then_val);
                pool.return_buffer(else_val);
                Ok(result)
            }
            AstNode::ForLoop {
                var,
                start,
                end,
                body,
            } => {
                let start_val = self.execute_with_pool_cached(start, ctx, pool, name_cache)?;
                let end_val = self.execute_with_pool_cached(end, ctx, pool, name_cache)?;
                let start_i = start_val[0] as i64;
                let end_i = end_val[0] as i64;
                pool.return_buffer(start_val);
                pool.return_buffer(end_val);
                let mut result = pool.get_buffer(ctx.data_len);
                let max_iterations = 10000i64;
                let var_arc = name_cache.get_or_create(var);
                for (count, i) in (start_i..=end_i).enumerate() {
                    if count as i64 >= max_iterations {
                        return Err(FormulaError::RuntimeError(format!(
                            "FOR loop exceeded maximum iterations ({})",
                            max_iterations
                        )));
                    }
                    let mut loop_var = pool.get_buffer(ctx.data_len);
                    for j in 0..ctx.data_len {
                        loop_var[j] = i as f64;
                    }
                    ctx.assign_var_no_copy(var_arc.clone(), loop_var);
                    for stmt in body {
                        let new_result =
                            self.execute_with_pool_cached(stmt, ctx, pool, name_cache)?;
                        pool.return_buffer(result);
                        result = new_result;
                    }
                }
                Ok(result)
            }
            AstNode::WhileLoop { cond, body } => {
                let mut result = pool.get_buffer(ctx.data_len);
                let max_iterations = 10000usize;
                for _ in 0..max_iterations {
                    let cond_val = self.execute_with_pool_cached(cond, ctx, pool, name_cache)?;
                    let should_break = !cond_val.iter().any(|&v| v > 0.0);
                    pool.return_buffer(cond_val);
                    if should_break {
                        break;
                    }
                    for stmt in body {
                        let new_result =
                            self.execute_with_pool_cached(stmt, ctx, pool, name_cache)?;
                        pool.return_buffer(result);
                        result = new_result;
                    }
                }
                Ok(result)
            }
        }
    }

    fn execute_with_pool(
        &self,
        ast: &AstNode,
        ctx: &mut FormulaContext,
        pool: &mut BufferPool,
    ) -> Result<Array1<f64>, FormulaError> {
        match ast {
            AstNode::Statements(stmts) => {
                let mut result = pool.get_buffer(ctx.data_len);
                for stmt in stmts {
                    let new_result = self.execute_with_pool(stmt, ctx, pool)?;
                    pool.return_buffer(result);
                    result = new_result;
                }
                Ok(result)
            }
            AstNode::Assignment { name, expr } => {
                let value = self.execute_with_pool(expr, ctx, pool)?;
                Ok(ctx.assign_var(name, value))
            }
            AstNode::CompoundAssignment { name, op, expr } => {
                let current = self.resolve_variable_zero_copy(name, ctx, pool)?;
                let rhs = self.execute_with_pool(expr, ctx, pool)?;
                let value = self.apply_compound_assign_pooled(op, &current, &rhs, pool)?;
                pool.return_buffer(current);
                pool.return_buffer(rhs);
                Ok(ctx.assign_var(name, value))
            }
            AstNode::Output {
                name,
                expr,
                modifier,
            } => {
                let value = self.execute_with_pool(expr, ctx, pool)?;
                let value = ctx.assign_var(name, value);
                if let Some(modifier) = modifier {
                    ctx.output_modifiers
                        .insert(name.to_string(), modifier.clone());
                }
                Ok(value)
            }
            AstNode::Variable(name) => self.resolve_variable_zero_copy(name, ctx, pool),
            AstNode::Number(val) => {
                let mut buf = pool.get_buffer(ctx.data_len);
                for i in 0..ctx.data_len {
                    buf[i] = *val;
                }
                Ok(buf)
            }
            AstNode::BinaryOp { op, left, right } => {
                let left_val = self.execute_with_pool(left, ctx, pool)?;
                let right_val = self.execute_with_pool(right, ctx, pool)?;
                let result = self.apply_binary_op_pooled(op, &left_val, &right_val, pool)?;
                pool.return_buffer(left_val);
                pool.return_buffer(right_val);
                Ok(result)
            }
            AstNode::UnaryOp { op, expr } => {
                let val = self.execute_with_pool(expr, ctx, pool)?;
                let result = self.apply_unary_op_pooled(op, &val, pool)?;
                pool.return_buffer(val);
                Ok(result)
            }
            AstNode::FunctionCall { name, args } => {
                let arg_values: Result<Vec<Array1<f64>>, FormulaError> = args
                    .iter()
                    .map(|a| self.execute_with_pool(a, ctx, pool))
                    .collect();
                let arg_values = arg_values?;
                let result = self.call_function(name, ctx, &arg_values)?;
                for arg in arg_values {
                    pool.return_buffer(arg);
                }
                Ok(result)
            }
            AstNode::IndexAccess { array, index } => {
                let arr_val = self.execute_with_pool(array, ctx, pool)?;
                let idx_val = self.execute_with_pool(index, ctx, pool)?;
                let mut result = pool.get_buffer(ctx.data_len);
                for i in 0..ctx.data_len {
                    let idx = idx_val[i] as usize;
                    if idx < arr_val.len() {
                        result[i] = arr_val[idx];
                    } else {
                        result[i] = f64::NAN;
                    }
                }
                pool.return_buffer(arr_val);
                pool.return_buffer(idx_val);
                Ok(result)
            }
            AstNode::StringLit(s) => {
                let idx = ctx.string_table.len();
                ctx.string_table.push(s.clone());
                Ok(Array1::from_elem(ctx.data_len, idx as f64))
            }
            AstNode::ParamDecl { .. } => Err(FormulaError::RuntimeError(
                "ParamDecl should be handled at parse time".to_string(),
            )),
            AstNode::DrawText {
                cond,
                price,
                text,
                color,
            } => {
                let cond_val = self.execute_with_pool(cond, ctx, pool)?;
                let price_val = self.execute_with_pool(price, ctx, pool)?;
                let color_str = color_to_string(color);
                ctx.draw_commands.borrow_mut().add_text(
                    cond_val,
                    price_val,
                    text.to_string(),
                    color_str,
                );
                let result = pool.get_buffer(ctx.data_len);
                Ok(result)
            }
            AstNode::DrawIcon {
                cond,
                price,
                icon,
                color,
            } => {
                let cond_val = self.execute_with_pool(cond, ctx, pool)?;
                let price_val = self.execute_with_pool(price, ctx, pool)?;
                let icon_val = self.execute_with_pool(icon, ctx, pool)?;
                let icon_type = icon_val[0] as i32;
                let color_str = color_to_string(color);
                ctx.draw_commands
                    .borrow_mut()
                    .add_icon(cond_val, price_val, icon_type, color_str);
                pool.return_buffer(icon_val);
                let result = pool.get_buffer(ctx.data_len);
                Ok(result)
            }
            AstNode::StickLine {
                cond,
                price1,
                price2,
                width,
                empty,
                color,
            } => {
                let cond_val = self.execute_with_pool(cond, ctx, pool)?;
                let price1_val = self.execute_with_pool(price1, ctx, pool)?;
                let price2_val = self.execute_with_pool(price2, ctx, pool)?;
                let width_val = self.execute_with_pool(width, ctx, pool)?;
                let width_int = width_val[0] as i32;
                let color_str = color_to_string(color);
                ctx.draw_commands.borrow_mut().add_stick(
                    cond_val, price1_val, price2_val, width_int, *empty, color_str,
                );
                pool.return_buffer(width_val);
                let result = pool.get_buffer(ctx.data_len);
                Ok(result)
            }
            AstNode::DrawGeneric {
                command,
                args,
                color,
            } => {
                let mut evaluated = Vec::with_capacity(args.len());
                for arg in args {
                    evaluated.push(self.execute_with_pool(arg, ctx, pool)?);
                }
                let color_str = color_to_string(color);
                match command.as_str() {
                    "DRAWLINE" if evaluated.len() >= 5 => {
                        let expand = evaluated[4][0] as i32;
                        pool.return_buffer(evaluated.remove(4));
                        ctx.draw_commands.borrow_mut().add_line(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            expand,
                            color_str,
                        );
                    }
                    "FILLRGN" if evaluated.len() >= 3 => {
                        ctx.draw_commands.borrow_mut().add_fill_rgn(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            color_str,
                        );
                    }
                    "PARTLINE" if evaluated.len() >= 2 => {
                        ctx.draw_commands.borrow_mut().add_part_line(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            color_str,
                        );
                    }
                    "POLYLINE" if evaluated.len() >= 2 => {
                        ctx.draw_commands.borrow_mut().add_poly_line(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            color_str,
                        );
                    }
                    "DRAWGBK" if !evaluated.is_empty() => {
                        ctx.draw_commands
                            .borrow_mut()
                            .add_background(evaluated.remove(0), color_str);
                    }
                    "DRAWSL" if evaluated.len() >= 4 => {
                        ctx.draw_commands.borrow_mut().add_slope_line(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            color_str,
                        );
                    }
                    "DRAWTEXT_FIX" if evaluated.len() >= 3 => {
                        let x = evaluated[0][0];
                        let y = evaluated[1][0];
                        pool.return_buffer(evaluated.remove(0));
                        pool.return_buffer(evaluated.remove(0));
                        pool.return_buffer(evaluated.remove(0));
                        ctx.draw_commands
                            .borrow_mut()
                            .add_text_fix(x, y, String::new(), color_str);
                    }
                    "DRAWNUMBER" if evaluated.len() >= 4 => {
                        let precision = evaluated[3][0] as i32;
                        pool.return_buffer(evaluated.remove(3));
                        ctx.draw_commands.borrow_mut().add_number(
                            evaluated.remove(0),
                            evaluated.remove(0),
                            evaluated.remove(0),
                            precision,
                            color_str,
                        );
                    }
                    "VERTLINE" if !evaluated.is_empty() => {
                        ctx.draw_commands
                            .borrow_mut()
                            .add_vert_line(evaluated.remove(0), color_str);
                    }
                    _ => {
                        for buf in evaluated {
                            pool.return_buffer(buf);
                        }
                    }
                }
                let result = pool.get_buffer(ctx.data_len);
                Ok(result)
            }
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond_val = self.execute_with_pool(cond, ctx, pool)?;
                let then_val = self.execute_with_pool(then_branch, ctx, pool)?;
                let else_val = self.execute_with_pool(else_branch, ctx, pool)?;
                let mut result = pool.get_buffer(ctx.data_len);
                for i in 0..ctx.data_len {
                    if cond_val[i] > 0.0 {
                        result[i] = then_val[i];
                    } else {
                        result[i] = else_val[i];
                    }
                }
                pool.return_buffer(cond_val);
                pool.return_buffer(then_val);
                pool.return_buffer(else_val);
                Ok(result)
            }
            AstNode::ForLoop {
                var,
                start,
                end,
                body,
            } => {
                let start_val = self.execute_with_pool(start, ctx, pool)?;
                let end_val = self.execute_with_pool(end, ctx, pool)?;
                let start_i = start_val[0] as i64;
                let end_i = end_val[0] as i64;
                pool.return_buffer(start_val);
                pool.return_buffer(end_val);
                let mut result = pool.get_buffer(ctx.data_len);
                let max_iterations = 10000i64;
                for (count, i) in (start_i..=end_i).enumerate() {
                    if count as i64 >= max_iterations {
                        return Err(FormulaError::RuntimeError(format!(
                            "FOR loop exceeded maximum iterations ({})",
                            max_iterations
                        )));
                    }
                    let mut loop_var = pool.get_buffer(ctx.data_len);
                    for j in 0..ctx.data_len {
                        loop_var[j] = i as f64;
                    }
                    ctx.variables.insert(Arc::from(var.to_string()), loop_var);
                    for stmt in body {
                        let new_result = self.execute_with_pool(stmt, ctx, pool)?;
                        pool.return_buffer(result);
                        result = new_result;
                    }
                }
                Ok(result)
            }
            AstNode::WhileLoop { cond, body } => {
                let mut result = pool.get_buffer(ctx.data_len);
                let max_iterations = 10000usize;
                for _ in 0..max_iterations {
                    let cond_val = self.execute_with_pool(cond, ctx, pool)?;
                    let should_break = !cond_val.iter().any(|&v| v > 0.0);
                    pool.return_buffer(cond_val);
                    if should_break {
                        break;
                    }
                    for stmt in body {
                        let new_result = self.execute_with_pool(stmt, ctx, pool)?;
                        pool.return_buffer(result);
                        result = new_result;
                    }
                }
                Ok(result)
            }
        }
    }

    fn copy_view_to_pool(view: ndarray::ArrayView1<f64>, pool: &mut BufferPool) -> Array1<f64> {
        let mut buf = pool.get_buffer(view.len());
        let out = buf.as_slice_mut().unwrap();
        let src = view.as_slice().unwrap();
        out[..view.len()].copy_from_slice(&src[..view.len()]);
        buf
    }

    fn resolve_variable_zero_copy(
        &self,
        name: &str,
        ctx: &FormulaContext,
        pool: &mut BufferPool,
    ) -> Result<Array1<f64>, FormulaError> {
        match classify_builtin_var(name) {
            Some(BuiltinVar::Close) => Ok(Self::copy_view_to_pool(ctx.close_view(), pool)),
            Some(BuiltinVar::High) => Ok(Self::copy_view_to_pool(ctx.high_view(), pool)),
            Some(BuiltinVar::Low) => Ok(Self::copy_view_to_pool(ctx.low_view(), pool)),
            Some(BuiltinVar::Open) => Ok(Self::copy_view_to_pool(ctx.open_view(), pool)),
            Some(BuiltinVar::Volume) => Ok(Self::copy_view_to_pool(ctx.volume_view(), pool)),
            Some(BuiltinVar::Amount) => match &ctx.amount {
                Some(amt) => Ok(Self::copy_view_to_pool(amt.view(), pool)),
                None => Err(FormulaError::RuntimeError(
                    "AMOUNT data not available".to_string(),
                )),
            },
            Some(BuiltinVar::BarsCount) => {
                let mut buf = pool.get_buffer(ctx.data_len);
                let s = buf.as_slice_mut().unwrap();
                for i in 0..ctx.data_len {
                    s[i] = ctx.data_len as f64;
                }
                Ok(buf)
            }
            Some(BuiltinVar::BarPos) => {
                let mut buf = pool.get_buffer(ctx.data_len);
                let s = buf.as_slice_mut().unwrap();
                for i in 0..ctx.data_len {
                    s[i] = (i + 1) as f64;
                }
                Ok(buf)
            }
            Some(BuiltinVar::Capital) => {
                let mut buf = pool.get_buffer(ctx.data_len);
                let val = ctx.capital.unwrap_or(f64::NAN);
                let s = buf.as_slice_mut().unwrap();
                for i in 0..ctx.data_len {
                    s[i] = val;
                }
                Ok(buf)
            }
            Some(BuiltinVar::DrawNull) => {
                let mut buf = pool.get_buffer(ctx.data_len);
                let s = buf.as_slice_mut().unwrap();
                for i in 0..ctx.data_len {
                    s[i] = f64::NAN;
                }
                Ok(buf)
            }
            None => ctx
                .variables
                .get(name)
                .map(FormulaContext::copy_array)
                .ok_or_else(|| FormulaError::RuntimeError(format!("Unknown variable: {}", name))),
        }
    }

    fn apply_binary_op_pooled(
        &self,
        op: &BinaryOperator,
        left: &Array1<f64>,
        right: &Array1<f64>,
        pool: &mut BufferPool,
    ) -> Result<Array1<f64>, FormulaError> {
        let len = left.len();
        let mut result = pool.get_buffer(len);
        let l = left.as_slice().unwrap();
        let r = right.as_slice().unwrap();
        let out = result.as_slice_mut().unwrap();
        match op {
            BinaryOperator::Add => SimdOps::add(l, r, out),
            BinaryOperator::Sub => SimdOps::sub(l, r, out),
            BinaryOperator::Mul => SimdOps::mul(l, r, out),
            BinaryOperator::Div => SimdOps::div(l, r, out),
            BinaryOperator::Mod => SimdOps::simd_mod(l, r, out),
            BinaryOperator::Pow => SimdOps::simd_pow(l, r, out),
            BinaryOperator::Gt => SimdOps::gt(l, r, out),
            BinaryOperator::Lt => SimdOps::lt(l, r, out),
            BinaryOperator::Gte => SimdOps::gte(l, r, out),
            BinaryOperator::Lte => SimdOps::lte(l, r, out),
            BinaryOperator::Eq => SimdOps::eq(l, r, out),
            BinaryOperator::Neq => SimdOps::neq(l, r, out),
            BinaryOperator::And => SimdOps::logical_and(l, r, out),
            BinaryOperator::Or => SimdOps::logical_or(l, r, out),
            BinaryOperator::Xor => SimdOps::logical_xor(l, r, out),
            BinaryOperator::StringConcat => {
                pool.return_buffer(result);
                return Err(FormulaError::InvalidOperation(
                    "String concatenation (&) is not supported for numeric values. Use STRCAT() function instead.".to_string()
                ));
            }
        }
        Ok(result)
    }

    fn apply_unary_op_pooled(
        &self,
        op: &UnaryOperator,
        val: &Array1<f64>,
        pool: &mut BufferPool,
    ) -> Result<Array1<f64>, FormulaError> {
        let len = val.len();
        let mut result = pool.get_buffer(len);
        match op {
            UnaryOperator::Not => {
                SimdOps::logical_not(val.as_slice().unwrap(), result.as_slice_mut().unwrap());
            }
            UnaryOperator::Neg => {
                for i in 0..len {
                    result[i] = -val[i];
                }
            }
        }
        Ok(result)
    }

    fn apply_compound_assign_pooled(
        &self,
        op: &CompoundAssignOp,
        current: &Array1<f64>,
        rhs: &Array1<f64>,
        pool: &mut BufferPool,
    ) -> Result<Array1<f64>, FormulaError> {
        let len = current.len();
        let mut result = pool.get_buffer(len);
        let c = current.as_slice().unwrap();
        let r = rhs.as_slice().unwrap();
        let out = result.as_slice_mut().unwrap();
        match op {
            CompoundAssignOp::AddAssign => SimdOps::add(c, r, out),
            CompoundAssignOp::SubAssign => SimdOps::sub(c, r, out),
            CompoundAssignOp::MulAssign => SimdOps::mul(c, r, out),
            CompoundAssignOp::DivAssign => SimdOps::div(c, r, out),
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::parser::parse_formula;

    fn make_ctx(len: usize) -> FormulaContext {
        let open = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.1).collect());
        let high = Array1::from_vec((0..len).map(|i| 11.0 + i as f64 * 0.2).collect());
        let low = Array1::from_vec((0..len).map(|i| 9.0 + i as f64 * 0.1).collect());
        let close = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.15).collect());
        let volume = Array1::from_vec((0..len).map(|i| 1000.0 + i as f64 * 10.0).collect());
        FormulaContext::new(open, high, low, close, volume, None)
    }

    fn execute_formula(
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        let executor = FormulaExecutor::new();
        executor.execute(&ast, ctx)
    }

    #[test]
    fn test_execute_number() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("42", &mut ctx).unwrap();
        assert_eq!(result.len(), 5);
        for i in 0..5 {
            assert!((result[i] - 42.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_addition() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("10 + 20", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 30.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_subtraction() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("100 - 50", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 50.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_multiplication() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("3 * 4", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 12.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_division() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("100 / 4", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 25.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_division_by_zero() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("10 / 0", &mut ctx).unwrap();
        for i in 0..5 {
            assert!(result[i].is_nan());
        }
    }

    #[test]
    fn test_execute_variable_close() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("CLOSE", &mut ctx).unwrap();
        assert_eq!(result.len(), 5);
        for i in 0..5 {
            let expected = 10.0 + i as f64 * 0.15;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_variable_high() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("H", &mut ctx).unwrap();
        for i in 0..5 {
            let expected = 11.0 + i as f64 * 0.2;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_variable_low() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("L", &mut ctx).unwrap();
        for i in 0..5 {
            let expected = 9.0 + i as f64 * 0.1;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_variable_volume() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("V", &mut ctx).unwrap();
        for i in 0..5 {
            let expected = 1000.0 + i as f64 * 10.0;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_variable_close_plus_open() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("C + O", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            let expected = close_val + open_val;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_comparison_gt() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("CLOSE > 10.5", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let expected = if close_val > 10.5 { 1.0 } else { 0.0 };
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_comparison_lt() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("CLOSE < 10.5", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let expected = if close_val < 10.5 { 1.0 } else { 0.0 };
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_and_operator() {
        let mut ctx = make_ctx(10);
        let result = execute_formula("CLOSE > 10.5 AND VOLUME > 1050", &mut ctx).unwrap();
        for i in 0..10 {
            let close_val = 10.0 + i as f64 * 0.15;
            let volume_val = 1000.0 + i as f64 * 10.0;
            let expected = if close_val > 10.5 && volume_val > 1050.0 {
                1.0
            } else {
                0.0
            };
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_or_operator() {
        let mut ctx = make_ctx(10);
        let result = execute_formula("CLOSE > 12.0 OR VOLUME < 1050.0", &mut ctx).unwrap();
        for i in 0..10 {
            let close_val = 10.0 + i as f64 * 0.15;
            let volume_val = 1000.0 + i as f64 * 10.0;
            let expected = if close_val > 12.0 || volume_val < 1050.0 {
                1.0
            } else {
                0.0
            };
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_assignment() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("UP := CLOSE + 1", &mut ctx).unwrap();
        for i in 0..5 {
            let expected = 10.0 + i as f64 * 0.15 + 1.0;
            assert!((result[i] - expected).abs() < 1e-10);
        }
        assert!(ctx.variables.contains_key("UP"));
    }

    #[test]
    fn test_execute_output() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("RESULT: CLOSE * 2", &mut ctx).unwrap();
        for i in 0..5 {
            let expected = (10.0 + i as f64 * 0.15) * 2.0;
            assert!((result[i] - expected).abs() < 1e-10);
        }
        assert!(ctx.variables.contains_key("RESULT"));
    }

    #[test]
    fn test_execute_function_call() {
        let mut ctx = make_ctx(10);
        let result = execute_formula("MA(CLOSE, 3)", &mut ctx).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(!result[2].is_nan());
    }

    #[test]
    fn test_execute_nested_function_call() {
        let mut ctx = make_ctx(10);
        let result = execute_formula("EMA(MA(CLOSE, 3), 5)", &mut ctx).unwrap();
        assert!(result[0].is_nan());
        assert!(result.len() == 10);
    }

    #[test]
    fn test_execute_multiple_statements() {
        let mut ctx = make_ctx(10);
        let source = "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); MA5 > MA10";
        let result = execute_formula(source, &mut ctx).unwrap();
        assert!(ctx.variables.contains_key("MA5"));
        assert!(ctx.variables.contains_key("MA10"));
        assert!(result.len() == 10);
    }

    #[test]
    fn test_execute_unary_not() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("NOT(CLOSE > 20)", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let expected = if close_val > 20.0 { 0.0 } else { 1.0 };
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_unary_neg() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("-100", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - (-100.0)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_power() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("2 ^ 10", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 1024.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_mod() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("10 % 3", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_operator_precedence() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("2 + 3 * 4", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 14.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_parentheses() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("(2 + 3) * 4", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 20.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_if_function() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("IF(CLOSE > 10.5, 100, 0)", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let expected = if close_val > 10.5 { 100.0 } else { 0.0 };
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_ref_function() {
        let mut ctx = make_ctx(10);
        let result = execute_formula("REF(CLOSE, 1)", &mut ctx).unwrap();
        assert!(result[0].is_nan());
        for i in 1..10 {
            let expected = 10.0 + (i - 1) as f64 * 0.15;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_complex_formula() {
        let mut ctx = make_ctx(30);
        let source = r#"
            MA5 := MA(CLOSE, 5);
            MA10 := MA(CLOSE, 10);
            GOLDEN := MA5 > MA10;
            SIGNAL: IF(GOLDEN, 1, 0)
        "#;
        let result = execute_formula(source, &mut ctx).unwrap();
        assert!(ctx.variables.contains_key("MA5"));
        assert!(ctx.variables.contains_key("MA10"));
        assert!(ctx.variables.contains_key("GOLDEN"));
        assert!(result.len() == 30);
        for i in 0..30 {
            assert!(result[i] == 0.0 || result[i] == 1.0);
        }
    }

    #[test]
    fn test_execute_barscount() {
        let mut ctx = make_ctx(10);
        let result = execute_formula("BARSCOUNT", &mut ctx).unwrap();
        for i in 0..10 {
            assert!((result[i] - 10.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_barpos() {
        let mut ctx = make_ctx(10);
        let result = execute_formula("BARPOS", &mut ctx).unwrap();
        for i in 0..10 {
            assert!((result[i] - (i as f64 + 1.0)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_cross_function() {
        let mut ctx = make_ctx(10);
        let source = "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); CROSS(MA5, MA10)";
        let result = execute_formula(source, &mut ctx).unwrap();
        assert!(result.len() == 10);
    }

    #[test]
    fn test_execute_count_function() {
        let mut ctx = make_ctx(10);
        let result = execute_formula("COUNT(CLOSE > OPEN, 5)", &mut ctx).unwrap();
        assert!(result.len() == 10);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert!(result[3].is_nan());
        assert!(!result[4].is_nan());
    }

    #[test]
    fn test_execute_variable_in_expression() {
        let mut ctx = make_ctx(5);
        let source = "UP := CLOSE + 1; UP * 2";
        let result = execute_formula(source, &mut ctx).unwrap();
        for i in 0..5 {
            let expected = (10.0 + i as f64 * 0.15 + 1.0) * 2.0;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_comparison_eq() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("CLOSE == CLOSE", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_comparison_neq() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("CLOSE != OPEN", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            let expected = if (close_val - open_val).abs() >= 1e-10 {
                1.0
            } else {
                0.0
            };
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_gte_lte() {
        let mut ctx = make_ctx(5);
        let result_gt = execute_formula("CLOSE >= 10.0", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result_gt[i] - 1.0).abs() < 1e-10);
        }

        let result_lt = execute_formula("CLOSE <= 20.0", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result_lt[i] - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_unknown_variable() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("UNKNOWN_VAR", &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_unknown_function() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("UNKNOWN_FUNC(CLOSE)", &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_max_function() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("MAX(CLOSE, OPEN)", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            let expected = close_val.max(open_val);
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_min_function() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("MIN(CLOSE, OPEN)", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            let expected = close_val.min(open_val);
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_hhv_function() {
        let mut ctx = make_ctx(10);
        let result = execute_formula("HHV(HIGH, 5)", &mut ctx).unwrap();
        assert!(result.len() == 10);
    }

    #[test]
    fn test_execute_llv_function() {
        let mut ctx = make_ctx(10);
        let result = execute_formula("LLV(LOW, 5)", &mut ctx).unwrap();
        assert!(result.len() == 10);
    }

    #[test]
    fn test_execute_sum_function() {
        let mut ctx = make_ctx(10);
        let result = execute_formula("SUM(VOLUME, 3)", &mut ctx).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(!result[2].is_nan());
        let expected: f64 = 1000.0 + 1010.0 + 1020.0;
        assert!((result[2] - expected).abs() < 1e-10);
    }

    #[test]
    fn test_execute_sqrt_function() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("SQRT(16)", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 4.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_abs_function() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("ABS(-10)", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 10.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_formula_with_amount_missing() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("AMOUNT", &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_multiple_variables() {
        let mut ctx = make_ctx(5);
        let source = "A := CLOSE + 1; B := A * 2; B";
        let result = execute_formula(source, &mut ctx).unwrap();
        for i in 0..5 {
            let expected = (10.0 + i as f64 * 0.15 + 1.0) * 2.0;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_complex_trading_formula() {
        let mut ctx = make_ctx(50);
        let source = r#"
            SHORT := 12;
            LONG := 26;
            DIF := EMA(CLOSE, SHORT) - EMA(CLOSE, LONG);
            DEA := EMA(DIF, 9);
            MACD := (DIF - DEA) * 2;
            BUY_SIGNAL := CROSS(DIF, DEA);
            MACD: MACD
        "#;
        let result = execute_formula(source, &mut ctx).unwrap();
        assert!(ctx.variables.contains_key("DIF"));
        assert!(ctx.variables.contains_key("DEA"));
        assert!(ctx.variables.contains_key("MACD"));
        assert!(ctx.variables.contains_key("BUY_SIGNAL"));
        assert!(result.len() == 50);
    }

    #[test]
    fn test_execute_barslast_function() {
        let mut ctx = make_ctx(10);
        let result = execute_formula("BARSLAST(CLOSE > 11)", &mut ctx).unwrap();
        assert!(result.len() == 10);
    }

    #[test]
    fn test_execute_filter_function() {
        let mut ctx = make_ctx(10);
        let result = execute_formula("FILTER(CLOSE > OPEN, 3)", &mut ctx).unwrap();
        assert!(result.len() == 10);
    }

    #[test]
    fn test_execute_string_concat() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("1 & 2", &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_string_concat_with_variables() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("10 & 5", &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_xor_true_false() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("1 XOR 0", &mut ctx).unwrap();
        for i in 0..5 {
            assert!(
                (result[i] - 1.0).abs() < 1e-10,
                "XOR at {}: {}",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_execute_xor_true_true() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("1 XOR 1", &mut ctx).unwrap();
        for i in 0..5 {
            assert!(
                (result[i] - 0.0).abs() < 1e-10,
                "XOR at {}: {}",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_execute_xor_false_false() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("0 XOR 0", &mut ctx).unwrap();
        for i in 0..5 {
            assert!(
                (result[i] - 0.0).abs() < 1e-10,
                "XOR at {}: {}",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_execute_compound_add_assign() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("X := 10; X += 5; X", &mut ctx).unwrap();
        for i in 0..5 {
            assert!(
                (result[i] - 15.0).abs() < 1e-10,
                "AddAssign at {}: {}",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_execute_compound_sub_assign() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("X := 10; X -= 3; X", &mut ctx).unwrap();
        for i in 0..5 {
            assert!(
                (result[i] - 7.0).abs() < 1e-10,
                "SubAssign at {}: {}",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_execute_compound_mul_assign() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("X := 10; X *= 3; X", &mut ctx).unwrap();
        for i in 0..5 {
            assert!(
                (result[i] - 30.0).abs() < 1e-10,
                "MulAssign at {}: {}",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_execute_compound_div_assign() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("X := 10; X /= 2; X", &mut ctx).unwrap();
        for i in 0..5 {
            assert!(
                (result[i] - 5.0).abs() < 1e-10,
                "DivAssign at {}: {}",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_execute_index_access() {
        let mut ctx = make_ctx(10);
        let result = execute_formula("MA(CLOSE, 5)[2]", &mut ctx).unwrap();
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_execute_xor_with_conditions() {
        let mut ctx = make_ctx(10);
        let result = execute_formula("(CLOSE > 10.5) XOR (VOLUME > 1050)", &mut ctx).unwrap();
        for i in 0..10 {
            let close_val = 10.0 + i as f64 * 0.15;
            let volume_val = 1000.0 + i as f64 * 10.0;
            let left = close_val > 10.5;
            let right = volume_val > 1050.0;
            let expected = if left != right { 1.0 } else { 0.0 };
            assert!(
                (result[i] - expected).abs() < 1e-10,
                "XOR at {}: expected {}, got {}",
                i,
                expected,
                result[i]
            );
        }
    }

    #[test]
    fn test_execute_compound_assign_with_variable() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("X := CLOSE; X += 1; X", &mut ctx).unwrap();
        for i in 0..5 {
            let expected = 10.0 + i as f64 * 0.15 + 1.0;
            assert!(
                (result[i] - expected).abs() < 1e-10,
                "CompoundAssign at {}: {}",
                i,
                result[i]
            );
        }
    }

    #[test]
    fn test_execute_if_then_else_mixed_conditions() {
        let mut ctx = make_ctx(20);
        let result = execute_formula("IF(CLOSE > 10.5, 100, 0)", &mut ctx).unwrap();
        for i in 0..20 {
            let close_val = 10.0 + i as f64 * 0.15;
            let expected = if close_val > 10.5 { 100.0 } else { 0.0 };
            assert!(
                (result[i] - expected).abs() < 1e-10,
                "IfThenElse at {}: expected {}, got {}",
                i,
                expected,
                result[i]
            );
        }
    }

    #[test]
    fn test_execute_while_loop_per_element() {
        let mut ctx = make_ctx(10);
        let source = "X := 0; WHILE CLOSE > X DO X := X + 1 END; X";
        let result = execute_formula(source, &mut ctx);
        assert!(result.is_ok(), "WhileLoop failed: {:?}", result.err());
        let result = result.unwrap();
        assert!(result[0] > 0.0);
    }

    #[test]
    fn test_execute_string_concat_returns_error() {
        let mut ctx = make_ctx(5);
        let result = execute_formula("3 & 7", &mut ctx);
        assert!(result.is_err());
        match result {
            Err(FormulaError::InvalidOperation(msg)) => {
                assert!(msg.contains("STRCAT"));
            }
            _ => panic!("Expected InvalidOperation error"),
        }
    }

    fn execute_formula_zero_copy(
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        let executor = FormulaExecutor::new();
        executor.execute_zero_copy(&ast, ctx)
    }

    #[test]
    fn test_zero_copy_constant() {
        let mut ctx = make_ctx(5);
        let result = execute_formula_zero_copy("42", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 42.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_zero_copy_addition() {
        let mut ctx = make_ctx(5);
        let result = execute_formula_zero_copy("10 + 20", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 30.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_zero_copy_variable_close() {
        let mut ctx = make_ctx(5);
        let result = execute_formula_zero_copy("CLOSE", &mut ctx).unwrap();
        for i in 0..5 {
            let expected = 10.0 + i as f64 * 0.15;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_zero_copy_close_plus_open() {
        let mut ctx = make_ctx(5);
        let result = execute_formula_zero_copy("C + O", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            assert!((result[i] - (close_val + open_val)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_zero_copy_comparison_gt() {
        let mut ctx = make_ctx(5);
        let result = execute_formula_zero_copy("CLOSE > 10.5", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let expected = if close_val > 10.5 { 1.0 } else { 0.0 };
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_zero_copy_assignment() {
        let mut ctx = make_ctx(5);
        let result = execute_formula_zero_copy("UP := CLOSE + 1", &mut ctx).unwrap();
        for i in 0..5 {
            let expected = 10.0 + i as f64 * 0.15 + 1.0;
            assert!((result[i] - expected).abs() < 1e-10);
        }
        assert!(ctx.variables.contains_key("UP"));
    }

    #[test]
    fn test_zero_copy_function_call() {
        let mut ctx = make_ctx(10);
        let result = execute_formula_zero_copy("MA(CLOSE, 3)", &mut ctx).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(!result[2].is_nan());
    }

    #[test]
    fn test_zero_copy_multiple_statements() {
        let mut ctx = make_ctx(10);
        let source = "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); MA5 > MA10";
        let result = execute_formula_zero_copy(source, &mut ctx).unwrap();
        assert!(ctx.variables.contains_key("MA5"));
        assert!(ctx.variables.contains_key("MA10"));
        assert!(result.len() == 10);
    }

    #[test]
    fn test_zero_copy_matches_normal_execution() {
        let mut ctx1 = make_ctx(30);
        let mut ctx2 = make_ctx(30);
        let source = "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); MA5 > MA10";
        let normal = execute_formula(source, &mut ctx1).unwrap();
        let zero_copy = execute_formula_zero_copy(source, &mut ctx2).unwrap();
        for i in 0..30 {
            if normal[i].is_nan() {
                assert!(zero_copy[i].is_nan());
            } else {
                assert!(
                    (normal[i] - zero_copy[i]).abs() < 1e-10,
                    "Mismatch at {}: {} vs {}",
                    i,
                    normal[i],
                    zero_copy[i]
                );
            }
        }
    }

    #[test]
    fn test_zero_copy_complex_formula_matches() {
        let mut ctx1 = make_ctx(50);
        let mut ctx2 = make_ctx(50);
        let source = r#"
            SHORT := 12;
            LONG := 26;
            DIF := EMA(CLOSE, SHORT) - EMA(CLOSE, LONG);
            DEA := EMA(DIF, 9);
            MACD := (DIF - DEA) * 2;
            MACD: MACD
        "#;
        let normal = execute_formula(source, &mut ctx1).unwrap();
        let zero_copy = execute_formula_zero_copy(source, &mut ctx2).unwrap();
        for i in 0..50 {
            if normal[i].is_nan() {
                assert!(zero_copy[i].is_nan());
            } else {
                assert!(
                    (normal[i] - zero_copy[i]).abs() < 1e-10,
                    "Mismatch at {}: {} vs {}",
                    i,
                    normal[i],
                    zero_copy[i]
                );
            }
        }
    }

    #[test]
    fn test_zero_copy_unary_neg() {
        let mut ctx = make_ctx(5);
        let result = execute_formula_zero_copy("-100", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - (-100.0)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_zero_copy_division_by_zero() {
        let mut ctx = make_ctx(5);
        let result = execute_formula_zero_copy("10 / 0", &mut ctx).unwrap();
        for i in 0..5 {
            assert!(result[i].is_nan());
        }
    }

    #[test]
    fn test_zero_copy_compound_assign() {
        let mut ctx = make_ctx(5);
        let result = execute_formula_zero_copy("X := 10; X += 5; X", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 15.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_zero_copy_barscount() {
        let mut ctx = make_ctx(10);
        let result = execute_formula_zero_copy("BARSCOUNT", &mut ctx).unwrap();
        for i in 0..10 {
            assert!((result[i] - 10.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_zero_copy_barpos() {
        let mut ctx = make_ctx(10);
        let result = execute_formula_zero_copy("BARPOS", &mut ctx).unwrap();
        for i in 0..10 {
            assert!((result[i] - (i as f64 + 1.0)).abs() < 1e-10);
        }
    }

    fn execute_formula_zero_copy_cached(
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        let executor = FormulaExecutor::new();
        executor.execute_zero_copy_cached(&ast, ctx)
    }

    #[test]
    fn test_zero_copy_cached_constant() {
        let mut ctx = make_ctx(5);
        let result = execute_formula_zero_copy_cached("42", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 42.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_zero_copy_cached_addition() {
        let mut ctx = make_ctx(5);
        let result = execute_formula_zero_copy_cached("10 + 20", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 30.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_zero_copy_cached_variable_close() {
        let mut ctx = make_ctx(5);
        let result = execute_formula_zero_copy_cached("CLOSE", &mut ctx).unwrap();
        for i in 0..5 {
            let expected = 10.0 + i as f64 * 0.15;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_zero_copy_cached_close_plus_open() {
        let mut ctx = make_ctx(5);
        let result = execute_formula_zero_copy_cached("C + O", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            assert!((result[i] - (close_val + open_val)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_zero_copy_cached_comparison_gt() {
        let mut ctx = make_ctx(5);
        let result = execute_formula_zero_copy_cached("CLOSE > 10.5", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let expected = if close_val > 10.5 { 1.0 } else { 0.0 };
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_zero_copy_cached_assignment() {
        let mut ctx = make_ctx(5);
        let result = execute_formula_zero_copy_cached("UP := CLOSE + 1", &mut ctx).unwrap();
        for i in 0..5 {
            let expected = 10.0 + i as f64 * 0.15 + 1.0;
            assert!((result[i] - expected).abs() < 1e-10);
        }
        assert!(ctx.variables.contains_key("UP"));
    }

    #[test]
    fn test_zero_copy_cached_function_call() {
        let mut ctx = make_ctx(10);
        let result = execute_formula_zero_copy_cached("MA(CLOSE, 3)", &mut ctx).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(!result[2].is_nan());
    }

    #[test]
    fn test_zero_copy_cached_multiple_statements() {
        let mut ctx = make_ctx(10);
        let source = "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); MA5 > MA10";
        let result = execute_formula_zero_copy_cached(source, &mut ctx).unwrap();
        assert!(ctx.variables.contains_key("MA5"));
        assert!(ctx.variables.contains_key("MA10"));
        assert!(result.len() == 10);
    }

    #[test]
    fn test_zero_copy_cached_matches_normal_execution() {
        let mut ctx1 = make_ctx(30);
        let mut ctx2 = make_ctx(30);
        let source = "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); MA5 > MA10";
        let normal = execute_formula(source, &mut ctx1).unwrap();
        let zero_copy_cached = execute_formula_zero_copy_cached(source, &mut ctx2).unwrap();
        for i in 0..30 {
            if normal[i].is_nan() {
                assert!(zero_copy_cached[i].is_nan());
            } else {
                assert!(
                    (normal[i] - zero_copy_cached[i]).abs() < 1e-10,
                    "Mismatch at {}: {} vs {}",
                    i,
                    normal[i],
                    zero_copy_cached[i]
                );
            }
        }
    }

    #[test]
    fn test_zero_copy_cached_matches_zero_copy() {
        let mut ctx1 = make_ctx(30);
        let mut ctx2 = make_ctx(30);
        let source = "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); MA5 > MA10";
        let zero_copy = execute_formula_zero_copy(source, &mut ctx1).unwrap();
        let zero_copy_cached = execute_formula_zero_copy_cached(source, &mut ctx2).unwrap();
        for i in 0..30 {
            if zero_copy[i].is_nan() {
                assert!(zero_copy_cached[i].is_nan());
            } else {
                assert!(
                    (zero_copy[i] - zero_copy_cached[i]).abs() < 1e-10,
                    "Mismatch at {}: {} vs {}",
                    i,
                    zero_copy[i],
                    zero_copy_cached[i]
                );
            }
        }
    }

    #[test]
    fn test_zero_copy_cached_complex_formula_matches() {
        let mut ctx1 = make_ctx(50);
        let mut ctx2 = make_ctx(50);
        let source = r#"
            SHORT := 12;
            LONG := 26;
            DIF := EMA(CLOSE, SHORT) - EMA(CLOSE, LONG);
            DEA := EMA(DIF, 9);
            MACD := (DIF - DEA) * 2;
            MACD: MACD
        "#;
        let normal = execute_formula(source, &mut ctx1).unwrap();
        let zero_copy_cached = execute_formula_zero_copy_cached(source, &mut ctx2).unwrap();
        for i in 0..50 {
            if normal[i].is_nan() {
                assert!(zero_copy_cached[i].is_nan());
            } else {
                assert!(
                    (normal[i] - zero_copy_cached[i]).abs() < 1e-10,
                    "Mismatch at {}: {} vs {}",
                    i,
                    normal[i],
                    zero_copy_cached[i]
                );
            }
        }
    }

    #[test]
    fn test_zero_copy_cached_unary_neg() {
        let mut ctx = make_ctx(5);
        let result = execute_formula_zero_copy_cached("-100", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - (-100.0)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_zero_copy_cached_division_by_zero() {
        let mut ctx = make_ctx(5);
        let result = execute_formula_zero_copy_cached("10 / 0", &mut ctx).unwrap();
        for i in 0..5 {
            assert!(result[i].is_nan());
        }
    }

    #[test]
    fn test_zero_copy_cached_compound_assign() {
        let mut ctx = make_ctx(5);
        let result = execute_formula_zero_copy_cached("X := 10; X += 5; X", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 15.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_zero_copy_cached_barscount() {
        let mut ctx = make_ctx(10);
        let result = execute_formula_zero_copy_cached("BARSCOUNT", &mut ctx).unwrap();
        for i in 0..10 {
            assert!((result[i] - 10.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_zero_copy_cached_barpos() {
        let mut ctx = make_ctx(10);
        let result = execute_formula_zero_copy_cached("BARPOS", &mut ctx).unwrap();
        for i in 0..10 {
            assert!((result[i] - (i as f64 + 1.0)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_zero_copy_cached_rsi() {
        let mut ctx1 = make_ctx(50);
        let mut ctx2 = make_ctx(50);
        let normal = execute_formula("RSI(CLOSE, 14)", &mut ctx1).unwrap();
        let zero_copy_cached =
            execute_formula_zero_copy_cached("RSI(CLOSE, 14)", &mut ctx2).unwrap();
        for i in 0..50 {
            if normal[i].is_nan() {
                assert!(zero_copy_cached[i].is_nan());
            } else {
                assert!(
                    (normal[i] - zero_copy_cached[i]).abs() < 1e-10,
                    "Mismatch at {}: {} vs {}",
                    i,
                    normal[i],
                    zero_copy_cached[i]
                );
            }
        }
    }

    fn execute_formula_eval_into(
        source: &str,
        ctx: &mut FormulaContext,
        output: &mut Array1<f64>,
    ) -> Result<(), FormulaError> {
        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        let executor = FormulaExecutor::new();
        executor.eval_into(&ast, ctx, output)
    }

    #[test]
    fn test_eval_into_constant() {
        let mut ctx = make_ctx(5);
        let mut output = Array1::zeros(5);
        execute_formula_eval_into("42", &mut ctx, &mut output).unwrap();
        for i in 0..5 {
            assert!((output[i] - 42.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_into_addition() {
        let mut ctx = make_ctx(5);
        let mut output = Array1::zeros(5);
        execute_formula_eval_into("10 + 20", &mut ctx, &mut output).unwrap();
        for i in 0..5 {
            assert!((output[i] - 30.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_into_close_times_two() {
        let mut ctx = make_ctx(10);
        let mut output = Array1::zeros(10);
        execute_formula_eval_into("CLOSE * 2", &mut ctx, &mut output).unwrap();
        for i in 0..10 {
            let expected = (10.0 + i as f64 * 0.15) * 2.0;
            assert!((output[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_into_length_mismatch() {
        let mut ctx = make_ctx(5);
        let mut output = Array1::zeros(3); // 长度不匹配
        let result = execute_formula_eval_into("42", &mut ctx, &mut output);
        assert!(result.is_err());
    }

    #[test]
    fn test_eval_into_function_call() {
        let mut ctx = make_ctx(10);
        let mut output = Array1::zeros(10);
        execute_formula_eval_into("MA(CLOSE, 3)", &mut ctx, &mut output).unwrap();
        assert!(output[0].is_nan());
        assert!(output[1].is_nan());
        assert!(!output[2].is_nan());
    }

    #[test]
    fn test_eval_into_matches_normal_execution() {
        let mut ctx1 = make_ctx(30);
        let mut ctx2 = make_ctx(30);
        let source = "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); MA5 > MA10";
        let normal = execute_formula(source, &mut ctx1).unwrap();
        let mut output = Array1::zeros(30);
        execute_formula_eval_into(source, &mut ctx2, &mut output).unwrap();
        for i in 0..30 {
            if normal[i].is_nan() {
                assert!(output[i].is_nan());
            } else {
                assert!(
                    (normal[i] - output[i]).abs() < 1e-10,
                    "Mismatch at {}: {} vs {}",
                    i,
                    normal[i],
                    output[i]
                );
            }
        }
    }

    #[test]
    fn test_eval_into_complex_formula_matches() {
        let mut ctx1 = make_ctx(50);
        let mut ctx2 = make_ctx(50);
        let source = r#"
            SHORT := 12;
            LONG := 26;
            DIF := EMA(CLOSE, SHORT) - EMA(CLOSE, LONG);
            DEA := EMA(DIF, 9);
            MACD := (DIF - DEA) * 2;
            MACD: MACD
        "#;
        let normal = execute_formula(source, &mut ctx1).unwrap();
        let mut output = Array1::zeros(50);
        execute_formula_eval_into(source, &mut ctx2, &mut output).unwrap();
        for i in 0..50 {
            if normal[i].is_nan() {
                assert!(output[i].is_nan());
            } else {
                assert!(
                    (normal[i] - output[i]).abs() < 1e-10,
                    "Mismatch at {}: {} vs {}",
                    i,
                    normal[i],
                    output[i]
                );
            }
        }
    }

    fn execute_formula_eval_borrowed(
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        let executor = FormulaExecutor::new();
        executor.eval_borrowed(&ast, ctx)
    }

    #[test]
    fn test_eval_borrowed_basic() {
        let mut ctx = make_ctx(10);
        let result = execute_formula_eval_borrowed("CLOSE + 1", &mut ctx).unwrap();
        for i in 0..10 {
            let expected = 10.0 + i as f64 * 0.15 + 1.0;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_borrowed_matches_zero_copy() {
        let mut ctx1 = make_ctx(30);
        let mut ctx2 = make_ctx(30);
        let source = "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); MA5 - MA10";
        let zero_copy = execute_formula_zero_copy(source, &mut ctx1).unwrap();
        let borrowed = execute_formula_eval_borrowed(source, &mut ctx2).unwrap();
        for i in 0..30 {
            if zero_copy[i].is_nan() {
                assert!(borrowed[i].is_nan());
            } else {
                assert!(
                    (zero_copy[i] - borrowed[i]).abs() < 1e-10,
                    "Mismatch at {}: {} vs {}",
                    i,
                    zero_copy[i],
                    borrowed[i]
                );
            }
        }
    }
}
