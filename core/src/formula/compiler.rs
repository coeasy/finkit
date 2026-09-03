use crate::formula::ast::AstNode;
use crate::formula::executor::FormulaExecutor;
use crate::formula::optimizer::FormulaOptimizer;
use crate::formula::parser::parse_formula;
use crate::formula::types::*;
use ahash::AHasher;
use lru::LruCache;
use ndarray::Array1;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;

/// 编译后的公式
#[derive(Debug, Clone)]
pub struct CompiledFormula {
    pub ast: AstNode,
    pub source: String,
}

/// 计算字符串的hash值
#[inline]
fn compute_hash(source: &str) -> u64 {
    let mut hasher = AHasher::default();
    source.hash(&mut hasher);
    hasher.finish()
}

/// 公式缓存管理器（O(1) 真 LRU 策略）
pub struct FormulaCache {
    cache: LruCache<u64, CompiledFormula>,
    counter: u64,
    max_size: usize,
}

impl FormulaCache {
    pub fn new(max_size: usize) -> Self {
        // LruCache::new 要求容量 > 0；max_size = 0 时退化为容量 1，避免 panic
        let capacity = NonZeroUsize::new(max_size.max(1)).expect("max_size.max(1) > 0");
        Self {
            cache: LruCache::new(capacity),
            counter: 0,
            max_size,
        }
    }

    pub fn get(&mut self, source: &str) -> Option<&CompiledFormula> {
        let hash = compute_hash(source);
        // 先更新计数再返回引用，避免与返回的 `&CompiledFormula` 借用冲突
        self.counter += 1;
        // Hash 只是索引；必须再次比较 source，避免极低概率的 hash 碰撞
        match self.cache.get(&hash) {
            Some(formula) if formula.source == source => Some(formula),
            _ => None,
        }
    }

    pub fn get_cloned(&mut self, source: &str) -> Option<CompiledFormula> {
        let hash = compute_hash(source);
        self.counter += 1;
        // Hash 只是索引；必须再次比较 source，避免返回碰撞项
        self.cache
            .get(&hash)
            .filter(|formula| formula.source == source)
            .cloned()
    }

    pub fn insert(&mut self, source: &str, formula: CompiledFormula) {
        let hash = compute_hash(source);
        self.counter += 1;
        // LruCache::put 在超过容量时自动驱逐最久未使用的条目
        self.cache.put(hash, formula);
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    pub fn contains(&self, source: &str) -> bool {
        let hash = compute_hash(source);
        self.cache
            .peek(&hash)
            .is_some_and(|formula| formula.source == source)
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn capacity(&self) -> usize {
        self.max_size
    }

    pub fn remove(&mut self, source: &str) -> Option<CompiledFormula> {
        let hash = compute_hash(source);
        if self
            .cache
            .peek(&hash)
            .is_some_and(|formula| formula.source == source)
        {
            self.cache.pop(&hash)
        } else {
            None
        }
    }
}

/// 公式编译器
pub struct FormulaCompiler {
    executor: FormulaExecutor,
    cache: FormulaCache,
}

impl FormulaCompiler {
    pub fn new(cache_size: usize) -> Self {
        Self {
            executor: FormulaExecutor::new(),
            cache: FormulaCache::new(cache_size),
        }
    }

    pub fn compile(&mut self, source: &str) -> Result<CompiledFormula, FormulaError> {
        if let Some(formula) = self.cache.get_cloned(source) {
            return Ok(formula);
        }

        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        let ast = FormulaOptimizer::optimize(&ast);
        let formula = CompiledFormula {
            ast,
            source: source.to_string(),
        };

        self.cache.insert(source, formula.clone());

        Ok(formula)
    }

    pub fn execute(
        &self,
        formula: &CompiledFormula,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        self.executor.execute(&formula.ast, ctx)
    }

    pub fn compile_and_execute(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let formula = self.compile(source)?;
        self.execute(&formula, ctx)
    }

    pub fn cache(&self) -> &FormulaCache {
        &self.cache
    }

    pub fn cache_mut(&mut self) -> &mut FormulaCache {
        &mut self.cache
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx(len: usize) -> FormulaContext {
        let open = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.1).collect());
        let high = Array1::from_vec((0..len).map(|i| 11.0 + i as f64 * 0.2).collect());
        let low = Array1::from_vec((0..len).map(|i| 9.0 + i as f64 * 0.1).collect());
        let close = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.15).collect());
        let volume = Array1::from_vec((0..len).map(|i| 1000.0 + i as f64 * 10.0).collect());
        FormulaContext::new(open, high, low, close, volume, None)
    }

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = FormulaCache::new(10);
        let formula = CompiledFormula {
            ast: AstNode::Number(42.0),
            source: "42".to_string(),
        };
        cache.insert("42", formula);
        assert!(cache.get("42").is_some());
        assert!(cache.contains("42"));
    }

    #[test]
    fn test_cache_rejects_hash_collision_entry() {
        let mut cache = FormulaCache::new(10);
        let requested = "requested";
        let collision = CompiledFormula {
            ast: AstNode::Number(99.0),
            source: "different-source".to_string(),
        };
        cache.cache.put(compute_hash(requested), collision);

        assert!(cache.get(requested).is_none());
        assert!(cache.get_cloned(requested).is_none());
        assert!(!cache.contains(requested));
        assert!(cache.remove(requested).is_none());
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = FormulaCache::new(10);
        assert!(cache.get("nonexistent").is_none());
        assert!(!cache.contains("nonexistent"));
    }

    #[test]
    fn test_cache_max_size_eviction() {
        let mut cache = FormulaCache::new(2);
        cache.insert(
            "1",
            CompiledFormula {
                ast: AstNode::Number(1.0),
                source: "1".to_string(),
            },
        );
        cache.insert(
            "2",
            CompiledFormula {
                ast: AstNode::Number(2.0),
                source: "2".to_string(),
            },
        );
        cache.insert(
            "3",
            CompiledFormula {
                ast: AstNode::Number(3.0),
                source: "3".to_string(),
            },
        );

        assert_eq!(cache.len(), 2);
        assert!(cache.get("3").is_some());
        assert!(!cache.contains("1"));
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = FormulaCache::new(3);
        cache.insert(
            "a",
            CompiledFormula {
                ast: AstNode::Number(1.0),
                source: "a".to_string(),
            },
        );
        cache.insert(
            "b",
            CompiledFormula {
                ast: AstNode::Number(2.0),
                source: "b".to_string(),
            },
        );
        cache.insert(
            "c",
            CompiledFormula {
                ast: AstNode::Number(3.0),
                source: "c".to_string(),
            },
        );

        cache.get("a");

        cache.insert(
            "d",
            CompiledFormula {
                ast: AstNode::Number(4.0),
                source: "d".to_string(),
            },
        );

        assert!(cache.contains("a"));
        assert!(!cache.contains("b"));
        assert!(cache.contains("c"));
        assert!(cache.contains("d"));
    }

    #[test]
    fn test_cache_lru_eviction_full_workflow() {
        // 验证 Task 6 真 LRU：插入 100 条、连续访问前 50 条、再插入 50 条新条目，
        // 应该驱逐未访问的后 50 条；前 50 条与新 50 条都应保留。
        let mut cache = FormulaCache::new(100);
        for i in 0..100 {
            let src = format!("s{i}");
            cache.insert(
                &src,
                CompiledFormula {
                    ast: AstNode::Number(i as f64),
                    source: src.clone(),
                },
            );
        }
        assert_eq!(cache.len(), 100);

        // 连续访问前 50 个 source，把它们提升为最近使用
        for i in 0..50 {
            let src = format!("s{i}");
            assert!(cache.get_cloned(&src).is_some());
        }

        // 插入 50 个新 source，触发 50 次驱逐
        for i in 100..150 {
            let src = format!("s{i}");
            cache.insert(
                &src,
                CompiledFormula {
                    ast: AstNode::Number(i as f64),
                    source: src.clone(),
                },
            );
        }
        assert_eq!(cache.len(), 100);

        // 前 50 个（被访问过）必须仍在
        for i in 0..50 {
            let src = format!("s{i}");
            assert!(
                cache.contains(&src),
                "recently-used entry s{i} should still be in cache"
            );
        }
        // 后 50 个（未访问）必须被驱逐
        for i in 50..100 {
            let src = format!("s{i}");
            assert!(
                !cache.contains(&src),
                "least-recently-used entry s{i} should have been evicted"
            );
        }
        // 新插入的 50 个必须都在
        for i in 100..150 {
            let src = format!("s{i}");
            assert!(cache.contains(&src));
        }
    }

    #[test]
    fn test_cache_lru_access_promotes_recency() {
        // 容量 3：插入 a/b/c，访问 a，插入 d：应驱逐 b（最久未用），保留 a/c/d。
        let mut cache = FormulaCache::new(3);
        cache.insert(
            "a",
            CompiledFormula {
                ast: AstNode::Number(1.0),
                source: "a".to_string(),
            },
        );
        cache.insert(
            "b",
            CompiledFormula {
                ast: AstNode::Number(2.0),
                source: "b".to_string(),
            },
        );
        cache.insert(
            "c",
            CompiledFormula {
                ast: AstNode::Number(3.0),
                source: "c".to_string(),
            },
        );

        // 通过 get_cloned 提升 a 的 LRU 顺序
        assert!(cache.get_cloned("a").is_some());

        cache.insert(
            "d",
            CompiledFormula {
                ast: AstNode::Number(4.0),
                source: "d".to_string(),
            },
        );

        assert_eq!(cache.len(), 3);
        assert!(cache.contains("a"), "a was just touched, must stay");
        assert!(!cache.contains("b"), "b is now LRU, must be evicted");
        assert!(cache.contains("c"));
        assert!(cache.contains("d"));
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = FormulaCache::new(10);
        cache.insert(
            "1",
            CompiledFormula {
                ast: AstNode::Number(1.0),
                source: "1".to_string(),
            },
        );
        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_remove() {
        let mut cache = FormulaCache::new(10);
        cache.insert(
            "test",
            CompiledFormula {
                ast: AstNode::Number(1.0),
                source: "test".to_string(),
            },
        );
        assert!(cache.contains("test"));

        let removed = cache.remove("test");
        assert!(removed.is_some());
        assert!(!cache.contains("test"));
    }

    #[test]
    fn test_cache_capacity() {
        let cache = FormulaCache::new(100);
        assert_eq!(cache.capacity(), 100);
    }

    #[test]
    fn test_cache_get_cloned() {
        let mut cache = FormulaCache::new(10);
        cache.insert(
            "42",
            CompiledFormula {
                ast: AstNode::Number(42.0),
                source: "42".to_string(),
            },
        );

        let cloned = cache.get_cloned("42");
        assert!(cloned.is_some());
        assert_eq!(cloned.unwrap().source, "42");
    }

    #[test]
    fn test_compiler_compile_simple() {
        let mut compiler = FormulaCompiler::new(10);
        let mut ctx = make_ctx(5);
        let result = compiler.compile_and_execute("10 + 20", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 30.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_compiler_uses_cache() {
        let mut compiler = FormulaCompiler::new(10);
        let mut ctx = make_ctx(5);

        compiler
            .compile_and_execute("CLOSE + OPEN", &mut ctx)
            .unwrap();
        assert!(compiler.cache().contains("CLOSE + OPEN"));

        compiler
            .compile_and_execute("CLOSE + OPEN", &mut ctx)
            .unwrap();
    }

    #[test]
    fn test_compiler_invalid_formula() {
        let mut compiler = FormulaCompiler::new(10);
        let mut ctx = make_ctx(5);
        let result = compiler.compile_and_execute("CLOSE +", &mut ctx);
        assert!(result.is_err());
    }
}
