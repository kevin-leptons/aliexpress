use crate::aliexpress_provider::tests::read_data_file;
use crate::aliexpress_provider::{
    get_data_script, get_product_data, get_standard_url, make_product, AliexpressProvider,
    Timestamp,
};
use crate::provider::{Product, ProviderIdentity};
use serde_json::{json, Value};
use url::Url;

#[test]
fn test_get_standard_url() {
    let link = String::from("https://vi.aliexpress.com/item/1005003678799430.html?x1&y=2");
    let actual = get_standard_url(link).unwrap();
    let expectation = Url::parse("https://aliexpress.com/item/1005003678799430.html").unwrap();
    assert_eq!(actual, expectation)
}

#[test]
fn test_get_data_script() {
    let page = read_data_file("product_page.html");
    let actual = get_data_script(String::from(page)).unwrap();
    assert!(actual.contains("window.runParams = "));
}

#[test]
fn test_get_product_data() {
    let script_data = read_data_file("product_data.js");
    let data = get_product_data(script_data).unwrap();
    let actual = &data["actionModule"]["productId"];
    match actual {
        Value::Number(v) => assert_eq!(v.as_f64(), Some(1005003678799430.0)),
        _ => panic!("bad data structure"),
    };
}

#[test]
fn test_get_product_page() {
    let mut provider = AliexpressProvider::new();
    let link = Url::parse("https://aliexpress.com/item/32915816912.html").unwrap();
    let actual = provider.get_product_page(&link).unwrap();
    assert!(actual.len() > 0);
}

#[test]
fn test_make_product() {
    let source = Url::parse("http://foo.bar").unwrap();
    let data = json!({
        "actionModule": {
            "productId": 1005003678799430 as u64
        },
        "titleModule": {
            "subject": "foo bar"
        }
    });
    let actual = make_product(&source, &data).unwrap();
    let expectation = Product {
        identity: 1005003678799430,
        owner_identity: 43434343,
        provider_identity: ProviderIdentity::Amazon,
        category_identity: 1212,
        name: "foo bar".to_string(),
        price: 1.2,
        cost: 1.2,
        image_url: Url::parse("http://foo.bar").unwrap(),
        orders: Some(3),
        rating: Some(4.5),
        shipping_fee: Some(6.7),
        store_identity: 23232,
        store_name: "foo bar".to_string(),
        revenue: Some(0.0),
    };
    assert_eq!(actual, expectation);
}
