use crate::aliexpress_provider::store::{
    extract_store, get_feedback_page_url, get_store, get_store_by_owner_id, get_store_page,
};
use crate::aliexpress_provider::tests::read_data_file;
use crate::aliexpress_provider::virtual_user::VirtualUser;
use crate::scraper_extension::query_selector_all_html;
use scraper::Html;
use url::Url;

#[test]
fn test_get_feedback_page_url() {
    let store_id = 912322050;
    let mut user = VirtualUser::new_with_defaults();
    let actual = get_feedback_page_url(store_id, &mut user).unwrap();
    let expectation = Url::parse(
        "https://feedback.aliexpress.com//display/evaluationDetail.htm?ownerMemberId=252607694",
    )
    .unwrap();
    assert_eq!(actual, expectation);
}

#[test]
fn test_get_store_page() {
    let identity = 2855009;
    let mut user = VirtualUser::new_with_defaults();
    let page = get_store_page(identity, &mut user).unwrap();
    let html = Html::parse_document(&page);
    let nodes = query_selector_all_html(&html, "#container").unwrap();
    assert_eq!(nodes.len(), 1);
}

#[test]
fn test_get_store() {
    let identity = 912322050;
    let mut user = VirtualUser::new_with_defaults();
    let actual = get_store(identity, &mut user).unwrap();
    assert_eq!(actual.identity, identity);
    assert_eq!(actual.name, "Asgard Official Store".to_string());
}

#[test]
fn test_get_store_by_owner_id() {
    let owner_id = 2668104220;
    let mut user = VirtualUser::new_with_defaults();
    let actual = get_store_by_owner_id(owner_id, &mut user).unwrap();
    assert_eq!(actual.identity, 1102334365);
    assert_eq!(actual.name, "Akalin Store".to_string());
}

#[test]
fn test_extract_store() {
    let page = read_data_file("store_feedback_page.html");
    let owner_id = 123;
    let actual = match extract_store(&page, owner_id) {
        Err(e) => {
            if e.skip {
                return;
            }
            panic!("{}", e.to_string());
        }
        Ok(v) => v,
    };
    assert_eq!(actual.identity, 1100263087);
}
