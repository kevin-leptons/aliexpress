use crate::aliexpress_provider::product_puller::ProductPagePuller;
use crate::aliexpress_provider::tests::read_data_file;
use serde_json::Value;

#[test]
fn test_extract_products() {
    let page = read_data_file("category_products.html");
    let products = ProductPagePuller::extract_products(&page).unwrap();
    assert!(products.len() > 0);
}
