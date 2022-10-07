use crate::aliexpress_provider::category::{extract_category, extract_level_3_items};
use crate::aliexpress_provider::tests::read_data_file;
use scraper::{Html, Selector};

// #[test]
// fn test_get_page() {
//     let mut it = CategoryIterator::new();
//     let page = it.get_page().unwrap();
//     let doc = Html::parse_document(&page);
//     let selector = Selector::parse("#we-wholesale-category-list").unwrap();
//     let mut nodes = doc.select(&selector);
//     assert_eq!(nodes.count(), 1);
// }
//
// #[test]
// fn test_get_items() {
//     let page = read_data_file("category_list_page.html");
//     let actual = CategoryIterator::get_items(&page).unwrap();
//     assert!(actual.len() > 0);
// }

#[test]
fn test_extract_level_3_items() {
    let page = read_data_file("../../category/test/data/level_3_page.html");
    let items = extract_level_3_items(&page).unwrap();
    assert_eq!(items.len(), 15);
}

#[test]
fn test_extract_category() {
    let page = read_data_file("../../category/test/data/category_page.html");
    let actual = extract_category(&page).unwrap();
    assert_eq!(actual.identity, 42);
    assert_eq!(actual.name, "Hardware");
}
