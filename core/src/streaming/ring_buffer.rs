use std::fmt;

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RingBuffer<T> {
    buf: Vec<T>,
    head: usize,
    len: usize,
}

impl<T: Clone + Default> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "RingBuffer capacity must be > 0");
        Self {
            buf: vec![T::default(); capacity],
            head: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, val: T) -> Option<T> {
        let evicted = if self.len == self.capacity() {
            Some(self.buf[self.head].clone())
        } else {
            None
        };

        self.buf[self.head] = val;
        self.head += 1;
        if self.head == self.capacity() {
            self.head = 0;
        }

        if self.len < self.capacity() {
            self.len += 1;
        }

        evicted
    }

    fn physical_index(&self, i: usize) -> usize {
        if self.len < self.capacity() {
            i
        } else {
            (self.head + i) % self.capacity()
        }
    }

    pub fn get(&self, i: usize) -> &T {
        assert!(i < self.len, "index {} out of bounds (len={})", i, self.len);
        &self.buf[self.physical_index(i)]
    }

    pub fn get_mut(&mut self, i: usize) -> &mut T {
        assert!(i < self.len, "index {} out of bounds (len={})", i, self.len);
        let idx = self.physical_index(i);
        &mut self.buf[idx]
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    pub fn is_full(&self) -> bool {
        self.len == self.capacity()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        let cap = self.capacity();
        let start = if self.len < cap { 0 } else { self.head };
        let len = self.len;
        (0..len).map(move |i| &self.buf[(start + i) % cap])
    }

    pub fn sum(&self) -> f64
    where
        T: Into<f64> + Copy,
    {
        self.iter().map(|&v| v.into()).sum()
    }
}

impl<T: Clone + Default + fmt::Debug> fmt::Debug for RingBuffer<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RingBuffer")
            .field("capacity", &self.capacity())
            .field("len", &self.len)
            .field("head", &self.head)
            .field("data", &self.iter().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_buffer_with_defaults() {
        let rb: RingBuffer<i32> = RingBuffer::new(4);
        assert_eq!(rb.capacity(), 4);
        assert_eq!(rb.len(), 0);
        assert!(rb.is_empty());
        assert!(!rb.is_full());
    }

    #[test]
    #[should_panic(expected = "capacity must be > 0")]
    fn new_panics_on_zero_capacity() {
        let _: RingBuffer<f64> = RingBuffer::new(0);
    }

    #[test]
    fn push_fills_buffer() {
        let mut rb = RingBuffer::new(3);
        assert_eq!(rb.push(10), None);
        assert_eq!(rb.len(), 1);
        assert!(!rb.is_full());

        assert_eq!(rb.push(20), None);
        assert_eq!(rb.len(), 2);

        assert_eq!(rb.push(30), None);
        assert_eq!(rb.len(), 3);
        assert!(rb.is_full());
    }

    #[test]
    fn push_evicts_oldest_when_full() {
        let mut rb = RingBuffer::new(3);
        rb.push(10);
        rb.push(20);
        rb.push(30);

        assert_eq!(rb.push(40), Some(10));
        assert_eq!(rb.len(), 3);
        assert!(rb.is_full());

        assert_eq!(rb.get(0), &20);
        assert_eq!(rb.get(1), &30);
        assert_eq!(rb.get(2), &40);
    }

    #[test]
    fn push_wraps_multiple_times() {
        let mut rb = RingBuffer::new(2);
        rb.push(1);
        rb.push(2);
        assert_eq!(rb.push(3), Some(1));
        assert_eq!(rb.push(4), Some(2));
        assert_eq!(rb.push(5), Some(3));
        assert_eq!(rb.push(6), Some(4));

        assert_eq!(rb.get(0), &5);
        assert_eq!(rb.get(1), &6);
    }

    #[test]
    fn get_returns_logical_order() {
        let mut rb = RingBuffer::new(4);
        rb.push(1);
        rb.push(2);
        rb.push(3);

        assert_eq!(rb.get(0), &1);
        assert_eq!(rb.get(1), &2);
        assert_eq!(rb.get(2), &3);
    }

    #[test]
    fn get_returns_logical_order_after_wrap() {
        let mut rb = RingBuffer::new(3);
        rb.push(10);
        rb.push(20);
        rb.push(30);
        rb.push(40);
        rb.push(50);

        assert_eq!(rb.get(0), &30);
        assert_eq!(rb.get(1), &40);
        assert_eq!(rb.get(2), &50);
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn get_panics_on_out_of_bounds() {
        let mut rb = RingBuffer::new(3);
        rb.push(1);
        rb.push(2);
        let _ = rb.get(2);
    }

    #[test]
    fn get_mut_modifies_value() {
        let mut rb = RingBuffer::new(3);
        rb.push(1);
        rb.push(2);
        rb.push(3);

        *rb.get_mut(1) = 99;
        assert_eq!(rb.get(1), &99);
    }

    #[test]
    fn get_mut_after_wrap() {
        let mut rb = RingBuffer::new(3);
        rb.push(10);
        rb.push(20);
        rb.push(30);
        rb.push(40);

        *rb.get_mut(0) = 100;
        assert_eq!(rb.get(0), &100);
        assert_eq!(rb.get(1), &30);
        assert_eq!(rb.get(2), &40);
    }

    #[test]
    fn clear_resets_state() {
        let mut rb = RingBuffer::new(3);
        rb.push(1);
        rb.push(2);
        rb.push(3);

        rb.clear();
        assert_eq!(rb.len(), 0);
        assert!(rb.is_empty());
        assert!(!rb.is_full());
        assert_eq!(rb.capacity(), 3);
    }

    #[test]
    fn clear_then_push_works() {
        let mut rb = RingBuffer::new(3);
        rb.push(1);
        rb.push(2);
        rb.push(3);
        rb.clear();

        rb.push(10);
        rb.push(20);
        assert_eq!(rb.len(), 2);
        assert_eq!(rb.get(0), &10);
        assert_eq!(rb.get(1), &20);
    }

    #[test]
    fn iter_empty_buffer() {
        let rb: RingBuffer<i32> = RingBuffer::new(3);
        let items: Vec<&i32> = rb.iter().collect();
        assert!(items.is_empty());
    }

    #[test]
    fn iter_partial_buffer() {
        let mut rb = RingBuffer::new(4);
        rb.push(1);
        rb.push(2);
        rb.push(3);

        let items: Vec<&i32> = rb.iter().collect();
        assert_eq!(items, vec![&1, &2, &3]);
    }

    #[test]
    fn iter_full_buffer_no_wrap() {
        let mut rb = RingBuffer::new(3);
        rb.push(10);
        rb.push(20);
        rb.push(30);

        let items: Vec<&i32> = rb.iter().collect();
        assert_eq!(items, vec![&10, &20, &30]);
    }

    #[test]
    fn iter_full_buffer_after_wrap() {
        let mut rb = RingBuffer::new(3);
        rb.push(10);
        rb.push(20);
        rb.push(30);
        rb.push(40);
        rb.push(50);

        let items: Vec<&i32> = rb.iter().collect();
        assert_eq!(items, vec![&30, &40, &50]);
    }

    #[test]
    fn sum_on_f64_buffer() {
        let mut rb = RingBuffer::new(3);
        rb.push(1.0);
        rb.push(2.0);
        rb.push(3.0);

        assert!((rb.sum() - 6.0).abs() < 1e-10);
    }

    #[test]
    fn sum_after_eviction() {
        let mut rb = RingBuffer::new(2);
        rb.push(10.0);
        rb.push(20.0);
        rb.push(30.0);

        assert!((rb.sum() - 50.0).abs() < 1e-10);
    }

    #[test]
    fn sum_empty_buffer() {
        let rb: RingBuffer<f64> = RingBuffer::new(3);
        assert!((rb.sum() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn clone_produces_equal_buffer() {
        let mut rb = RingBuffer::new(3);
        rb.push(1);
        rb.push(2);
        rb.push(3);
        rb.push(4);

        let cloned = rb.clone();
        assert_eq!(cloned.len(), rb.len());
        assert_eq!(cloned.capacity(), rb.capacity());
        assert_eq!(cloned.get(0), rb.get(0));
        assert_eq!(cloned.get(1), rb.get(1));
        assert_eq!(cloned.get(2), rb.get(2));
    }

    #[test]
    fn debug_output_includes_logical_data() {
        let mut rb = RingBuffer::new(3);
        rb.push(10);
        rb.push(20);
        rb.push(30);
        rb.push(40);

        let debug_str = format!("{:?}", rb);
        assert!(debug_str.contains("RingBuffer"));
        assert!(debug_str.contains("40"));
        assert!(debug_str.contains("20"));
        assert!(debug_str.contains("30"));
    }

    #[test]
    fn with_generic_tuple_type() {
        let mut rb = RingBuffer::new(2);
        rb.push((1.0, 2.0));
        rb.push((3.0, 4.0));

        assert_eq!(rb.get(0), &(1.0, 2.0));
        assert_eq!(rb.get(1), &(3.0, 4.0));

        let evicted = rb.push((5.0, 6.0));
        assert_eq!(evicted, Some((1.0, 2.0)));
    }

    #[test]
    fn single_capacity_buffer() {
        let mut rb = RingBuffer::new(1);
        assert_eq!(rb.push(42), None);
        assert!(rb.is_full());
        assert_eq!(rb.get(0), &42);

        assert_eq!(rb.push(99), Some(42));
        assert_eq!(rb.get(0), &99);
    }

    #[test]
    fn push_returns_none_until_full() {
        let mut rb = RingBuffer::new(5);
        for i in 0..4 {
            assert_eq!(rb.push(i), None);
        }
        assert_eq!(rb.push(4), None);
        assert_eq!(rb.push(5), Some(0));
    }

    #[test]
    fn iter_after_clear_and_refill() {
        let mut rb = RingBuffer::new(3);
        rb.push(1);
        rb.push(2);
        rb.push(3);
        rb.clear();
        rb.push(100);
        rb.push(200);

        let items: Vec<&i32> = rb.iter().collect();
        assert_eq!(items, vec![&100, &200]);
    }
}
