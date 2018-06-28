use std::cmp::min;

static UNITS: [&str; 9] = ["B", "kB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
const DELIMITER: f64 = 1000_f64;

pub fn format_size(num: f64) -> String {
    let sign = if num.is_sign_positive() { "" } else { "-" };

    let num = num.abs();

    if num < 1_f64 {
        return format!("{}{} {}", sign, num, "B");
    }

    let exponent = min(
        (num.ln() / DELIMITER.ln()).floor() as i32,
        (UNITS.len() - 1) as i32,
    );

    let pretty_bytes = format!("{:.2}", num / DELIMITER.powi(exponent));

    format!(
        "{}{} {}",
        sign,
        pretty_bytes.trim_matches('0').trim_matches('.'),
        UNITS[exponent as usize]
    )
}

#[cfg(test)]
#[path = "./byte_format_tests.rs"]
mod byte_format_tests;
