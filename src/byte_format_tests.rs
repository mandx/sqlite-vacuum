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
