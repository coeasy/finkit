use finkit::formula::compiler::{CompiledFormula, FormulaCache, FormulaCompiler};
use finkit::formula::engine::FormulaEngine;
use finkit::formula::types::FormulaContext;
use ndarray::Array1;

fn make_ctx(len: usize) -> FormulaContext {
    let open = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.1).collect());
    let high = Array1::from_vec((0..len).map(|i| 11.0 + i as f64 * 0.2).collect());
    let low = Array1::from_vec((0..len).map(|i| 9.0 + i as f64 * 0.1).collect());
    let close = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.15).collect());
    let volume = Array1::from_vec((0..len).map(|i| 1000.0 + i as f64 * 10.0).collect());
    FormulaContext::new(open, high, low, close, volume, None)
}

mod cache_hit_tests {
    use super::*;

    #[test]
    fn test_cache_hit_basic() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(10);

        let result1 = engine.eval("CLOSE + OPEN", &mut ctx).unwrap();
        assert!(engine.cache_hit("CLOSE + OPEN"));

        let result2 = engine.eval("CLOSE + OPEN", &mut ctx).unwrap();

        for i in 0..10 {
            assert!((result1[i] - result2[i]).abs() < 1e-10);
        }
    }

    #[test]
    fn test_cache_hit_complex_formula() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);

        let formula = "MA(CLOSE, 5) + MA(OPEN, 10)";

        engine.eval(formula, &mut ctx).unwrap();
        assert!(engine.cache_hit(formula));

        engine.eval(formula, &mut ctx).unwrap();
        assert_eq!(engine.cache_size(), 1);
    }

    #[test]
    fn test_cache_hit_multiple_formulas() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(10);

        let formulas = vec![
            "CLOSE + OPEN",
            "HIGH - LOW",
            "VOLUME / 1000",
            "MA(CLOSE, 5)",
        ];

        for formula in &formulas {
            engine.eval(formula, &mut ctx).unwrap();
        }

        assert_eq!(engine.cache_size(), 4);

        for formula in &formulas {
            assert!(engine.cache_hit(formula));
        }
    }

    #[test]
    fn test_cache_hit_with_whitespace() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(10);

        engine.eval("CLOSE + OPEN", &mut ctx).unwrap();
        assert!(engine.cache_hit("CLOSE + OPEN"));

        assert!(!engine.cache_hit("CLOSE+OPEN"));
        assert!(!engine.cache_hit("CLOSE  +  OPEN"));
    }
}

mod cache_miss_tests {
    use super::*;

    #[test]
    fn test_cache_miss_new_formula() {
        let mut engine = FormulaEngine::new();

        assert!(!engine.cache_hit("CLOSE + OPEN"));
    }

    #[test]
    fn test_cache_miss_after_clear() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(10);

        engine.eval("CLOSE + OPEN", &mut ctx).unwrap();
        assert!(engine.cache_hit("CLOSE + OPEN"));

        engine.clear_cache();

        assert!(!engine.cache_hit("CLOSE + OPEN"));
    }

    #[test]
    fn test_cache_miss_different_formulas() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(10);

        engine.eval("CLOSE + OPEN", &mut ctx).unwrap();

        assert!(!engine.cache_hit("CLOSE - OPEN"));
        assert!(!engine.cache_hit("OPEN + CLOSE"));
    }
}

mod cache_invalidation_tests {
    use super::*;

    #[test]
    fn test_clear_cache() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(10);

        engine.eval("CLOSE + 1", &mut ctx).unwrap();
        engine.eval("CLOSE + 2", &mut ctx).unwrap();
        engine.eval("CLOSE + 3", &mut ctx).unwrap();

        assert_eq!(engine.cache_size(), 3);

        engine.clear_cache();

        assert_eq!(engine.cache_size(), 0);
    }

    #[test]
    fn test_cache_eviction_lru() {
        let mut engine = FormulaEngine::with_cache_size(3);
        let mut ctx = make_ctx(10);

        engine.eval("A: 1", &mut ctx).unwrap();
        engine.eval("B: 2", &mut ctx).unwrap();
        engine.eval("C: 3", &mut ctx).unwrap();

        assert_eq!(engine.cache_size(), 3);

        engine.eval("D: 4", &mut ctx).unwrap();

        assert_eq!(engine.cache_size(), 3);
        assert!(!engine.cache_hit("A: 1"));
        assert!(engine.cache_hit("B: 2"));
        assert!(engine.cache_hit("C: 3"));
        assert!(engine.cache_hit("D: 4"));
    }

    #[test]
    fn test_cache_lru_access_updates_order() {
        let mut engine = FormulaEngine::with_cache_size(3);
        let mut ctx = make_ctx(10);

        engine.eval("A: 1", &mut ctx).unwrap();
        engine.eval("B: 2", &mut ctx).unwrap();
        engine.eval("C: 3", &mut ctx).unwrap();

        engine.eval("A: 1", &mut ctx).unwrap();

        engine.eval("D: 4", &mut ctx).unwrap();

        assert!(engine.cache_hit("A: 1"));
        assert!(!engine.cache_hit("B: 2"));
        assert!(engine.cache_hit("C: 3"));
        assert!(engine.cache_hit("D: 4"));
    }

    #[test]
    fn test_cache_capacity_default() {
        let engine = FormulaEngine::new();
        assert_eq!(engine.cache_size(), 0);
    }

    #[test]
    fn test_cache_capacity_custom() {
        let engine = FormulaEngine::with_cache_size(500);
        assert_eq!(engine.cache_size(), 0);
    }
}

mod cache_performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_cache_hit_zero_compilation_overhead() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(100);

        let complex_formula = "MA(CLOSE, 5) + MA(HIGH, 10) - MA(LOW, 20) * 0.5 + VOLUME / 10000";

        let start_compile = Instant::now();
        engine.eval(complex_formula, &mut ctx).unwrap();
        let compile_time = start_compile.elapsed();

        let iterations = 1000;
        let start_cached = Instant::now();
        for _ in 0..iterations {
            engine.eval(complex_formula, &mut ctx).unwrap();
        }
        let cached_total_time = start_cached.elapsed();
        let avg_cached_time = cached_total_time / iterations;

        println!("Initial compile time: {:?}", compile_time);
        println!("Average cached eval time: {:?}", avg_cached_time);
        println!(
            "Total cached time for {} iterations: {:?}",
            iterations, cached_total_time
        );

        assert!(
            avg_cached_time < compile_time,
            "Cached evaluation should be faster than initial compilation"
        );
    }

    #[test]
    fn test_cache_benefit_repeated_evaluations() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(100);

        let formulas = vec![
            "MA(CLOSE, 5)",
            "MA(CLOSE, 10)",
            "MA(CLOSE, 20)",
            "EMA(CLOSE, 12)",
            "EMA(CLOSE, 26)",
        ];

        let start_no_cache = Instant::now();
        for _ in 0..100 {
            let mut fresh_engine = FormulaEngine::new();
            for formula in &formulas {
                fresh_engine.eval(formula, &mut ctx).unwrap();
            }
        }
        let no_cache_time = start_no_cache.elapsed();

        let start_with_cache = Instant::now();
        for _ in 0..100 {
            for formula in &formulas {
                engine.eval(formula, &mut ctx).unwrap();
            }
        }
        let with_cache_time = start_with_cache.elapsed();

        println!("Without cache reuse: {:?}", no_cache_time);
        println!("With cache reuse: {:?}", with_cache_time);

        assert!(
            with_cache_time < no_cache_time,
            "Cache reuse should be faster than recompiling"
        );
    }
}

mod cache_data_structure_tests {
    use super::*;
    use finkit::formula::ast::AstNode;

    #[test]
    fn test_cache_insert_and_retrieve() {
        let mut cache = FormulaCache::new(10);
        let formula = CompiledFormula {
            ast: AstNode::Number(42.0),
            source: "42".to_string(),
        };

        cache.insert("42", formula);

        assert!(cache.contains("42"));
        let retrieved = cache.get("42");
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_cache_hash_consistency() {
        let mut cache = FormulaCache::new(10);
        let formula = CompiledFormula {
            ast: AstNode::Number(42.0),
            source: "test_formula".to_string(),
        };

        cache.insert("test_formula", formula);

        assert!(cache.contains("test_formula"));
        assert!(cache.get("test_formula").is_some());
    }

    #[test]
    fn test_cache_different_sources_different_hashes() {
        let mut cache = FormulaCache::new(10);

        cache.insert(
            "formula1",
            CompiledFormula {
                ast: AstNode::Number(1.0),
                source: "formula1".to_string(),
            },
        );
        cache.insert(
            "formula2",
            CompiledFormula {
                ast: AstNode::Number(2.0),
                source: "formula2".to_string(),
            },
        );

        assert!(cache.contains("formula1"));
        assert!(cache.contains("formula2"));

        let f1 = cache.get_cloned("formula1").unwrap();
        let f2 = cache.get_cloned("formula2").unwrap();

        if let (AstNode::Number(n1), AstNode::Number(n2)) = (&f1.ast, &f2.ast) {
            assert_ne!(n1, n2);
        } else {
            panic!("Expected Number nodes");
        }
    }

    #[test]
    fn test_cache_update_existing() {
        let mut cache = FormulaCache::new(10);

        cache.insert(
            "test",
            CompiledFormula {
                ast: AstNode::Number(1.0),
                source: "test".to_string(),
            },
        );

        cache.insert(
            "test",
            CompiledFormula {
                ast: AstNode::Number(2.0),
                source: "test_updated".to_string(),
            },
        );

        assert_eq!(cache.len(), 1);
        let retrieved = cache.get("test").unwrap();
        if let AstNode::Number(n) = &retrieved.ast {
            assert_eq!(*n, 2.0);
        } else {
            panic!("Expected Number node");
        }
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

        let removed_again = cache.remove("test");
        assert!(removed_again.is_none());
    }

    #[test]
    fn test_cache_capacity() {
        let cache = FormulaCache::new(100);
        assert_eq!(cache.capacity(), 100);
    }

    #[test]
    fn test_cache_is_empty() {
        let cache = FormulaCache::new(10);
        assert!(cache.is_empty());

        let mut cache = FormulaCache::new(10);
        cache.insert(
            "test",
            CompiledFormula {
                ast: AstNode::Number(1.0),
                source: "test".to_string(),
            },
        );
        assert!(!cache.is_empty());
    }
}

mod compiler_cache_tests {
    use super::*;

    #[test]
    fn test_compiler_cache_hit() {
        let mut compiler = FormulaCompiler::new(10);
        let mut ctx = make_ctx(10);

        compiler
            .compile_and_execute("CLOSE + OPEN", &mut ctx)
            .unwrap();
        assert!(compiler.cache().contains("CLOSE + OPEN"));

        compiler
            .compile_and_execute("CLOSE + OPEN", &mut ctx)
            .unwrap();
        assert_eq!(compiler.cache().len(), 1);
    }

    #[test]
    fn test_compiler_cache_miss() {
        let compiler = FormulaCompiler::new(10);
        assert!(!compiler.cache().contains("CLOSE + OPEN"));
    }

    #[test]
    fn test_compiler_cache_clear() {
        let mut compiler = FormulaCompiler::new(10);
        let mut ctx = make_ctx(10);

        compiler.compile_and_execute("CLOSE + 1", &mut ctx).unwrap();
        compiler.compile_and_execute("CLOSE + 2", &mut ctx).unwrap();

        assert_eq!(compiler.cache().len(), 2);

        compiler.cache_mut().clear();

        assert_eq!(compiler.cache().len(), 0);
    }

    #[test]
    fn test_compiler_cache_eviction() {
        let mut compiler = FormulaCompiler::new(2);
        let mut ctx = make_ctx(10);

        compiler.compile_and_execute("A: 1", &mut ctx).unwrap();
        compiler.compile_and_execute("B: 2", &mut ctx).unwrap();
        compiler.compile_and_execute("C: 3", &mut ctx).unwrap();

        assert_eq!(compiler.cache().len(), 2);
        assert!(!compiler.cache().contains("A: 1"));
    }
}
