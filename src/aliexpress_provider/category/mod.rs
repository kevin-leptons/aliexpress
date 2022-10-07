#[cfg(test)]
mod test;

use crate::aliexpress_provider::js_value_to_u64;
use crate::aliexpress_provider::virtual_user::VirtualUser;
use crate::provider::{Category, CategoryIteratorResult};
use crate::result::{BoxResult, PullError, PullErrorKind, PullResult, UnexpectedError};
use crate::scraper_extension::{
    get_inner_text_element, query_selector_all_element, query_selector_all_html,
    query_selector_element,
};
use fs_extra::dir::DirEntryAttr::Name;
use js_sandbox::Script;
use num_format::Locale::id;
use scraper::{ElementRef, Html, Selector};
use serde_json::Value;
use serde_json::Value::Null;
use std::collections::VecDeque;
use url::Url;

// https://www.aliexpress.com/category/42/hardware.html?spm=a2g0o.category_nav.1.51.b6a648b6yDXn1e
pub fn get_category(identity: u64, user: &mut VirtualUser) -> PullResult<Category> {
    static STEP: &str = "get_category";
    let url_str = format!(
        "https://www.aliexpress.com/category/{}/hardware.html",
        identity
    );
    let url = Url::parse(&url_str).unwrap();
    let res = match user.get(&url) {
        Err(e) => return e.stack_step(STEP).to_result(),
        Ok(v) => v,
    };
    let category = match extract_category(&res.body) {
        Err(e) => {
            return e
                .stack_step(STEP)
                .set_http_context(&url, res.status, &res.body)
                .to_result()
        }
        Ok(v) => v,
    };
    return Ok(category);
}

pub fn extract_category(page: &String) -> PullResult<Category> {
    let script = extract_page_script(page)?;
    let data = extract_page_data(&script)?;
    return extract_selected_category_from_data(&data);
}

fn extract_selected_category_from_data(data: &Value) -> PullResult<Category> {
    static STEP: &str = "extract_selected_category_from_data";
    if is_empty_data(data) == true {
        return PullError::from_step(STEP, PullErrorKind::NoData)
            .set_message("category page has no data")
            .set_skip(true)
            .to_result();
    }
    let filters = match data["data"]["data"]["root"]["fields"]["mods"]["searchRefineFilters"]["content"].as_array() {
        None => return PullError::from_step(STEP, PullErrorKind::BadData).to_result(),
        Some(v) => v,
    };
    let filter = &filters[1];
    let filter_type = match filter["type"].as_str() {
        None => {
            return PullError::from_step(STEP, PullErrorKind::BadData)
                .set_message("no refine type")
                .to_result()
        },
        Some(v) => String::from(v)
    };
    if filter_type != "category" {
        return PullError::from_step(STEP, PullErrorKind::BadData)
            .set_message("expect refine type: category")
            .to_result();
    }
    let content = match filter["content"][0].as_object() {
        None=> {
            return PullError::from_step(STEP, PullErrorKind::BadData)
                .set_message("no refine content")
                .to_result()
        },
        Some(v)=> v
    };
    let name = match content["categoryEnName"].as_str() {
        None => {
            return PullError::from_step(STEP, PullErrorKind::BadData)
                .set_message("bad name")
                .to_result()
        }
        Some(v) => String::from(v),
    };
    let identity = match js_value_to_u64(&content["categoryId"]) {
        Err(e) => {
            return PullError::from_step(STEP, PullErrorKind::BadData)
                .set_message("bad identity")
                .to_result()
        }
        Ok(v) => v,
    };
    let category = Category {
        identity: identity,
        name: name,
        name_url: "_anything_is_ok_".to_string(),
        level: 2,
        parent_identity: None,
    };
    return Ok(category);
}

pub fn pull_level_1_2_categories(user: &mut VirtualUser) -> PullResult<Vec<Category>> {
    static STEP: &str = "pull_level_1_2_categories";
    let url = Url::parse("https://www.aliexpress.com/all-wholesale-products.html").unwrap();
    let res = match user.get(&url) {
        Err(e) => return e.stack_step(STEP).to_result(),
        Ok(v) => v,
    };
    let items = match extract_level_1_2_items(&res.body) {
        Err(e) => {
            return e
                .stack_step(STEP)
                .set_http_context(&url, res.status, &res.body)
                .to_result()
        }
        Ok(v) => v,
    };
    return Ok(items);
}

pub fn pull_level_3_categories(
    level_2_identity: u64,
    user: &mut VirtualUser,
) -> PullResult<Vec<Category>> {
    static STEP: &str = "pull_level_3_categories";
    let url_str = format!(
        "https://www.aliexpress.com/category/{}/something.html",
        level_2_identity.to_string()
    );
    let url = Url::parse(&url_str).unwrap();
    let res = match user.get(&url) {
        Err(e) => return e.stack_step(STEP).to_result(),
        Ok(v) => v,
    };
    let mut items = match extract_level_3_items(&res.body) {
        Err(e) => {
            return e
                .stack_step(STEP)
                .set_http_context(&url, res.status, &res.body)
                .to_result()
        }
        Ok(v) => v,
    };
    for mut item in &mut items {
        item.parent_identity = Some(level_2_identity);
    }
    return Ok(items);
}

fn extract_level_3_items(page: &String) -> PullResult<Vec<Category>> {
    let script = extract_page_script(page)?;
    let data = extract_page_data(&script)?;
    return extract_level_3_items_from_data(&data);
}

fn extract_page_script(page: &String) -> PullResult<String> {
    static STEP: &str = "extract_page_script";
    let html = Html::parse_document(&page);
    let selector = Selector::parse("script").unwrap();
    let matched_node = html.select(&selector).find(|node| {
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

fn extract_page_data(script: &String) -> PullResult<Value> {
    static STEP: &str = "extract_page_data";
    let js_code_base = String::from("window = {};\n");
    let js_inspect_code = "function _getRunParams() {return window._dida_config_._init_data_};\n";
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
    let data: Value = match script.call("_getRunParams", &arg) {
        Err(e) => {
            return PullError::from_step(STEP, PullErrorKind::BadData)
                .set_message(e.to_string().as_str())
                .to_result()
        }
        Ok(v) => v,
    };
    return Ok(data.clone());
}

fn extract_level_3_items_from_data(data: &Value) -> PullResult<Vec<Category>> {
    static STEP: &str = "extract_level_3_items_from_data";
    if is_empty_data(data) == true {
        return Ok(Vec::new());
    }
    let items_data = match extract_level_3_items_data(data) {
        Err(e) => return e.stack_step(STEP).to_result(),
        Ok(v) => v,
    };
    let mut items = Vec::new();
    for item_data in items_data {
        let item = match extract_level_3_item_from_data(&item_data) {
            Err(e) => return e.stack_step(STEP).to_result(),
            Ok(v) => v,
        };
        items.push(item);
    }
    return Ok(items);
}

fn extract_level_3_items_data(data: &Value) -> PullResult<Vec<Value>> {
    static STEP: &str = "extract_level_3_items_data";
    let refine_category = match data["data"]["data"]["root"]["fields"]["mods"]["searchRefineFilters"]["content"].as_array() {
        None => {
            return PullError::from_step(STEP, PullErrorKind::NoData)
                .set_message("no attribute '...refineCategory'")
                .to_result()
        }
        Some(v) => v,
    };
    if refine_category.len() == 0 {
        return PullError::from_step(STEP, PullErrorKind::NoData)
            .set_message("empty 'refineCategory'")
            .to_result();
    }
    if refine_category.len() >= 2 {
        return PullError::from_step(STEP, PullErrorKind::NoData)
            .set_message("too many 'refineCategory'")
            .to_result();
    }
    let items_data = match &data["refineCategory"][0]["childCategories"] {
        Value::Null => Vec::new(),
        Value::Array(v) => v.clone(),
        _ => return PullError::from_step(STEP, PullErrorKind::BadData).to_result(),
    };
    return Ok(items_data);
}

fn extract_level_3_item_from_data(data: &Value) -> PullResult<Category> {
    static STEP: &str = "extract_level_3_item_from_data";
    let identity = match js_value_to_u64(&data["categoryId"]) {
        Err(_) => {
            return PullError::from_step(STEP, PullErrorKind::BadData)
                .set_message("bad identity")
                .to_result()
        }
        Ok(v) => v,
    };
    let name = match data["categoryName"].as_str() {
        None => {
            return PullError::from_step(STEP, PullErrorKind::BadData)
                .set_message("bad name")
                .to_result()
        }
        Some(v) => v.to_string(),
    };
    let name_url = match extract_level_3_item_name_url(data) {
        Err(e) => return e.stack_step(STEP).to_result(),
        Ok(v) => v,
    };
    let item = Category {
        identity: identity,
        level: 3,
        name: name,
        name_url: name_url,
        parent_identity: None,
    };
    return Ok(item);
}

fn extract_level_3_item_name_url(data: &Value) -> PullResult<String> {
    static STEP: &str = "extract_level_3_item_name_url";
    let url_str = match data["categoryUrl"].as_str() {
        None => return PullError::from_step(STEP, PullErrorKind::NoData).to_result(),
        Some(v) => v,
    };
    if url_str.len() < 2 {
        return PullError::from_step(STEP, PullErrorKind::BadData).to_result();
    }
    let valid_url_str = "http:".to_string() + url_str;
    let name_url = match extract_category_identity_and_name_url(valid_url_str.as_str()) {
        Err(e) => return e.stack_step(STEP).to_result(),
        Ok((_, v)) => v,
    };
    return Ok(name_url);
}

fn extract_level_1_2_items(page: &String) -> PullResult<Vec<Category>> {
    static STEP: &str = "extract_level_1_2_items";
    let mut items = Vec::new();
    let html = Html::parse_document(&page);
    let level_1_nodes = query_selector_all_html(&html, "#category div.cg-main div.item").unwrap();
    for node in level_1_nodes {
        let more_items = match extract_level_1_2_items_from_node(&node) {
            Err(e) => return e.stack_step(STEP).to_result(),
            Ok(v) => v,
        };
        items.extend(more_items);
    }
    let filtered_items = filter_items(items);
    return Ok(filtered_items);
}

fn extract_level_1_2_items_from_node(node: &ElementRef) -> PullResult<Vec<Category>> {
    static STEP: &str = "extract_level_1_2_items_from_node";
    let parent_item = match extract_level_1_item(node) {
        Err(e) => return Err(e),
        Ok(v) => v,
    };
    let mut child_items = match extract_level_2_items(node, parent_item.identity) {
        Err(e) => return e.stack_step(STEP).to_result(),
        Ok(v) => v,
    };
    let mut items = Vec::new();
    items.push(parent_item);
    items.append(&mut child_items);
    return Ok(items);
}

fn filter_items(items: Vec<Category>) -> Vec<Category> {
    let filtered_it = items.into_iter().filter(|item| {
        if is_ignored_category(item.identity) {
            return false;
        }
        match item.parent_identity {
            None => return true,
            Some(v) => (is_ignored_category(v) == false),
        }
    });
    return filtered_it.collect();
}

fn extract_level_1_item(node: &ElementRef) -> PullResult<Category> {
    static STEP: &str = "extract_level_1_item";
    let link = match query_selector_element(node, "h3 a") {
        None => {
            return PullError::from_step(STEP, PullErrorKind::NoData)
                .set_message("no category link")
                .to_result()
        }
        Some(v) => v,
    };
    let url = match link.value().attr("href") {
        None => {
            return PullError::from_step(STEP, PullErrorKind::NoData)
                .set_message("no category url")
                .to_result()
        }
        Some(v) => "https:".to_string() + v,
    };
    let (identity, name_url) = match extract_category_identity_and_name_url(url.as_str()) {
        Err(e) => return e.stack_step(STEP).to_result(),
        Ok(v) => v,
    };
    let item = Category {
        identity: identity,
        name: get_inner_text_element(&link),
        name_url: name_url,
        parent_identity: None,
        level: 1,
    };
    return Ok(item);
}

fn extract_level_2_items(
    parent_node: &ElementRef,
    parent_identity: u64,
) -> PullResult<Vec<Category>> {
    static STEP: &str = "extract_level_2_items";
    let mut items = Vec::new();
    let child_nodes = query_selector_all_element(parent_node, "ul a").unwrap();
    for node in child_nodes {
        let item = match extract_level_2_item(&node, parent_identity) {
            Err(e) => return e.stack_step(STEP).to_result(),
            Ok(v) => v,
        };
        items.push(item);
    }
    return Ok(items);
}

fn extract_level_2_item(node: &ElementRef, level_1_identity: u64) -> PullResult<Category> {
    static STEP: &str = "extract_level_2_item";
    let url_str = match node.value().attr("href") {
        None => {
            return PullError::from_step(STEP, PullErrorKind::NoData)
                .set_message("no category url")
                .to_result()
        }
        Some(v) => "http:".to_owned() + v,
    };
    let (identity, name_url) = match extract_category_identity_and_name_url(url_str.as_str()) {
        Err(e) => return e.stack_step(STEP).to_result(),
        Ok(v) => v,
    };
    let item = Category {
        identity: identity,
        name: get_inner_text_element(node),
        name_url: name_url,
        parent_identity: Some(level_1_identity),
        level: 2,
    };
    return Ok(item);
}

fn extract_category_identity_and_name_url(url_str: &str) -> PullResult<(u64, String)> {
    static STEP: &str = "extract_category_identity_and_name_url";
    let url = match Url::parse(url_str) {
        Err(e) => {
            return PullError::from_step(STEP, PullErrorKind::BadData)
                .set_message(e.to_string().as_str())
                .to_result()
        }
        Ok(v) => v,
    };
    let mut url_segments = match url.path_segments() {
        None => {
            return PullError::from_step(STEP, PullErrorKind::BadData)
                .set_message("bad category url")
                .to_result()
        }
        Some(v) => v,
    };
    let segments = url_segments.collect::<Vec<&str>>();
    if segments.len() != 3 {
        return PullError::from_step(STEP, PullErrorKind::BadData)
            .set_message("bad category url")
            .to_result();
    }
    if segments[0] != "category" {
        return PullError::from_step(STEP, PullErrorKind::BadData)
            .set_message("bad category url")
            .to_result();
    }
    let identity = match segments[1].parse::<u64>() {
        Err(e) => {
            return PullError::from_step(STEP, PullErrorKind::BadData)
                .set_message(e.to_string().as_str())
                .to_result()
        }
        Ok(v) => v,
    };
    let name_url_osstr = match std::path::Path::new(segments[2]).file_stem() {
        None => {
            return PullError::from_step(STEP, PullErrorKind::BadData)
                .set_message("bad category name")
                .to_result()
        }
        Some(v) => v,
    };
    let name_url = match name_url_osstr.to_str() {
        None => {
            return PullError::from_step(STEP, PullErrorKind::BadData)
                .set_message("bad category name")
                .to_result()
        }
        Some(v) => v.to_string(),
    };
    return Ok((identity, name_url));
}

fn is_ignored_category(identity: u64) -> bool {
    return identity == 2 || identity == 1501;
}

fn is_empty_data(data: &Value) -> bool {
    let result_type = match data["resultType"].as_str() {
        None => return false,
        Some(v) => v,
    };
    return result_type == "zero_result";
}
