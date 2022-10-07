use crate::aliexpress_provider::formatter::str_to_f64;
use crate::aliexpress_provider::store::extract_store_owner_identity;
use crate::aliexpress_provider::virtual_user::VirtualUser;
use crate::aliexpress_provider::{
    extract_product_price, js_str_to_u64, js_value_to_f64, js_value_to_u64, ROOT_URL,
};
use crate::javascript::get_rhs_object;
use crate::provider::{Product, ProviderIdentity};
use crate::result::{BoxResult, PullError, PullErrorKind, PullResult, UnexpectedError};
use crate::scraper_extension::query_selector_html;
use js_sandbox::Script;
use scraper::{Html, Selector};
use serde_json::Value;
use url::Url;

pub struct ProductPagePuller;

impl ProductPagePuller {
    // https://www.aliexpress.com/category/100003109/women-clothing.html?page=2
    pub fn pull(
        category_identity: u64,
        page_index: u32,
        user: &mut VirtualUser,
    ) -> PullResult<Vec<Product>> {
        static STEP: &str = "product_page_puller.pull";
        let url = Self::get_page_url(category_identity, page_index);
        let res = match user.get(&url) {
            Err(e) => return e.stack_step(STEP).to_result(),
            Ok(v) => v,
        };
        let mut products = match Self::extract_products(&res.body) {
            Err(e) => {
                return e
                    .stack_step(STEP)
                    .set_http_context(&url, res.status, &res.body)
                    .to_result()
            }
            Ok(v) => v,
        };
        for product in &mut products {
            product.category_identity = category_identity;
        }
        return Ok(products);
    }

    fn get_page_url(category_id: u64, page_index: u32) -> Url {
        let mut url = Url::parse(ROOT_URL).unwrap();
        let path = "/category/".to_string() + &category_id.to_string() + "/meaningless.html";
        url.set_path(&path);
        url.query_pairs_mut()
            .append_pair("page", &page_index.to_string())
            .append_pair("trafficChannel", "main");
            // .append_pair("ltype", "wholesale");
        // .append_pair("SortType", "default")
        // .append_pair("g", "n");
        // .append_pair("isrefine", "y");
        return url;
    }

    pub(super) fn extract_products(page: &String) -> PullResult<Vec<Product>> {
        static STEP: &str = "extract_products";
        let data = Self::get_page_data(page)?;
        if Self::is_no_products(&data) {
            return PullError::from_step(STEP, PullErrorKind::NoData)
                .set_message("no products in the category")
                .set_skip(true)
                .to_result();
        }
        return Self::extract_products_from_data(data);
    }

    fn get_page_data(page: &String) -> PullResult<Value> {
        let script = Self::get_page_data_script(page)?;
        return Self::get_page_run_params(&script);
    }

    fn get_page_data_script(page: &String) -> PullResult<String> {
        static STEP: &str = "get_page_data_script";
        let doc = Html::parse_document(&page);
        let selector = Selector::parse("script").unwrap();
        let matched_node = doc.select(&selector).find(|node| {
            let text = node.inner_html();
            return text.contains("window._dida_config_._init_data_= {");
        });
        let node = match matched_node {
            None => return PullError::from_step(STEP, PullErrorKind::NoData).to_result(),
            Some(v) => v,
        };
        let script = node.inner_html();
        return Ok(script);
    }

    fn is_no_products(data: &Value) -> bool {
        let result_type = match data["resultType"].as_str() {
            None => return false,
            Some(v) => v,
        };
        return result_type == "zero_result";
    }

    fn get_page_run_params(script: &String) -> PullResult<Value> {
        static STEP: &str = "get_page_run_params";
        let js_code_base = String::from("window = {};\n");
        let js_inspect_code = "function getProduct() {return window._dida_config_._init_data_};\n";
        let js_code = js_code_base + &script + ";\n" + js_inspect_code;
        let mut script = match Script::from_string(&js_code.to_string()) {
            Err(e) => {
                return PullError::from_step(STEP, PullErrorKind::BadData)
                    .set_message(e.to_string().as_str())
                    .to_result()
            }
            Ok(v) => v,
        };
        let arg = 0;
        let data: Value = match script.call("getProduct", &arg) {
            Err(e) => {
                return PullError::from_step(STEP, PullErrorKind::BadData)
                    .set_message(e.to_string().as_str())
                    .to_result()
            }
            Ok(v) => v,
        };
        return Ok(data.clone());
    }

    fn extract_products_from_data(data: Value) -> PullResult<Vec<Product>> {
        static STEP: &str = "extract_products_from_data";
        let raw_products =
            match &data["data"]["data"]["root"]["fields"]["mods"]["itemList"]["content"] {
                Value::Array(v) => v,
                _ => return PullError::from_step(STEP, PullErrorKind::NoData).to_result(),
            };
        let mut products = Vec::new();
        for raw_product in raw_products {
            let product_option = match ProductPagePuller::extract_product_from_data(raw_product) {
                Err(e) => return e.stack_step(STEP).to_result(),
                Ok(v) => v,
            };
            match product_option {
                None => continue,
                Some(v) => products.push(v),
            }
        }
        return Ok(products);
    }

    fn extract_product_from_data(data: &Value) -> PullResult<Option<Product>> {
        static STEP: &str = "extract_product_from_data";
        let identity = match js_str_to_u64(&data["productId"]) {
            Err(_) => {
                return PullError::from_step(STEP, PullErrorKind::BadData)
                    .set_message("identity")
                    .to_result()
            }
            Ok(v) => v,
        };
        let name = String::from("UNABLE_TO_GET");
        // let name = match &data["title"]["displayTitle"] {
        //     Value::String(v) => v.clone(),
        //     _ => {
        //         return PullError::from_step(STEP, PullErrorKind::BadData)
        //             .set_message("name")
        //             .to_result()
        //     }
        // };
        let price = match extract_product_price(&data) {
            Err(_) => {
                return PullError::from_step(STEP, PullErrorKind::BadData)
                    .set_message("price")
                    .to_result()
            }
            Ok(v) => v,
        };
        let orders = match ProductPagePuller::extract_product_orders(data) {
            Err(_) => {
                return PullError::from_step(STEP, PullErrorKind::BadData)
                    .set_message("order")
                    .to_result()
            }
            Ok(v) => v,
        };
        let shipping_fee = match Self::extract_shipping_fee(&data) {
            Err(_) => {
                return PullError::from_step(STEP, PullErrorKind::BadData)
                    .set_message("shipping_fee")
                    .to_result()
            }
            Ok(v) => v,
        };
        let image_url = match Self::extract_image_url(&data) {
            Err(_) => {
                return PullError::from_step(STEP, PullErrorKind::BadData)
                    .set_message("image_url")
                    .to_result()
            }
            Ok(v) => v,
        };
        let store_name = match &data["store"]["storeName"] {
            Value::String(v) => v.to_string(),
            Value::Null => return Ok(None),
            _ => {
                return PullError::from_step(STEP, PullErrorKind::BadData)
                    .set_message("store_name")
                    .to_result()
            }
        };
        let store_identity = match js_value_to_u64(&data["store"]["storeId"]) {
            Err(_) => {
                return PullError::from_step(STEP, PullErrorKind::BadData)
                    .set_message("store_identity")
                    .to_result()
            }
            Ok(v) => v,
        };
        let owner_identity = match js_value_to_u64(&data["store"]["aliMemberId"]) {
            Err(_) => {
                return PullError::from_step(STEP, PullErrorKind::BadData)
                    .set_message("owner_identity")
                    .to_result();
            }
            Ok(v) => v,
        };
        let rating = match Self::extract_product_rating(&data) {
            Err(_) => {
                return PullError::from_step(STEP, PullErrorKind::BadData)
                    .set_message("rating")
                    .to_result()
            }
            Ok(v) => v,
        };
        let product = Product {
            provider_identity: ProviderIdentity::Aliexpress,
            identity: identity,
            name: name,
            price: price,
            cost: get_cost(price, shipping_fee),
            image_url: image_url,
            store_name: store_name,
            store_identity: store_identity,
            owner_identity: owner_identity,
            category_identity: 0,

            orders: orders,
            shipping_fee: shipping_fee,
            rating: rating,
            revenue: get_revenue(price, orders),
        };
        return Ok(Some(product));
    }

    fn extract_product_orders(raw_product: &Value) -> BoxResult<Option<u64>> {
        let trade_str = match raw_product["trade"]["tradeDesc"].as_str() {
            Some(v) => v,
            None => return Ok(Option::None),
        };
        let mut parts = trade_str.split_whitespace();
        let orders_str = match parts.nth(0) {
            Some(v) => v,
            None => return UnexpectedError::new_as_box_result("bad orders data"),
        };
        let orders = match orders_str.parse::<u64>() {
            Err(e) => return Err(Box::new(e)),
            Ok(v) => v,
        };
        return Ok(Option::Some(orders));
    }

    fn extract_image_url(raw_product: &Value) -> BoxResult<Url> {
        let s = match &raw_product["image"]["imgUrl"] {
            Value::String(v) => v,
            _ => return UnexpectedError::new_as_box_result("bad image url"),
        };
        let url_str = "http:".to_owned() + s;
        let u = match Url::parse(&url_str) {
            Err(e) => return Err(Box::new(e)),
            Ok(v) => v,
        };
        return Ok(u);
    }

    fn extract_shipping_fee(raw_product: &Value) -> BoxResult<Option<f64>> {
        let selling_points = match &raw_product["sellingPoints"] {
            Value::Array(v) => v,
            _ => return Ok(Option::None),
        };
        let shipping_fee_point = match Self::extract_shipping_fee_point(selling_points)? {
            None => return Ok(Option::None),
            Some(v) => v,
        };
        let fee = Self::extract_shipping_fee_from_point(&shipping_fee_point)?;
        return Ok(Some(fee));
    }

    fn extract_product_rating(raw_product: &Value) -> BoxResult<Option<f64>> {
        let rating_value = &raw_product["evaluation"]["starRating"];
        match rating_value {
            Value::Null => return Ok(Option::None),
            _ => {}
        };
        return match js_value_to_f64(&rating_value) {
            Err(e) => Err(e),
            Ok(v) => Ok(Option::Some(v)),
        };
    }

    fn extract_shipping_fee_point(selling_points: &Vec<Value>) -> BoxResult<Option<Value>> {
        let mut matched_points = Vec::new();
        for point in selling_points {
            let tag_id = match point["sellingPointTagId"].as_str() {
                None => continue,
                Some(v) => v,
            };
            if tag_id != "885603359" {
                continue;
            }
            matched_points.push(point);
            if matched_points.len() > 1 {
                return UnexpectedError::new_as_box_result("too many matched shipping fee");
            }
        }
        if matched_points.len() == 0 {
            return Ok(None);
        }
        return Ok(Some(matched_points[0].clone()));
    }

    fn extract_shipping_fee_from_point(point: &Value) -> BoxResult<f64> {
        let tag = match &point["tagContent"]["tagText"] {
            Value::String(v) => v.clone(),
            _ => return UnexpectedError::new_as_box_result("no shipping fee tag"),
        };
        if tag == "Free Shipping" {
            return Ok(0.0);
        }
        if tag.contains("+Shipping: US $") == false {
            return UnexpectedError::new_as_box_result("bad shipping fee tag");
        }
        let parts = tag.split("$");
        let fee_str = match parts.last() {
            Some(v) => v,
            None => return UnexpectedError::new_as_box_result("bad shipping fee tag"),
        };
        return str_to_f64(fee_str);
    }
}

fn get_revenue(price: f64, orders: Option<u64>) -> Option<f64> {
    return match orders {
        None => None,
        Some(v) => Some(price * (v as f64)),
    };
}

fn get_cost(price: f64, shipping_fee: Option<f64>) -> f64 {
    return match shipping_fee {
        None => price,
        Some(v) => price + v,
    };
}
