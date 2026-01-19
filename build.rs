fn main() {
    let now = time::OffsetDateTime::now_utc();
    let date_fmt = time::format_description::parse("[month repr:short] [day padding:space] [year]")
        .expect("valid date format");
    let time_fmt = time::format_description::parse("[hour]:[minute]:[second]")
        .expect("valid time format");

    let date = std::env::var("ALEMBIC_BUILD_DATE")
        .unwrap_or_else(|_| now.format(&date_fmt).unwrap_or_else(|_| "unknown".to_string()));
    let time = std::env::var("ALEMBIC_BUILD_TIME")
        .unwrap_or_else(|_| now.format(&time_fmt).unwrap_or_else(|_| "unknown".to_string()));

    println!("cargo:rustc-env=ALEMBIC_BUILD_DATE={}", date);
    println!("cargo:rustc-env=ALEMBIC_BUILD_TIME={}", time);
}
