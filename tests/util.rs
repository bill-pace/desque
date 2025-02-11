/// Check whether two floats have a relative difference of at
/// most 5e-5 times the smaller value.
#[macro_export]
macro_rules! assert_floats_near_equal {
    ($val1:expr, $val2:expr, $msg:expr) => {{
        // open new scope to prevent unexpectedly shadowing variables at the call site
        let diff = $val1 - $val2;
        let relative_diff = if $val1.abs() < $val2.abs() {
            diff / $val1
        } else {
            diff / $val2
        };
        assert!(relative_diff < 0.00005, "{}", $msg);
    }};
}
