use ahash::AHashMap;
use ndarray::Array1;
use std::collections::HashSet;

use crate::formula::bytecode::{Bytecode, ExecResult, OpCode};
use crate::formula::drawing::DrawResult;
use crate::formula::functions::get_builtin_functions;
use crate::formula::simd::SimdOps;
use crate::formula::types::*;

type FormulaFn = fn(&FormulaContext, &[Array1<f64>]) -> Result<Array1<f64>, FormulaError>;

pub struct JitCompiler {
    /// P.3 aHash 化（替代 std HashMap）：更快的小字符串哈希
    inline_cache: AHashMap<String, usize>,
    hot_count: AHashMap<String, u32>,
    jit_threshold: u32,
    optimized_cache: AHashMap<String, OptimizedBytecode>,
    /// 内置函数表（按 name 排序，二分查找 O(log N)，替代 HashMap 哈希）
    builtin_vec: Vec<(String, FormulaFn)>,
}

pub struct OptimizedBytecode {
    bytecode: Bytecode,
    buffer_size: usize,
    cached_function_indices: Vec<Option<usize>>,
    is_hot: bool,
}

impl Default for JitCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl JitCompiler {
    pub fn new() -> Self {
        let builtin_vec = Self::build_builtin_vec();
        Self {
            inline_cache: AHashMap::new(),
            hot_count: AHashMap::new(),
            jit_threshold: 10,
            optimized_cache: AHashMap::new(),
            builtin_vec,
        }
    }

    /// P.3 perfect-hash 二分查找：O(log N) 内置函数定位。
    /// 替代 `builtin_vec.iter().find(|f| f.0 == name)` 的 O(N) 线性扫描。
    pub fn find_builtin(&self, name: &str) -> Option<&FormulaFn> {
        if self.builtin_vec.is_empty() {
            return None;
        }
        let mut lo = 0usize;
        let mut hi = self.builtin_vec.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            match self.builtin_vec[mid].0.as_str().cmp(name) {
                std::cmp::Ordering::Equal => return Some(&self.builtin_vec[mid].1),
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
            }
        }
        None
    }

    pub fn with_threshold(mut self, threshold: u32) -> Self {
        self.jit_threshold = threshold;
        self
    }

    fn build_builtin_vec() -> Vec<(String, FormulaFn)> {
        let builtins = get_builtin_functions();
        let mut vec: Vec<(String, FormulaFn)> = builtins.into_iter().collect();
        vec.sort_by(|a, b| a.0.cmp(&b.0));
        vec
    }

    pub fn compile(&mut self, bytecode: Bytecode) -> OptimizedBytecode {
        let is_hot = self.is_hot(&bytecode.source);
        let optimized_bytecode = if is_hot {
            self.optimize_bytecode(&bytecode)
        } else {
            bytecode.clone()
        };

        let buffer_size = Self::estimate_buffer_size(&optimized_bytecode.instructions);
        let cached_function_indices =
            Self::build_inline_cache(&optimized_bytecode.instructions, &mut self.inline_cache);

        OptimizedBytecode {
            bytecode: optimized_bytecode,
            buffer_size,
            cached_function_indices,
            is_hot,
        }
    }

    pub fn execute(
        &self,
        optimized: &OptimizedBytecode,
        ctx: &mut FormulaContext,
    ) -> Result<ExecResult, FormulaError> {
        self.execute_optimized(optimized, ctx)
    }

    pub fn execute_optimized(
        &self,
        optimized: &OptimizedBytecode,
        ctx: &mut FormulaContext,
    ) -> Result<ExecResult, FormulaError> {
        let data_len = ctx.data_len;
        let mut stack: Vec<Array1<f64>> = Vec::with_capacity(optimized.buffer_size);
        let mut variables: AHashMap<String, Array1<f64>> = AHashMap::new();
        let mut outputs: AHashMap<String, Array1<f64>> = AHashMap::new();
        let mut draw_commands = DrawResult::new();
        let instructions = &optimized.bytecode.instructions;
        let mut pc: usize = 0;

        while pc < instructions.len() {
            let op = &instructions[pc];
            match op {
                OpCode::PushConst(val) => {
                    stack.push(Array1::from_elem(data_len, *val));
                }
                OpCode::LoadVar(name) => {
                    let value = Self::load_variable(&variables, name, ctx)?;
                    stack.push(value);
                }
                OpCode::StoreVar(name) => {
                    if let Some(value) = stack.pop() {
                        variables.insert(name.clone(), value);
                    } else {
                        return Err(FormulaError::RuntimeError(
                            "Stack underflow on StoreVar".to_string(),
                        ));
                    }
                }
                OpCode::Add => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let len = left.len().min(right.len());
                    let result = if len >= 16 {
                        let mut r = Array1::zeros(len);
                        SimdOps::add(
                            left.as_slice().unwrap(),
                            right.as_slice().unwrap(),
                            r.as_slice_mut().unwrap(),
                        );
                        r
                    } else {
                        &left + &right
                    };
                    stack.push(result);
                }
                OpCode::Sub => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let len = left.len().min(right.len());
                    let result = if len >= 16 {
                        let mut r = Array1::zeros(len);
                        SimdOps::sub(
                            left.as_slice().unwrap(),
                            right.as_slice().unwrap(),
                            r.as_slice_mut().unwrap(),
                        );
                        r
                    } else {
                        &left - &right
                    };
                    stack.push(result);
                }
                OpCode::Mul => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let len = left.len().min(right.len());
                    let result = if len >= 16 {
                        let mut r = Array1::zeros(len);
                        SimdOps::mul(
                            left.as_slice().unwrap(),
                            right.as_slice().unwrap(),
                            r.as_slice_mut().unwrap(),
                        );
                        r
                    } else {
                        &left * &right
                    };
                    stack.push(result);
                }
                OpCode::Div => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let len = left.len().min(right.len());
                    let result = if len >= 16 {
                        let mut r = Array1::zeros(len);
                        SimdOps::div(
                            left.as_slice().unwrap(),
                            right.as_slice().unwrap(),
                            r.as_slice_mut().unwrap(),
                        );
                        r
                    } else {
                        let mut r = Array1::zeros(left.len());
                        for i in 0..left.len() {
                            if right[i].abs() < 1e-15 {
                                r[i] = f64::NAN;
                            } else {
                                r[i] = left[i] / right[i];
                            }
                        }
                        r
                    };
                    stack.push(result);
                }
                OpCode::Mod => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let mut result = Array1::zeros(left.len());
                    for i in 0..left.len() {
                        if right[i].abs() < 1e-15 {
                            result[i] = f64::NAN;
                        } else {
                            result[i] = left[i] % right[i];
                        }
                    }
                    stack.push(result);
                }
                OpCode::Pow => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let result = left
                        .iter()
                        .zip(right.iter())
                        .map(|(&l, &r)| l.powf(r))
                        .collect();
                    stack.push(result);
                }
                OpCode::Gt => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let len = left.len().min(right.len());
                    let result = if len >= 16 {
                        let mut r = Array1::zeros(len);
                        SimdOps::gt(
                            left.as_slice().unwrap(),
                            right.as_slice().unwrap(),
                            r.as_slice_mut().unwrap(),
                        );
                        r
                    } else {
                        left.iter()
                            .zip(right.iter())
                            .map(|(&l, &r)| if l > r { 1.0 } else { 0.0 })
                            .collect()
                    };
                    stack.push(result);
                }
                OpCode::Lt => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let len = left.len().min(right.len());
                    let result = if len >= 16 {
                        let mut r = Array1::zeros(len);
                        SimdOps::lt(
                            left.as_slice().unwrap(),
                            right.as_slice().unwrap(),
                            r.as_slice_mut().unwrap(),
                        );
                        r
                    } else {
                        left.iter()
                            .zip(right.iter())
                            .map(|(&l, &r)| if l < r { 1.0 } else { 0.0 })
                            .collect()
                    };
                    stack.push(result);
                }
                OpCode::Gte => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let len = left.len().min(right.len());
                    let result = if len >= 16 {
                        let mut r = Array1::zeros(len);
                        SimdOps::gte(
                            left.as_slice().unwrap(),
                            right.as_slice().unwrap(),
                            r.as_slice_mut().unwrap(),
                        );
                        r
                    } else {
                        left.iter()
                            .zip(right.iter())
                            .map(|(&l, &r)| if l >= r { 1.0 } else { 0.0 })
                            .collect()
                    };
                    stack.push(result);
                }
                OpCode::Lte => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let len = left.len().min(right.len());
                    let result = if len >= 16 {
                        let mut r = Array1::zeros(len);
                        SimdOps::lte(
                            left.as_slice().unwrap(),
                            right.as_slice().unwrap(),
                            r.as_slice_mut().unwrap(),
                        );
                        r
                    } else {
                        left.iter()
                            .zip(right.iter())
                            .map(|(&l, &r)| if l <= r { 1.0 } else { 0.0 })
                            .collect()
                    };
                    stack.push(result);
                }
                OpCode::Eq => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let len = left.len().min(right.len());
                    let result = if len >= 16 {
                        let mut r = Array1::zeros(len);
                        SimdOps::eq(
                            left.as_slice().unwrap(),
                            right.as_slice().unwrap(),
                            r.as_slice_mut().unwrap(),
                        );
                        r
                    } else {
                        left.iter()
                            .zip(right.iter())
                            .map(|(&l, &r)| if (l - r).abs() < 1e-10 { 1.0 } else { 0.0 })
                            .collect()
                    };
                    stack.push(result);
                }
                OpCode::Neq => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let len = left.len().min(right.len());
                    let result = if len >= 16 {
                        let mut r = Array1::zeros(len);
                        SimdOps::neq(
                            left.as_slice().unwrap(),
                            right.as_slice().unwrap(),
                            r.as_slice_mut().unwrap(),
                        );
                        r
                    } else {
                        left.iter()
                            .zip(right.iter())
                            .map(|(&l, &r)| if (l - r).abs() >= 1e-10 { 1.0 } else { 0.0 })
                            .collect()
                    };
                    stack.push(result);
                }
                OpCode::And => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let result = left
                        .iter()
                        .zip(right.iter())
                        .map(|(&l, &r)| if l > 0.0 && r > 0.0 { 1.0 } else { 0.0 })
                        .collect();
                    stack.push(result);
                }
                OpCode::Or => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let result = left
                        .iter()
                        .zip(right.iter())
                        .map(|(&l, &r)| if l > 0.0 || r > 0.0 { 1.0 } else { 0.0 })
                        .collect();
                    stack.push(result);
                }
                OpCode::Xor => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let result = left
                        .iter()
                        .zip(right.iter())
                        .map(|(&l, &r)| if (l > 0.0) != (r > 0.0) { 1.0 } else { 0.0 })
                        .collect();
                    stack.push(result);
                }
                OpCode::Not => {
                    let val = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let result = val.mapv(|v| if v > 0.0 { 0.0 } else { 1.0 });
                    stack.push(result);
                }
                OpCode::StringConcat => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let result = left
                        .iter()
                        .zip(right.iter())
                        .map(|(&l, &r)| {
                            let l_str = if l.is_nan() {
                                "NaN".to_string()
                            } else {
                                format!("{}", l)
                            };
                            let r_str = if r.is_nan() {
                                "NaN".to_string()
                            } else {
                                format!("{}", r)
                            };
                            let combined = format!("{}{}", l_str, r_str);
                            combined.parse::<f64>().unwrap_or(0.0)
                        })
                        .collect();
                    stack.push(result);
                }
                OpCode::Jump(target) => {
                    pc = *target;
                    continue;
                }
                OpCode::JumpIfFalse(target) => {
                    let cond = stack.pop().ok_or_else(|| {
                        FormulaError::RuntimeError("Stack underflow on JumpIfFalse".to_string())
                    })?;
                    let all_false = cond.iter().all(|&v| v <= 0.0);
                    if all_false {
                        pc = *target;
                        continue;
                    }
                    stack.push(cond);
                }
                OpCode::Call { name, arg_count } => {
                    let arg_count = *arg_count;
                    let mut args = Vec::with_capacity(arg_count);
                    for _ in 0..arg_count {
                        args.push(stack.pop().ok_or_else(|| {
                            FormulaError::RuntimeError("Stack underflow".to_string())
                        })?);
                    }
                    args.reverse();

                    let result = if let Some(Some(idx)) = optimized.cached_function_indices.get(pc)
                    {
                        if *idx < self.builtin_vec.len() {
                            (self.builtin_vec[*idx].1)(ctx, &args)?
                        } else {
                            return Err(FormulaError::RuntimeError(format!(
                                "Invalid cached index for function: {}",
                                name
                            )));
                        }
                    } else {
                        let builtins = get_builtin_functions();
                        let func = builtins.get(name.as_str()).ok_or_else(|| {
                            FormulaError::RuntimeError(format!("Unknown function: {}", name))
                        })?;
                        func(ctx, &args)?
                    };
                    stack.push(result);
                }
                OpCode::LoadData(name) => {
                    let data = ctx.get_data(name).ok_or_else(|| {
                        FormulaError::RuntimeError(format!("Data not available: {}", name))
                    })?;
                    stack.push(Array1::from_vec(data.to_vec()));
                }
                OpCode::Index => {
                    let idx_val = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let arr_val = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let mut result = Array1::zeros(data_len);
                    for i in 0..data_len {
                        let idx = idx_val[i] as usize;
                        if idx < arr_val.len() {
                            result[i] = arr_val[idx];
                        } else {
                            result[i] = f64::NAN;
                        }
                    }
                    stack.push(result);
                }
                OpCode::CompoundStore { name, op } => {
                    let rhs = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let current = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
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
                    variables.insert(name.clone(), value.clone());
                    stack.push(value);
                }
                OpCode::Output(name) => {
                    if let Some(value) = variables.get(name) {
                        outputs.insert(name.clone(), value.clone());
                    }
                }
                OpCode::DrawText { text, color } => {
                    let price = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let cond = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    draw_commands.add_text(cond, price, text.clone(), color.clone());
                }
                OpCode::DrawIcon { color } => {
                    let _icon = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let price = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let cond = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let icon_type = _icon[0] as i32;
                    draw_commands.add_icon(cond, price, icon_type, color.clone());
                }
                OpCode::StickLine { empty, color } => {
                    let width_val = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let price2 = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let price1 = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let cond = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let width = width_val[0] as i32;
                    draw_commands.add_stick(cond, price1, price2, width, *empty, color.clone());
                }
                OpCode::DrawGeneric { arg_count, .. } => {
                    for _ in 0..*arg_count {
                        stack.pop();
                    }
                }
                OpCode::PushString(_) => {
                    stack.push(Array1::zeros(data_len));
                }
                OpCode::Select => {
                    let else_val = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let then_val = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let cond = stack
                        .pop()
                        .ok_or_else(|| FormulaError::RuntimeError("Stack underflow".to_string()))?;
                    let len = cond.len().min(then_val.len()).min(else_val.len());
                    let result = if len >= 16 {
                        SimdOps::simd_select_arrays(&cond, &then_val, &else_val)
                    } else {
                        cond.iter()
                            .zip(then_val.iter())
                            .zip(else_val.iter())
                            .map(|((&c, &t), &e)| if c > 0.0 { t } else { e })
                            .collect()
                    };
                    stack.push(result);
                }
            }
            pc += 1;
        }

        let final_value = stack.pop().unwrap_or_else(|| Array1::zeros(data_len));
        Ok(ExecResult {
            outputs,
            final_value,
            draw_commands,
        })
    }

    pub fn compile_and_execute(
        &mut self,
        bytecode: Bytecode,
        ctx: &mut FormulaContext,
    ) -> Result<ExecResult, FormulaError> {
        let source = bytecode.source.clone();
        self.record_execution(&source);

        if self.is_hot(&source) {
            if !self.optimized_cache.contains_key(&source) {
                let optimized = self.compile(bytecode);
                self.optimized_cache.insert(source.clone(), optimized);
            }
            let optimized = self.optimized_cache.get(&source).unwrap();
            self.execute_optimized(optimized, ctx)
        } else {
            let optimized = self.compile(bytecode);
            self.execute_optimized(&optimized, ctx)
        }
    }

    pub fn optimize_bytecode(&mut self, bytecode: &Bytecode) -> Bytecode {
        let mut result = bytecode.clone();
        result.instructions = Self::constant_fold(&result.instructions);
        result.instructions = Self::identity_eliminate(&result.instructions);
        result.instructions = Self::dead_code_eliminate(&result.instructions);
        result
    }

    fn identity_eliminate(instructions: &[OpCode]) -> Vec<OpCode> {
        let mut result: Vec<OpCode> = Vec::with_capacity(instructions.len());
        for op in instructions {
            result.push(op.clone());
            if result.len() >= 2 {
                let len = result.len();
                let should_remove = match (&result[len - 2], &result[len - 1]) {
                    // x + 0 → x  |  0 + x → already folded by constant_fold
                    (_, OpCode::Add) | (_, OpCode::Sub) => {
                        matches!(&result.get(len.wrapping_sub(3)), Some(OpCode::PushConst(v)) if *v == 0.0)
                            || (len >= 2
                                && matches!(&result[len - 2], OpCode::PushConst(v) if *v == 0.0))
                    }
                    // x * 1 → x  |  x / 1 → x
                    (OpCode::PushConst(v), OpCode::Mul | OpCode::Div) if *v == 1.0 => true,
                    // x * 0 → PushConst(0)
                    (OpCode::PushConst(v), OpCode::Mul) if *v == 0.0 => false,
                    _ => false,
                };
                if should_remove {
                    result.pop();
                    result.pop();
                }
            }
        }
        result
    }

    fn constant_fold(instructions: &[OpCode]) -> Vec<OpCode> {
        let mut result: Vec<OpCode> = Vec::with_capacity(instructions.len());
        for op in instructions {
            result.push(op.clone());
            while result.len() >= 3 {
                let len = result.len();
                match (&result[len - 3], &result[len - 2], &result[len - 1]) {
                    (OpCode::PushConst(a), OpCode::PushConst(b), bin_op) => {
                        if let Some(folded) = Self::fold_binary(*a, *b, bin_op) {
                            result.truncate(len - 2);
                            result[len - 3] = OpCode::PushConst(folded);
                        } else {
                            break;
                        }
                    }
                    _ => break,
                }
            }
        }
        result
    }

    fn fold_binary(a: f64, b: f64, op: &OpCode) -> Option<f64> {
        match op {
            OpCode::Add => Some(a + b),
            OpCode::Sub => Some(a - b),
            OpCode::Mul => Some(a * b),
            OpCode::Div => {
                if b.abs() < 1e-15 {
                    Some(f64::NAN)
                } else {
                    Some(a / b)
                }
            }
            OpCode::Mod => {
                if b.abs() < 1e-15 {
                    Some(f64::NAN)
                } else {
                    Some(a % b)
                }
            }
            OpCode::Pow => Some(a.powf(b)),
            OpCode::Gt => Some(if a > b { 1.0 } else { 0.0 }),
            OpCode::Lt => Some(if a < b { 1.0 } else { 0.0 }),
            OpCode::Gte => Some(if a >= b { 1.0 } else { 0.0 }),
            OpCode::Lte => Some(if a <= b { 1.0 } else { 0.0 }),
            OpCode::Eq => Some(if (a - b).abs() < 1e-10 { 1.0 } else { 0.0 }),
            OpCode::Neq => Some(if (a - b).abs() >= 1e-10 { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    fn dead_code_eliminate(instructions: &[OpCode]) -> Vec<OpCode> {
        if instructions.is_empty() {
            return Vec::new();
        }

        let mut used_vars: HashSet<String> = HashSet::new();
        for op in instructions {
            match op {
                OpCode::LoadVar(name) => {
                    used_vars.insert(name.clone());
                }
                OpCode::Output(name) => {
                    used_vars.insert(name.clone());
                }
                OpCode::CompoundStore { name, .. } => {
                    used_vars.insert(name.clone());
                }
                _ => {}
            }
        }

        let mut remove_set: HashSet<usize> = HashSet::new();

        for (i, op) in instructions.iter().enumerate() {
            if let OpCode::StoreVar(name) = op {
                if !used_vars.contains(name) {
                    remove_set.insert(i);
                    let mut stack_depth: i32 = 1;
                    let mut j: i32 = i as i32 - 1;
                    while stack_depth > 0 && j >= 0 {
                        let (pushes, pops) = Self::stack_effect(&instructions[j as usize]);
                        stack_depth = stack_depth - pushes as i32 + pops as i32;
                        remove_set.insert(j as usize);
                        j -= 1;
                    }
                }
            }
        }

        let mut jump_targets: HashSet<usize> = HashSet::new();
        for op in instructions {
            match op {
                OpCode::Jump(target) => {
                    jump_targets.insert(*target);
                }
                OpCode::JumpIfFalse(target) => {
                    jump_targets.insert(*target);
                }
                _ => {}
            }
        }

        for (i, op) in instructions.iter().enumerate() {
            if let OpCode::Jump(target) = op {
                for j in (i + 1)..(*target).min(instructions.len()) {
                    if !jump_targets.contains(&j) {
                        remove_set.insert(j);
                    }
                }
            }
        }

        instructions
            .iter()
            .enumerate()
            .filter(|(i, _)| !remove_set.contains(i))
            .map(|(_, op)| op.clone())
            .collect()
    }

    fn stack_effect(op: &OpCode) -> (usize, usize) {
        match op {
            OpCode::PushConst(_) | OpCode::LoadVar(_) | OpCode::LoadData(_) => (1, 0),
            OpCode::StoreVar(_) => (0, 1),
            OpCode::Add
            | OpCode::Sub
            | OpCode::Mul
            | OpCode::Div
            | OpCode::Mod
            | OpCode::Pow
            | OpCode::Gt
            | OpCode::Lt
            | OpCode::Gte
            | OpCode::Lte
            | OpCode::Eq
            | OpCode::Neq
            | OpCode::And
            | OpCode::Or
            | OpCode::Xor
            | OpCode::StringConcat => (1, 2),
            OpCode::Call { arg_count, .. } => (1, *arg_count),
            OpCode::CompoundStore { .. } => (1, 2),
            OpCode::Output(_) => (0, 0),
            OpCode::Not => (1, 1),
            OpCode::Index => (1, 2),
            OpCode::Jump(_) | OpCode::JumpIfFalse(_) => (0, 0),
            OpCode::DrawText { .. } => (0, 2),
            OpCode::DrawIcon { .. } => (0, 3),
            OpCode::StickLine { .. } => (0, 4),
            OpCode::DrawGeneric { arg_count, .. } => (0, *arg_count),
            OpCode::PushString(_) => (1, 0),
            OpCode::Select => (1, 3),
        }
    }

    fn load_variable(
        variables: &AHashMap<String, Array1<f64>>,
        name: &str,
        ctx: &FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let bytes = name.as_bytes();
        if bytes.eq_ignore_ascii_case(b"BARSCOUNT") {
            Ok(Array1::from_elem(ctx.data_len, ctx.data_len as f64))
        } else if bytes.eq_ignore_ascii_case(b"BARPOS") {
            Ok(Array1::from(
                (1..=ctx.data_len).map(|i| i as f64).collect::<Vec<_>>(),
            ))
        } else if bytes.eq_ignore_ascii_case(b"CAPITAL") {
            let val = ctx.capital.unwrap_or(f64::NAN);
            Ok(Array1::from_elem(ctx.data_len, val))
        } else if bytes.eq_ignore_ascii_case(b"DRAWNULL") {
            Ok(Array1::from_elem(ctx.data_len, f64::NAN))
        } else {
            variables
                .get(name)
                .cloned()
                .ok_or_else(|| FormulaError::RuntimeError(format!("Unknown variable: {}", name)))
        }
    }

    fn is_hot(&self, source: &str) -> bool {
        self.hot_count
            .get(source)
            .map(|&count| count >= self.jit_threshold)
            .unwrap_or(false)
    }

    fn record_execution(&mut self, source: &str) {
        *self.hot_count.entry(source.to_string()).or_insert(0) += 1;
    }

    pub fn get_execution_count(&self, source: &str) -> u32 {
        self.hot_count.get(source).copied().unwrap_or(0)
    }

    pub fn reset_hot_counts(&mut self) {
        self.hot_count.clear();
        self.optimized_cache.clear();
    }

    pub fn is_cached(&self, source: &str) -> bool {
        self.optimized_cache.contains_key(source)
    }

    fn estimate_buffer_size(instructions: &[OpCode]) -> usize {
        let mut stack_depth = 0usize;
        let mut max_depth = 0usize;

        for op in instructions {
            match op {
                OpCode::PushConst(_) | OpCode::LoadVar(_) | OpCode::LoadData(_) => {
                    stack_depth += 1;
                    if stack_depth > max_depth {
                        max_depth = stack_depth;
                    }
                }
                OpCode::Add
                | OpCode::Sub
                | OpCode::Mul
                | OpCode::Div
                | OpCode::Mod
                | OpCode::Pow
                | OpCode::Gt
                | OpCode::Lt
                | OpCode::Gte
                | OpCode::Lte
                | OpCode::Eq
                | OpCode::Neq
                | OpCode::And
                | OpCode::Or => {
                    stack_depth = stack_depth.saturating_sub(1);
                }
                OpCode::Call { arg_count, .. } if *arg_count > 0 => {
                    stack_depth = stack_depth.saturating_sub(arg_count - 1);
                }
                OpCode::StoreVar(_) | OpCode::Output(_) => {
                    stack_depth = stack_depth.saturating_sub(1);
                }
                _ => {}
            }
        }

        max_depth.max(4)
    }

    fn build_inline_cache(
        instructions: &[OpCode],
        global_cache: &mut AHashMap<String, usize>,
    ) -> Vec<Option<usize>> {
        let builtins = get_builtin_functions();
        let mut builtin_names: Vec<&str> = builtins.keys().map(|s| s.as_str()).collect();
        builtin_names.sort();

        let mut cached = Vec::with_capacity(instructions.len());

        for op in instructions {
            match op {
                OpCode::Call { name, .. } => {
                    if let Some(&idx) = global_cache.get(name.as_str()) {
                        cached.push(Some(idx));
                    } else if let Some(pos) = builtin_names.iter().position(|&n| n == name) {
                        global_cache.insert(name.clone(), pos);
                        cached.push(Some(pos));
                    } else {
                        cached.push(None);
                    }
                }
                _ => cached.push(None),
            }
        }

        cached
    }
}

impl OptimizedBytecode {
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    pub fn is_hot(&self) -> bool {
        self.is_hot
    }

    pub fn source(&self) -> &str {
        &self.bytecode.source
    }

    pub fn instruction_count(&self) -> usize {
        self.bytecode.instructions.len()
    }

    pub fn cached_call_count(&self) -> usize {
        self.cached_function_indices
            .iter()
            .filter(|c| c.is_some())
            .count()
    }

    pub fn instructions(&self) -> &[OpCode] {
        &self.bytecode.instructions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::bytecode::compile_to_bytecode;
    use crate::formula::parser::parse_formula;

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

    // =========================================================================
    // P.3 新增测试：perfect-hash / hot count / aHash 一致性
    // =========================================================================

    #[test]
    fn test_jit_perfect_hash_lookup_1000_hits() {
        let compiler = JitCompiler::new();
        let names: Vec<String> = compiler
            .builtin_vec
            .iter()
            .map(|(n, _)| n.clone())
            .collect();
        assert!(!names.is_empty(), "builtin_vec should not be empty");
        for i in 0..1000usize {
            let idx = i.wrapping_mul(2654435761) % names.len();
            let name = &names[idx];
            let found = compiler.find_builtin(name);
            assert!(found.is_some(), "builtin {} not found", name);
        }
        assert!(compiler.find_builtin("NONEXISTENT_FUNC_42").is_none());
    }

    #[test]
    fn test_jit_hot_count_threshold() {
        let mut compiler = JitCompiler::new().with_threshold(10);
        let src = "A := CLOSE + 1.0; A";
        let make_ctx_v = |len: usize| -> FormulaContext {
            let close = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.1).collect());
            let open = close.clone();
            let high = close.clone();
            let low = close.clone();
            let volume = close.clone();
            FormulaContext::new(open, high, low, close, volume, None)
        };
        for i in 1..=9 {
            let bc = compile_formula(src);
            let mut ctx = make_ctx_v(20);
            let _ = compiler.compile_and_execute(bc, &mut ctx);
            let count = *compiler
                .hot_count
                .get(src)
                .expect("hot_count should track source");
            assert_eq!(count, i, "hot_count mismatch at iter {}", i);
            assert!(
                !compiler.is_hot(src),
                "should not be hot at iter {} (count={})",
                i,
                count
            );
        }
        // 第 10 次：count=10 >= threshold=10 → is_hot 应为 true
        let bc = compile_formula(src);
        let mut ctx = make_ctx_v(20);
        let _ = compiler.compile_and_execute(bc, &mut ctx);
        let count = *compiler.hot_count.get(src).unwrap();
        assert_eq!(count, 10);
        assert!(
            compiler.is_hot(src),
            "should be hot at count=10 with threshold=10"
        );
    }

    #[test]
    fn test_jit_ahash_matches_std_output() {
        use std::collections::HashMap;
        let mut ah: AHashMap<String, i32> = AHashMap::new();
        let mut std: HashMap<String, i32> = HashMap::new();
        let pairs = [("SMA", 5), ("EMA", 10), ("RSI", 14), ("MACD", 12)];
        for (k, v) in pairs {
            ah.insert(k.to_string(), v);
            std.insert(k.to_string(), v);
        }
        for (k, v) in pairs {
            assert_eq!(ah.get(k), Some(&v));
            assert_eq!(std.get(k), Some(&v));
        }
        assert_eq!(ah.len(), std.len());
    }

    #[test]
    fn test_jit_compiler_new() {
        let compiler = JitCompiler::new();
        assert_eq!(compiler.jit_threshold, 10);
        assert!(compiler.inline_cache.is_empty());
        assert!(compiler.hot_count.is_empty());
    }

    #[test]
    fn test_jit_compiler_with_threshold() {
        let compiler = JitCompiler::new().with_threshold(5);
        assert_eq!(compiler.jit_threshold, 5);
    }

    #[test]
    fn test_compile_simple_expression() {
        let mut compiler = JitCompiler::new();
        let bytecode = compile_formula("10 + 20");
        let optimized = compiler.compile(bytecode);
        assert!(optimized.buffer_size() >= 2);
        assert!(!optimized.is_hot());
        assert_eq!(optimized.source(), "10 + 20");
    }

    #[test]
    fn test_execute_simple_expression() {
        let mut compiler = JitCompiler::new();
        let bytecode = compile_formula("10 + 20");
        let optimized = compiler.compile(bytecode);
        let mut ctx = make_ctx(5);
        let result = compiler.execute(&optimized, &mut ctx).unwrap().final_value;
        for i in 0..5 {
            assert!((result[i] - 30.0).abs() < 1e-10, "index {}", i);
        }
    }

    #[test]
    fn test_hot_detection() {
        let mut compiler = JitCompiler::new().with_threshold(3);
        let source = "CLOSE + OPEN";
        assert!(!compiler.is_hot(source));
        for _ in 0..3 {
            compiler.record_execution(source);
        }
        assert!(compiler.is_hot(source));
    }

    #[test]
    fn test_record_execution() {
        let mut compiler = JitCompiler::new();
        compiler.record_execution("test");
        compiler.record_execution("test");
        assert_eq!(compiler.get_execution_count("test"), 2);
        assert_eq!(compiler.get_execution_count("other"), 0);
    }

    #[test]
    fn test_reset_hot_counts() {
        let mut compiler = JitCompiler::new();
        compiler.record_execution("test");
        compiler.reset_hot_counts();
        assert_eq!(compiler.get_execution_count("test"), 0);
    }

    #[test]
    fn test_inline_cache_for_function_calls() {
        let mut compiler = JitCompiler::new();
        let bytecode = compile_formula("MA(CLOSE, 5)");
        let optimized = compiler.compile(bytecode);
        assert!(optimized.cached_call_count() > 0);
    }

    #[test]
    fn test_compile_and_execute() {
        let mut compiler = JitCompiler::new();
        let bytecode = compile_formula("CLOSE * 2");
        let mut ctx = make_ctx(5);
        let result = compiler
            .compile_and_execute(bytecode, &mut ctx)
            .unwrap()
            .final_value;
        for i in 0..5 {
            let expected = (10.0 + i as f64 * 0.15) * 2.0;
            assert!((result[i] - expected).abs() < 1e-10);
        }
        assert_eq!(compiler.get_execution_count("CLOSE * 2"), 1);
    }

    #[test]
    fn test_hot_path_optimization() {
        let mut compiler = JitCompiler::new().with_threshold(2);
        let source = "MA(CLOSE, 5)";
        for _ in 0..3 {
            let bytecode = compile_formula(source);
            let mut ctx = make_ctx(10);
            let _ = compiler.compile_and_execute(bytecode, &mut ctx);
        }
        let bytecode = compile_formula(source);
        let optimized = compiler.compile(bytecode);
        assert!(optimized.is_hot());
    }

    #[test]
    fn test_buffer_size_estimation() {
        let mut compiler = JitCompiler::new();
        let bytecode = compile_formula("10 + 20 + 30");
        let optimized = compiler.compile(bytecode);
        assert!(optimized.buffer_size() >= 2);
        let simple_bytecode = compile_formula("42");
        let simple_opt = compiler.compile(simple_bytecode);
        assert!(simple_opt.buffer_size() >= 1);
    }

    #[test]
    fn test_instruction_count() {
        let mut compiler = JitCompiler::new();
        let bytecode = compile_formula("10 + 20");
        let optimized = compiler.compile(bytecode);
        assert_eq!(optimized.instruction_count(), 3);
    }

    #[test]
    fn test_execute_with_variable_reference() {
        let mut compiler = JitCompiler::new();
        let bytecode = compile_formula("CLOSE");
        let optimized = compiler.compile(bytecode);
        let mut ctx = make_ctx(5);
        let result = compiler.execute(&optimized, &mut ctx).unwrap().final_value;
        for i in 0..5 {
            let expected = 10.0 + i as f64 * 0.15;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_complex_formula() {
        let mut compiler = JitCompiler::new();
        let source = "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); MA5 > MA10";
        let bytecode = compile_formula(source);
        let optimized = compiler.compile(bytecode);
        let mut ctx = make_ctx(30);
        let result = compiler.execute(&optimized, &mut ctx).unwrap();
        assert_eq!(result.final_value.len(), 30);
    }

    #[test]
    fn test_multiple_sources_independent_hot_tracking() {
        let mut compiler = JitCompiler::new().with_threshold(2);
        compiler.record_execution("source_a");
        compiler.record_execution("source_a");
        assert!(compiler.is_hot("source_a"));
        assert!(!compiler.is_hot("source_b"));
    }

    #[test]
    fn test_constant_folding_addition() {
        let mut compiler = JitCompiler::new();
        let bytecode = Bytecode {
            instructions: vec![
                OpCode::PushConst(10.0),
                OpCode::PushConst(20.0),
                OpCode::Add,
            ],
            source: "10 + 20".to_string(),
            output_names: vec![],
        };
        let optimized = compiler.optimize_bytecode(&bytecode);
        assert_eq!(optimized.instructions.len(), 1);
        match &optimized.instructions[0] {
            OpCode::PushConst(val) => assert!((*val - 30.0).abs() < 1e-10),
            other => panic!("Expected PushConst, got {:?}", other),
        }
    }

    #[test]
    fn test_constant_folding_complex() {
        let mut compiler = JitCompiler::new();
        let bytecode = Bytecode {
            instructions: vec![
                OpCode::PushConst(1.0),
                OpCode::PushConst(2.0),
                OpCode::PushConst(3.0),
                OpCode::Mul,
                OpCode::Add,
            ],
            source: "1 + 2 * 3".to_string(),
            output_names: vec![],
        };
        let optimized = compiler.optimize_bytecode(&bytecode);
        assert_eq!(optimized.instructions.len(), 1);
        match &optimized.instructions[0] {
            OpCode::PushConst(val) => assert!((*val - 7.0).abs() < 1e-10),
            other => panic!("Expected PushConst, got {:?}", other),
        }
    }

    #[test]
    fn test_dead_code_elimination() {
        let mut compiler = JitCompiler::new();
        let bytecode = Bytecode {
            instructions: vec![
                OpCode::PushConst(42.0),
                OpCode::StoreVar("UNUSED".to_string()),
                OpCode::PushConst(99.0),
            ],
            source: "test_dce".to_string(),
            output_names: vec![],
        };
        let optimized = compiler.optimize_bytecode(&bytecode);
        assert_eq!(optimized.instructions.len(), 1);
        match &optimized.instructions[0] {
            OpCode::PushConst(val) => assert!((*val - 99.0).abs() < 1e-10),
            other => panic!("Expected PushConst(99.0), got {:?}", other),
        }
    }

    #[test]
    fn test_optimized_execution_matches_regular() {
        let mut compiler = JitCompiler::new();
        let formulas = vec![
            "10 + 20",
            "CLOSE",
            "CLOSE * 2",
            "CLOSE + OPEN",
            "MA(CLOSE, 5)",
            "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); MA5 > MA10",
        ];

        for source in formulas {
            let bytecode = compile_formula(source);
            let optimized = compiler.compile(bytecode.clone());

            let ctx1 = make_ctx(30);
            let mut ctx2 = make_ctx(30);

            let mut vm = crate::formula::bytecode::BytecodeVM::new();
            let regular_result = vm.execute(&bytecode, &ctx1).unwrap();
            let jit_result = compiler.execute(&optimized, &mut ctx2).unwrap().final_value;

            assert_eq!(
                regular_result.final_value.len(),
                jit_result.len(),
                "Length mismatch for: {}",
                source
            );
            for i in 0..jit_result.len() {
                let reg = regular_result.final_value[i];
                let jit = jit_result[i];
                if reg.is_nan() {
                    assert!(
                        jit.is_nan(),
                        "Mismatch at {} for {}: reg=NaN, jit={}",
                        i,
                        source,
                        jit
                    );
                } else {
                    assert!(
                        (reg - jit).abs() < 1e-10,
                        "Mismatch at {} for {}: reg={}, jit={}",
                        i,
                        source,
                        reg,
                        jit
                    );
                }
            }
        }
    }

    #[test]
    fn test_hot_path_auto_compile() {
        let mut compiler = JitCompiler::new().with_threshold(3);
        let source = "CLOSE + OPEN";

        for _ in 0..4 {
            let bytecode = compile_formula(source);
            let mut ctx = make_ctx(10);
            let _ = compiler.compile_and_execute(bytecode, &mut ctx);
        }

        assert!(compiler.is_cached(source));

        let mut ctx = make_ctx(10);
        let bytecode = compile_formula(source);
        let result = compiler
            .compile_and_execute(bytecode, &mut ctx)
            .unwrap()
            .final_value;
        for i in 0..10 {
            let expected = (10.0 + i as f64 * 0.15) + (10.0 + i as f64 * 0.1);
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_inline_cache_speedup() {
        let mut compiler = JitCompiler::new();
        let bytecode = compile_formula("MA(CLOSE, 5)");
        let optimized = compiler.compile(bytecode);

        assert!(optimized.cached_call_count() > 0);

        let mut ctx = make_ctx(20);
        let result = compiler.execute(&optimized, &mut ctx).unwrap().final_value;
        assert_eq!(result.len(), 20);
        assert!(result[0].is_nan());
        assert!(!result[4].is_nan());
    }

    #[test]
    fn test_constant_folding_subtraction() {
        let mut compiler = JitCompiler::new();
        let bytecode = Bytecode {
            instructions: vec![
                OpCode::PushConst(100.0),
                OpCode::PushConst(37.0),
                OpCode::Sub,
            ],
            source: "100 - 37".to_string(),
            output_names: vec![],
        };
        let optimized = compiler.optimize_bytecode(&bytecode);
        assert_eq!(optimized.instructions.len(), 1);
        match &optimized.instructions[0] {
            OpCode::PushConst(val) => assert!((*val - 63.0).abs() < 1e-10),
            other => panic!("Expected PushConst, got {:?}", other),
        }
    }

    #[test]
    fn test_constant_folding_division_by_zero() {
        let mut compiler = JitCompiler::new();
        let bytecode = Bytecode {
            instructions: vec![OpCode::PushConst(10.0), OpCode::PushConst(0.0), OpCode::Div],
            source: "10 / 0".to_string(),
            output_names: vec![],
        };
        let optimized = compiler.optimize_bytecode(&bytecode);
        assert_eq!(optimized.instructions.len(), 1);
        match &optimized.instructions[0] {
            OpCode::PushConst(val) => assert!(val.is_nan()),
            other => panic!("Expected PushConst(NaN), got {:?}", other),
        }
    }

    #[test]
    fn test_constant_folding_comparison() {
        let mut compiler = JitCompiler::new();
        let bytecode = Bytecode {
            instructions: vec![OpCode::PushConst(5.0), OpCode::PushConst(3.0), OpCode::Gt],
            source: "5 > 3".to_string(),
            output_names: vec![],
        };
        let optimized = compiler.optimize_bytecode(&bytecode);
        assert_eq!(optimized.instructions.len(), 1);
        match &optimized.instructions[0] {
            OpCode::PushConst(val) => assert!((*val - 1.0).abs() < 1e-10),
            other => panic!("Expected PushConst(1.0), got {:?}", other),
        }
    }

    #[test]
    fn test_constant_folding_mixed_with_variables() {
        let mut compiler = JitCompiler::new();
        let bytecode = Bytecode {
            instructions: vec![
                OpCode::LoadData("CLOSE".to_string()),
                OpCode::PushConst(10.0),
                OpCode::PushConst(20.0),
                OpCode::Add,
                OpCode::Mul,
            ],
            source: "CLOSE * (10 + 20)".to_string(),
            output_names: vec![],
        };
        let optimized = compiler.optimize_bytecode(&bytecode);
        assert_eq!(optimized.instructions.len(), 3);
        assert!(matches!(&optimized.instructions[0], OpCode::LoadData(_)));
        match &optimized.instructions[1] {
            OpCode::PushConst(val) => assert!((*val - 30.0).abs() < 1e-10),
            other => panic!("Expected PushConst(30.0), got {:?}", other),
        }
        assert!(matches!(&optimized.instructions[2], OpCode::Mul));
    }

    #[test]
    fn test_dead_code_elimination_preserves_used_vars() {
        let mut compiler = JitCompiler::new();
        let bytecode = Bytecode {
            instructions: vec![
                OpCode::PushConst(42.0),
                OpCode::StoreVar("USED".to_string()),
                OpCode::LoadVar("USED".to_string()),
                OpCode::PushConst(1.0),
                OpCode::Add,
            ],
            source: "test_dce_preserve".to_string(),
            output_names: vec![],
        };
        let optimized = compiler.optimize_bytecode(&bytecode);
        assert_eq!(optimized.instructions.len(), 5);
    }

    #[test]
    fn test_dead_code_elimination_after_jump() {
        let mut compiler = JitCompiler::new();
        let bytecode = Bytecode {
            instructions: vec![
                OpCode::PushConst(1.0),
                OpCode::Jump(4),
                OpCode::PushConst(2.0),
                OpCode::PushConst(3.0),
                OpCode::PushConst(4.0),
            ],
            source: "test_jump_dce".to_string(),
            output_names: vec![],
        };
        let optimized = compiler.optimize_bytecode(&bytecode);
        assert!(optimized.instructions.len() < 5);
        assert!(matches!(&optimized.instructions[0], OpCode::PushConst(_)));
        assert!(matches!(&optimized.instructions[1], OpCode::Jump(_)));
    }

    #[test]
    fn test_constant_folding_power() {
        let mut compiler = JitCompiler::new();
        let bytecode = Bytecode {
            instructions: vec![OpCode::PushConst(2.0), OpCode::PushConst(10.0), OpCode::Pow],
            source: "2 ^ 10".to_string(),
            output_names: vec![],
        };
        let optimized = compiler.optimize_bytecode(&bytecode);
        assert_eq!(optimized.instructions.len(), 1);
        match &optimized.instructions[0] {
            OpCode::PushConst(val) => assert!((*val - 1024.0).abs() < 1e-10),
            other => panic!("Expected PushConst(1024.0), got {:?}", other),
        }
    }

    #[test]
    fn test_constant_folding_modulo() {
        let mut compiler = JitCompiler::new();
        let bytecode = Bytecode {
            instructions: vec![OpCode::PushConst(17.0), OpCode::PushConst(5.0), OpCode::Mod],
            source: "17 % 5".to_string(),
            output_names: vec![],
        };
        let optimized = compiler.optimize_bytecode(&bytecode);
        assert_eq!(optimized.instructions.len(), 1);
        match &optimized.instructions[0] {
            OpCode::PushConst(val) => assert!((*val - 2.0).abs() < 1e-10),
            other => panic!("Expected PushConst(2.0), got {:?}", other),
        }
    }

    #[test]
    fn test_optimized_execution_with_assignment() {
        let mut compiler = JitCompiler::new();
        let source = "UP := CLOSE + 1; UP * 2";
        let bytecode = compile_formula(source);
        let optimized = compiler.compile(bytecode);
        let mut ctx = make_ctx(5);
        let result = compiler.execute(&optimized, &mut ctx).unwrap().final_value;
        for i in 0..5 {
            let expected = (10.0 + i as f64 * 0.15 + 1.0) * 2.0;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_optimized_execution_comparison() {
        let mut compiler = JitCompiler::new();
        let bytecode = compile_formula("CLOSE > 10.5");
        let optimized = compiler.compile(bytecode);
        let mut ctx = make_ctx(5);
        let result = compiler.execute(&optimized, &mut ctx).unwrap().final_value;
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let expected = if close_val > 10.5 { 1.0 } else { 0.0 };
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_hot_path_uses_optimized_bytecode() {
        let mut compiler = JitCompiler::new().with_threshold(2);
        let source = "10 + 20";

        for _ in 0..3 {
            let bytecode = compile_formula(source);
            let mut ctx = make_ctx(5);
            let _ = compiler.compile_and_execute(bytecode, &mut ctx);
        }

        let cached = compiler.optimized_cache.get(source).unwrap();
        assert!(cached.is_hot());
        assert_eq!(cached.instruction_count(), 1);
        match &cached.instructions()[0] {
            OpCode::PushConst(val) => assert!((*val - 30.0).abs() < 1e-10),
            other => panic!("Expected folded PushConst, got {:?}", other),
        }
    }

    #[test]
    fn test_dead_code_elimination_with_function_call() {
        let mut compiler = JitCompiler::new();
        let bytecode = Bytecode {
            instructions: vec![
                OpCode::LoadData("CLOSE".to_string()),
                OpCode::PushConst(5.0),
                OpCode::Call {
                    name: "MA".to_string(),
                    arg_count: 2,
                },
                OpCode::StoreVar("UNUSED_MA".to_string()),
                OpCode::PushConst(42.0),
            ],
            source: "test_dce_func".to_string(),
            output_names: vec![],
        };
        let optimized = compiler.optimize_bytecode(&bytecode);
        assert_eq!(optimized.instructions.len(), 1);
        match &optimized.instructions[0] {
            OpCode::PushConst(val) => assert!((*val - 42.0).abs() < 1e-10),
            other => panic!("Expected PushConst(42.0), got {:?}", other),
        }
    }

    #[test]
    fn test_constant_folding_equality() {
        let mut compiler = JitCompiler::new();
        let bytecode = Bytecode {
            instructions: vec![OpCode::PushConst(5.0), OpCode::PushConst(5.0), OpCode::Eq],
            source: "5 == 5".to_string(),
            output_names: vec![],
        };
        let optimized = compiler.optimize_bytecode(&bytecode);
        assert_eq!(optimized.instructions.len(), 1);
        match &optimized.instructions[0] {
            OpCode::PushConst(val) => assert!((*val - 1.0).abs() < 1e-10),
            other => panic!("Expected PushConst(1.0), got {:?}", other),
        }
    }

    #[test]
    fn test_reset_clears_cache() {
        let mut compiler = JitCompiler::new().with_threshold(1);
        let source = "CLOSE";

        let bytecode = compile_formula(source);
        let mut ctx = make_ctx(5);
        let _ = compiler.compile_and_execute(bytecode, &mut ctx);
        assert!(compiler.is_cached(source));

        compiler.reset_hot_counts();
        assert!(!compiler.is_cached(source));
        assert_eq!(compiler.get_execution_count(source), 0);
    }

    #[test]
    fn test_inline_cache_nested_functions() {
        let mut compiler = JitCompiler::new();
        let bytecode = compile_formula("EMA(MA(CLOSE, 10), 12)");
        let optimized = compiler.compile(bytecode);

        assert!(optimized.cached_call_count() >= 2);

        let mut ctx = make_ctx(30);
        let result = compiler.execute(&optimized, &mut ctx).unwrap().final_value;
        assert_eq!(result.len(), 30);
    }

    #[test]
    fn test_preallocated_stack_capacity() {
        let mut compiler = JitCompiler::new();
        let bytecode = compile_formula("CLOSE + OPEN + HIGH + LOW");
        let optimized = compiler.compile(bytecode);
        assert!(optimized.buffer_size() >= 2);

        let mut ctx = make_ctx(5);
        let result = compiler.execute(&optimized, &mut ctx).unwrap().final_value;
        for i in 0..5 {
            let expected = (10.0 + i as f64 * 0.15)
                + (10.0 + i as f64 * 0.1)
                + (11.0 + i as f64 * 0.2)
                + (9.0 + i as f64 * 0.1);
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }
}
