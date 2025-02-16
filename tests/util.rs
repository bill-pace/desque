/// Check whether two floats have a relative difference of at most 5e-5 times the smaller value.
#[macro_export]
macro_rules! assert_floats_near_equal {
    ($val1:expr, $val2:expr, $msg:expr) => {{
        let a = $val1;
        let b = $val2;
        let diff = a - b;
        let relative_diff = if a.abs() < b.abs() { diff / a } else { diff / b };
        assert!(relative_diff < 0.00005, "{}", $msg);
    }};
}
