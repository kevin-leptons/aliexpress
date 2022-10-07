mod category;
mod formatter;
mod product_puller;
mod store;
#[cfg(test)]
mod tests;
mod virtual_user;

use crate::aliexpress_provider::product_puller::ProductPagePuller;
use crate::aliexpress_provider::store::{get_store, get_store_by_owner_id};
use crate::aliexpress_provider::virtual_user::VirtualUser;
use crate::provider::{
    Category, CategoryIteratorResult, CompactProduct, CompactProductIteratorResult, Product,
    Provider, ProviderIdentity, Store, Timestamp,
};
use crate::result::{BoxResult, PullError, PullErrorKind, PullResult, UnexpectedError};
use crate::scraper_extension::{
    get_inner_text_element, query_selector_all_element, query_selector_all_html,
    query_selector_element,
};
use chrono::Duration;
use core::cmp::PartialEq;
use js_sandbox::Script;
use mongodb::options::ReturnDocument;
use rand;
use rand::Rng;
use scraper::node::Element;
use scraper::{ElementRef, Html, Node, Selector};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Number, Value};
use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::ops::Sub;
use std::process::id;
use std::{panic::UnwindSafe, ptr::null, string};
use ureq::{Agent, AgentBuilder, Response};
use url::quirks::domain_to_unicode;
use url::{ParseError, Url};

const ROOT_URL: &str = "https://aliexpress.com";

pub struct AliexpressProvider {
    user: VirtualUser,
    store_user: VirtualUser,
}

fn js_str_to_u64(value: &Value) -> BoxResult<u64> {
    let s = match value {
        Value::String(v) => v,
        _ => return UnexpectedError::new_as_box_result("bad data"),
    };
    let n = match s.parse::<u64>() {
        Err(e) => return Err(Box::new(e)),
        Ok(v) => v,
    };
    return Ok(n);
}

fn js_value_to_f64(value: &Value) -> BoxResult<f64> {
    match value.as_f64() {
        Some(v) => return Ok(v),
        None => {}
    };
    match value.as_u64() {
        Some(v) => return Ok(v as f64),
        None => {}
    }
    return UnexpectedError::new_as_box_result("bad float data");
}

impl Provider for AliexpressProvider {
    fn get_category(&mut self, identity: u64) -> PullResult<Category> {
        static STEP: &str = "get_category";
        let item = match category::get_category(identity, &mut self.user) {
            Err(e) => return e.stack_step(STEP).to_result(),
            Ok(v) => v,
        };
        return Ok(item);
    }

    fn get_product(&mut self, link: String) -> BoxResult<Product> {
        let url = match get_standard_url(link) {
            Err(e) => return Err(Box::new(e)),
            Ok(v) => v,
        };
        let page = match self.get_product_page(&url) {
            Err(e) => return Err(e),
            Ok(v) => v,
        };
        let script = match get_data_script(page) {
            Err(e) => return Err(e),
            Ok(v) => v,
        };
        let data = match get_product_data(script) {
            Err(e) => return Err(e),
            Ok(v) => v,
        };
        return make_product(&url, &data);
    }

    fn get_level_1_2_categories(&mut self) -> PullResult<Vec<Category>> {
        return category::pull_level_1_2_categories(&mut self.user);
    }

    fn get_level_3_categories(&mut self, level_2_identity: u64) -> PullResult<Vec<Category>> {
        return category::pull_level_3_categories(level_2_identity, &mut self.user);
    }

    fn get_store(&mut self, identity: u64) -> BoxResult<Store> {
        return get_store(identity, &mut self.user);
    }

    fn get_store_by_owner_id(&mut self, owner_id: u64) -> PullResult<Store> {
        static STEP: &str = "aliexpress.get_store_by_owner_id";
        return match get_store_by_owner_id(owner_id, &mut self.store_user) {
            Err(e) => e.stack_step(STEP).to_result(),
            Ok(v) => Ok(v),
        };
    }

    fn get_products(
        &mut self,
        category_identity: u64,
        page_index: u32,
    ) -> PullResult<Vec<Product>> {
        return ProductPagePuller::pull(category_identity, page_index, &mut self.user);
    }
}

impl AliexpressProvider {
    pub fn new() -> AliexpressProvider {
        let user = VirtualUser::new(Duration::seconds(2), Duration::seconds(5)).unwrap();
        let store_user =
            VirtualUser::new(Duration::milliseconds(500), Duration::milliseconds(501)).unwrap();
        return AliexpressProvider { user, store_user };
    }

    fn get_product_page(&mut self, source: &Url) -> BoxResult<String> {
        let res = self.user.get(source)?;
        return Ok(res.body);
    }
}

fn get_standard_url(link: String) -> Result<Url, ParseError> {
    let source = match Url::parse(&link) {
        Err(e) => return Err(e),
        Ok(v) => v,
    };
    let mut target = Url::parse("https://aliexpress.com").unwrap();
    target.set_path(source.path());
    return Ok(target);
}

fn get_product_data(data_script: String) -> BoxResult<Value> {
    let js_code_base = String::from("window = {};\n");
    let js_inspect_code = "function getProduct() {return window.runParams};\n";
    let js_code = js_code_base + &data_script + ";\n" + js_inspect_code;
    let mut script = match Script::from_string(&js_code.to_string()) {
        Err(e) => {
            let e2 = UnexpectedError::new(&e.to_string());
            return Err(Box::new(e2));
        }
        Ok(v) => v,
    };
    let arg = 0;
    let data: Value = match script.call("getProduct", &arg) {
        Err(e) => {
            let e2 = UnexpectedError::new(&e.to_string());
            return Err(Box::new(e2));
        }
        Ok(v) => v,
    };
    let product_data = data["data"].clone();
    return Ok(product_data);
}

fn make_product(source: &Url, data: &Value) -> BoxResult<Product> {
    let identity = match js_value_to_u64(&data["actionModule"]["productId"]) {
        Ok(v) => v,
        Err(e) => return Err(e),
    };
    let name = match data["titleModule"]["subject"].as_str() {
        Some(v) => v,
        None => return UnexpectedError::new_as_box_result("bad product name"),
    };
    let p = Product {
        provider_identity: ProviderIdentity::Aliexpress,
        identity: identity as u64,
        name: name.to_string(),
        price: 0.0,
        cost: 0.0,
        store_identity: 0,
        store_name: "".to_string(),
        image_url: Url::parse("http://foo.bar").unwrap(),
        owner_identity: 0,
        category_identity: 0,

        orders: Option::Some(0),
        rating: Option::Some(0.0),
        shipping_fee: Option::Some(0.0),
        revenue: None,
    };
    return Ok(p);
}

fn js_value_to_u64(value: &Value) -> BoxResult<u64> {
    match value.as_f64() {
        Some(v) => return Ok(v as u64),
        None => {}
    };
    match value.as_u64() {
        Some(v) => return Ok(v),
        None => {}
    };
    return UnexpectedError::new_as_box_result("bad product identity");
}

fn get_data_script(page: String) -> BoxResult<String> {
    let doc = Html::parse_document(&page);
    let selector = Selector::parse("script").unwrap();
    let node = match doc.select(&selector).nth(17) {
        None => {
            let e = UnexpectedError::new("no data script tag");
            return Err(Box::new(e));
        }
        Some(v) => v,
    };
    let script = node.inner_html();
    return Ok(script);
}

fn extract_product_price(product: &Value) -> BoxResult<f64> {
    let price_value = &product["prices"]["salePrice"]["minPrice"];
    if *price_value == Value::Null {
        return Ok(0.0);
    }
    return js_value_to_f64(price_value);
}
