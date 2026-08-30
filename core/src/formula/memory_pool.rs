use ndarray::{Array1, ArrayView1};
use std::collections::HashMap;

pub struct BufferPool {
    array_pool_by_size: HashMap<usize, Vec<Array1<f64>>>,
}

impl BufferPool {
    pub fn new(_size_hint: usize, _initial_count: usize) -> Self {
        Self {
            array_pool_by_size: HashMap::new(),
        }
    }

    pub fn get_buffer(&mut self, size: usize) -> Array1<f64> {
        if let Some(buffers) = self.array_pool_by_size.get_mut(&size) {
            buffers.pop().unwrap_or_else(|| Array1::zeros(size))
        } else {
            Array1::zeros(size)
        }
    }

    pub fn return_buffer(&mut self, buffer: Array1<f64>) {
        self.array_pool_by_size
            .entry(buffer.len())
            .or_insert_with(Vec::new)
            .push(buffer);
    }

    pub fn clear(&mut self) {
        self.array_pool_by_size.clear();
    }
}

fn resolve_ohlcv_name(name: &str) -> Option<u8> {
    let bytes = name.as_bytes();
    match bytes.len() {
        1 => match bytes[0] {
            b'C' | b'c' => Some(0),
            b'O' | b'o' => Some(1),
            b'H' | b'h' => Some(2),
            b'L' | b'l' => Some(3),
            b'V' | b'v' => Some(4),
            _ => None,
        },
        3 if name.eq_ignore_ascii_case("LOW") => Some(3),
        4 if name.eq_ignore_ascii_case("OPEN") => Some(1),
        4 if name.eq_ignore_ascii_case("HIGH") => Some(2),
        5 if name.eq_ignore_ascii_case("CLOSE") => Some(0),
        6 if name.eq_ignore_ascii_case("VOLUME") => Some(4),
        _ => None,
    }
}

pub struct ZeroCopyContext<'a> {
    pub open: ArrayView1<'a, f64>,
    pub high: ArrayView1<'a, f64>,
    pub low: ArrayView1<'a, f64>,
    pub close: ArrayView1<'a, f64>,
    pub volume: ArrayView1<'a, f64>,
    pub variables: HashMap<String, Array1<f64>>,
    pub buffer_pool: BufferPool,
}

impl<'a> ZeroCopyContext<'a> {
    pub fn new(
        open: ArrayView1<'a, f64>,
        high: ArrayView1<'a, f64>,
        low: ArrayView1<'a, f64>,
        close: ArrayView1<'a, f64>,
        volume: ArrayView1<'a, f64>,
    ) -> Self {
        let data_len = open.len();
        Self {
            open,
            high,
            low,
            close,
            volume,
            variables: HashMap::new(),
            buffer_pool: BufferPool::new(data_len, 0),
        }
    }

    pub fn with_pool_capacity(mut self, initial_count: usize) -> Self {
        self.buffer_pool = BufferPool::new(self.data_len(), initial_count);
        self
    }

    pub fn data_len(&self) -> usize {
        self.close.len()
    }

    pub fn get_data_view(&self, name: &str) -> Option<ArrayView1<'_, f64>> {
        match resolve_ohlcv_name(name)? {
            0 => Some(self.close.view()),
            1 => Some(self.open.view()),
            2 => Some(self.high.view()),
            3 => Some(self.low.view()),
            4 => Some(self.volume.view()),
            _ => None,
        }
    }

    pub fn get_data_as_slice(&self, name: &str) -> Option<&[f64]> {
        match resolve_ohlcv_name(name)? {
            0 => Some(self.close.as_slice().unwrap()),
            1 => Some(self.open.as_slice().unwrap()),
            2 => Some(self.high.as_slice().unwrap()),
            3 => Some(self.low.as_slice().unwrap()),
            4 => Some(self.volume.as_slice().unwrap()),
            _ => None,
        }
    }

    pub fn set_variable(&mut self, name: String, value: Array1<f64>) {
        self.variables.insert(name, value);
    }

    pub fn get_variable(&self, name: &str) -> Option<&Array1<f64>> {
        self.variables.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array1;

    type OhlcvArrays = (
        Array1<f64>,
        Array1<f64>,
        Array1<f64>,
        Array1<f64>,
        Array1<f64>,
    );

    fn make_arrays(len: usize) -> OhlcvArrays {
        let open = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.1).collect());
        let high = Array1::from_vec((0..len).map(|i| 11.0 + i as f64 * 0.2).collect());
        let low = Array1::from_vec((0..len).map(|i| 9.0 + i as f64 * 0.1).collect());
        let close = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.15).collect());
        let volume = Array1::from_vec((0..len).map(|i| 1000.0 + i as f64 * 10.0).collect());
        (open, high, low, close, volume)
    }

    #[test]
    fn test_buffer_pool_new_is_empty() {
        let pool = BufferPool::new(100, 5);
        assert!(pool.array_pool_by_size.is_empty());
    }

    #[test]
    fn test_zero_copy_context_new() {
        let (open, high, low, close, volume) = make_arrays(10);
        let ctx = ZeroCopyContext::new(
            open.view(),
            high.view(),
            low.view(),
            close.view(),
            volume.view(),
        );
        assert_eq!(ctx.data_len(), 10);
    }

    #[test]
    fn test_zero_copy_context_data_views() {
        let (open, high, low, close, volume) = make_arrays(5);
        let ctx = ZeroCopyContext::new(
            open.view(),
            high.view(),
            low.view(),
            close.view(),
            volume.view(),
        );

        let close_view = ctx.get_data_view("CLOSE").unwrap();
        for i in 0..5 {
            let expected = 10.0 + i as f64 * 0.15;
            assert!((close_view[i] - expected).abs() < 1e-10);
        }

        let open_view = ctx.get_data_view("OPEN").unwrap();
        for i in 0..5 {
            let expected = 10.0 + i as f64 * 0.1;
            assert!((open_view[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_zero_copy_context_data_views_shortcuts() {
        let (open, high, low, close, volume) = make_arrays(5);
        let ctx = ZeroCopyContext::new(
            open.view(),
            high.view(),
            low.view(),
            close.view(),
            volume.view(),
        );

        assert!(ctx.get_data_view("C").is_some());
        assert!(ctx.get_data_view("O").is_some());
        assert!(ctx.get_data_view("H").is_some());
        assert!(ctx.get_data_view("L").is_some());
        assert!(ctx.get_data_view("V").is_some());
        assert!(ctx.get_data_view("UNKNOWN").is_none());
    }

    #[test]
    fn test_zero_copy_context_data_as_slice() {
        let (open, high, low, close, volume) = make_arrays(5);
        let ctx = ZeroCopyContext::new(
            open.view(),
            high.view(),
            low.view(),
            close.view(),
            volume.view(),
        );

        let close_slice = ctx.get_data_as_slice("CLOSE").unwrap();
        assert_eq!(close_slice.len(), 5);
    }

    #[test]
    fn test_zero_copy_context_variables() {
        let (open, high, low, close, volume) = make_arrays(5);
        let mut ctx = ZeroCopyContext::new(
            open.view(),
            high.view(),
            low.view(),
            close.view(),
            volume.view(),
        );

        let var = Array1::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        ctx.set_variable("TEST".to_string(), var.clone());

        let retrieved = ctx.get_variable("TEST").unwrap();
        for i in 0..5 {
            assert!((retrieved[i] - var[i]).abs() < 1e-10);
        }

        assert!(ctx.get_variable("NONEXISTENT").is_none());
    }

    #[test]
    fn test_zero_copy_context_with_pool_capacity() {
        let (open, high, low, close, volume) = make_arrays(10);
        let ctx = ZeroCopyContext::new(
            open.view(),
            high.view(),
            low.view(),
            close.view(),
            volume.view(),
        )
        .with_pool_capacity(8);
        assert!(ctx.buffer_pool.array_pool_by_size.is_empty());
    }

    #[test]
    fn test_zero_copy_no_clone_of_input_data() {
        let (open, high, low, close, volume) = make_arrays(5);
        let open_ptr = open.as_ptr();
        let close_ptr = close.as_ptr();

        let ctx = ZeroCopyContext::new(
            open.view(),
            high.view(),
            low.view(),
            close.view(),
            volume.view(),
        );

        assert_eq!(ctx.open.as_ptr(), open_ptr);
        assert_eq!(ctx.close.as_ptr(), close_ptr);
    }

    #[test]
    fn test_buffer_pool_get_buffer_new() {
        let mut pool = BufferPool::new(10, 0);
        let buf = pool.get_buffer(5);
        assert_eq!(buf.len(), 5);
    }

    #[test]
    fn test_buffer_pool_return_and_reuse() {
        let mut pool = BufferPool::new(10, 0);
        let mut buf = pool.get_buffer(5);
        for i in 0..5 {
            buf[i] = 42.0;
        }
        pool.return_buffer(buf);
        let reused = pool.get_buffer(5);
        assert_eq!(reused.len(), 5);
    }

    #[test]
    fn test_buffer_pool_get_buffer_different_sizes() {
        let mut pool = BufferPool::new(10, 0);
        let buf5 = pool.get_buffer(5);
        let buf10 = pool.get_buffer(10);
        pool.return_buffer(buf5);
        pool.return_buffer(buf10);
        let reused5 = pool.get_buffer(5);
        assert_eq!(reused5.len(), 5);
        let reused10 = pool.get_buffer(10);
        assert_eq!(reused10.len(), 10);
    }

    #[test]
    fn test_buffer_pool_clear() {
        let mut pool = BufferPool::new(10, 3);
        let buf = pool.get_buffer(10);
        pool.return_buffer(buf);
        assert!(!pool.array_pool_by_size.is_empty());
        pool.clear();
        assert!(pool.array_pool_by_size.is_empty());
    }
}
