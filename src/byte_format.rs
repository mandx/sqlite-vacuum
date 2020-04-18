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
mod byte_format_tests {
    use super::format_size;

    #[test]
    fn it_converts_bytes_to_human_readable_strings() {
        assert_eq!(format_size(0_f64), "0 B");
        assert_eq!(format_size(0.4_f64), "0.4 B");
        assert_eq!(format_size(0.7_f64), "0.7 B");
        assert_eq!(format_size(10_f64), "10 B");
        assert_eq!(format_size(10.1_f64), "10.1 B");
        assert_eq!(format_size(999_f64), "999 B");
        assert_eq!(format_size(1001_f64), "1 kB");
        assert_eq!(format_size(1e16), "10 PB");
        assert_eq!(format_size(1e30), "1000000 YB");
    }

    #[test]
    fn it_supports_negative_numbers() {
        assert_eq!(format_size(-0.4_f64), "-0.4 B");
        assert_eq!(format_size(-0.7_f64), "-0.7 B");
        assert_eq!(format_size(-10.1_f64), "-10.1 B");
        assert_eq!(format_size(-999_f64), "-999 B");
        assert_eq!(format_size(-1001_f64), "-1 kB");
    }
}
