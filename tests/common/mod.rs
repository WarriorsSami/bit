const TMPDIR: &str = "../playground";

pub fn redirect_temp_dir() {
    std::env::set_var("TMPDIR", TMPDIR);
}
