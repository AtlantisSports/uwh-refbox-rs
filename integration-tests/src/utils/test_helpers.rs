//! Common test helpers.

#[allow(dead_code)]
pub fn init_logger_once() {
    let _ = env_logger::builder().is_test(true).try_init();
}

