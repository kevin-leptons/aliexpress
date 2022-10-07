mod product_page_puller;
mod provider;
mod store;

use std::fs;
use std::path::Path;
use ureq::Error;

pub(crate) fn read_data_file(relative_path: &str) -> String {
    let file_path = Path::new("src")
        .join("aliexpress_provider")
        .join("tests")
        .join("data")
        .join(relative_path);
    return fs::read_to_string(file_path).unwrap();
}
