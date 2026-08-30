// Temporary debug test
#[test]
fn debug_cvar2() {
    let confidence: f64 = 0.70;
    let sorted_len: f64 = 10.0;
    let v = (1.0 - confidence) * sorted_len;
    println!("(1.0 - 0.70) * 10.0 = {}", v);
    println!("(1.0 - 0.7) = {}", 1.0 - 0.7);
    let c = v.ceil() as usize;
    println!("ceil = {}, as usize = {}", v.ceil(), c);
}
