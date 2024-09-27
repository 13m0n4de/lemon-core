#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn() -> Result<&'static str, &'static str>]) {
    println!("Running {} tests...", tests.len());
    let mut succeed = 0;
    for test in tests {
        match test() {
            Ok(msg) => {
                println!(" \x1b[1;32m[ok]: {}\x1b[0m", msg);
                succeed += 1;
            }
            Err(msg) => {
                println!(" \x1b[1;31m[err]: {}\x1b[0m", msg);
            }
        }
    }
    println!(
        "{} tests in total, {} succeed, {} failed",
        tests.len(),
        succeed,
        tests.len() - succeed
    );
    crate::sbi::shutdown(false);
}

/// Defines a test function that wraps a given block of code into a test case.
///
/// # Usages
///
/// `test!(function_name, { test_code })`
#[macro_export]
macro_rules! test {
    ($func_name: ident, $func: block) => {
        #[test_case]
        fn $func_name() -> Result<&'static str, &'static str> {
            print!("\x1b[4;37m{}\x1b[0m::{}", file!(), stringify!($func_name));
            $func
        }
    };
}

/// Asserts a condition within a test, returning an error message on failure.
///
/// # Usages
///
/// * `test_assert!(expression, "error message")`: Asserts with a custom message.
/// * `test_assert!(expression)`: Asserts with a default failure message.
#[macro_export]
#[allow(clippy::module_name_repetitions)]
macro_rules! test_assert {
    ($assert_expr: expr, $info: literal) => {
        if !$assert_expr {
            return Err(concat!($info, " at line ", line!()));
        }
    };
    ($assert_expr: expr) => {
        if !$assert_expr {
            return Err(concat!(
                "Assertion failed: ",
                stringify!($assert_expr),
                " at ",
                file!(),
                ":",
                line!()
            ));
        }
    };
}
