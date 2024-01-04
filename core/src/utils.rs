

#[macro_export]
macro_rules! roundup2 {
    ($x:expr, $y:expr) => {
        (($x) + ($y - 1)) & (!(($y) - 1))
    };
}
