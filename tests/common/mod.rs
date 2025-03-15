const TMPDIR: &str = "../playground";

pub fn redirect_temp_dir() {
    unsafe {
        std::env::set_var("TMPDIR", TMPDIR);
    }
}
