//! 黄金测试公共模块
//!
//! 该目录被 Rust 集成测试机制识别为共享模块：`tests/common/` 下的所有文件
//! 不会作为独立的 test crate 编译，但能被同目录其他测试文件通过
//! `mod common;` 引用，从而避免在每个测试文件中重复工具函数。
//!
//! 子模块：
//! - [`golden_loader`] — 黄金 CSV 加载与容差断言 helper

pub mod golden_loader;
