pub fn format_large_number(number: usize, singular_suffix: &str, plural_suffix: &str) -> String {
    if number >= 100_000_000 {
        format!("{:.0}M {plural_suffix}", number as f32 / 1_000_000.0)
    } else if number >= 1_000_000 {
        format!("{:.1}M {plural_suffix}", number as f32 / 1_000_000.0)
    } else if number >= 100_000 {
        format!("{:.0}k {plural_suffix}", number as f32 / 1_000.0)
    } else if number >= 1_000 {
        format!("{:.1}k {plural_suffix}", number as f32 / 1_000.0)
    } else if number == 1 {
        format!("1 {singular_suffix}")
    } else {
        format!("{} {plural_suffix}", number)
    }
}

pub fn format_large_number_detailed(
    number: usize,
    singular_suffix: &str,
    plural_suffix: &str,
) -> String {
    if number >= 100_000_000 {
        format!("{:.0}M {plural_suffix}", number as f32 / 1_000_000.0)
    } else if number >= 10_000_000 {
        format!("{:.1}M {plural_suffix}", number as f32 / 1_000_000.0)
    } else if number >= 1_000_000 {
        format!("{:.2}M {plural_suffix}", number as f32 / 1_000_000.0)
    } else if number >= 10_000 {
        format!("{:.1}k {plural_suffix}", number as f32 / 1_000.0)
    } else if number == 1 {
        format!("1 {singular_suffix}")
    } else {
        format!("{} {plural_suffix}", number)
    }
}

pub fn format_file_size(size: usize) -> String {
    if size >= 100_000_000_000 {
        format!("{:.0} GB", size as f32 / 1_000_000_000.0)
    } else if size >= 1_000_000_000 {
        format!("{:.1} GB", size as f32 / 1_000_000_000.0)
    } else if size >= 100_000_000 {
        format!("{:.0} MB", size as f32 / 1_000_000.0)
    } else if size >= 1_000_000 {
        format!("{:.1} MB", size as f32 / 1_000_000.0)
    } else if size >= 100_000 {
        format!("{:.0} kB", size as f32 / 1_000.0)
    } else if size >= 1_000 {
        format!("{:.1} kB", size as f32 / 1_000.0)
    } else {
        format!("{} bytes", size)
    }
}
