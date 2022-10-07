use crate::aliexpress_provider::formatter::str_to_u64;
use crate::aliexpress_provider::virtual_user::{ImmutableResponse, VirtualUser};
use crate::provider::{Store, Timestamp};
use crate::result::{BoxResult, PullError, PullErrorKind, PullResult, UnexpectedError};
use crate::scraper_extension::{
    get_inner_text_element, query_selector_all_html, query_selector_element, query_selector_html,
};
use chrono::{NaiveDate, NaiveDateTime, Utc};
use js_sandbox::Script;
use mongodb::options::ReturnDocument;
use scraper::{ElementRef, Html};
use serde_json::Value;
use std::borrow::{Borrow, Cow};
use std::io::Split;
use std::os::unix::raw::uid_t;
use url::Url;

pub fn get_store_by_owner_id(identity: u64, user: &mut VirtualUser) -> PullResult<Store> {
    static STEP: &str = "get_store_by_owner_id";
    let res = match get_feedback_page_by_owner_id(identity, user) {
        Err(e) => return e.stack_step(STEP).to_result(),
        Ok(v) => v,
    };
    let store = match extract_store(&res.body, identity) {
        Err(e) => {
            return e
                .stack_step(STEP)
                .set_http_context(&res.url, res.status, &res.body)
                .to_result()
        }
        Ok(v) => v,
    };
    return Ok(store);
}

// https://feedback.aliexpress.com//display/evaluationDetail.htm?ownerMemberId=252607694
pub fn get_feedback_page_by_owner_id(
    identity: u64,
    user: &mut VirtualUser,
) -> PullResult<ImmutableResponse> {
    static STEP: &str = "get_feedback_page_by_owner_id";
    let identity_str = identity.to_string();
    let mut url =
        Url::parse("https://feedback.aliexpress.com//display/evaluationDetail.htm").unwrap();
    url.query_pairs_mut()
        .append_pair("ownerMemberId", identity_str.as_str());
    return user.get(&url);
}

// https://www.aliexpress.com/store/feedback-score/912322050.html
// https://feedback.aliexpress.com//display/evaluationDetail.htm?ownerMemberId=252607694&memberType=seller&callType=iframe&iframe_delete=true
pub fn get_store(identity: u64, user: &mut VirtualUser) -> BoxResult<Store> {
    todo!()
    // let page = get_feedback_page(identity, user)?;
    // return extract_store(&page);
}

fn get_feedback_page(store_id: u64, user: &mut VirtualUser) -> BoxResult<String> {
    let url = get_feedback_page_url(store_id, user)?;
    let res = user.get(&url)?;
    return Ok(res.body);
}

// https://feedback.aliexpress.com//display/evaluationDetail.htm?ownerMemberId=252607694
pub(super) fn get_feedback_page_url(store_id: u64, user: &mut VirtualUser) -> BoxResult<Url> {
    let owner_id = get_store_owner_identity(store_id, user)?;
    let mut url =
        Url::parse("https://feedback.aliexpress.com//display/evaluationDetail.htm").unwrap();
    url.query_pairs_mut()
        .append_pair("ownerMemberId", owner_id.to_string().as_str());
    return Ok(url);
}

pub(super) fn get_store_owner_identity(
    store_identity: u64,
    user: &mut VirtualUser,
) -> BoxResult<u64> {
    let store_page = get_store_page(store_identity, user)?;
    return extract_store_owner_identity(&store_page);
}

// https://m.aliexpress.com/store/912322050?trace=store2mobilestoreNew&spm=a2g0o.productlist.0.0.58cd4d36ZS4JHD
pub(super) fn get_store_page(identity: u64, user: &mut VirtualUser) -> BoxResult<String> {
    let base_url = Url::parse("https://m.aliexpress.com/store/912322050").unwrap();
    let url = base_url.join(identity.to_string().as_str()).unwrap();
    let res = user.get(&url)?;
    return Ok(res.body);
}

pub(super) fn extract_store_owner_identity(page: &String) -> BoxResult<u64> {
    let html = Html::parse_document(&page);
    let data_script = extract_store_owner_data_script(&html)?;
    let data = extract_store_owner_data(&data_script)?;
    return extract_store_owner_identity_from_url(&data);
}

fn extract_store_owner_data_script(html: &Html) -> BoxResult<String> {
    let nodes = query_selector_all_html(html, "script")?;
    for node in nodes {
        let script = node.inner_html();
        if script.contains("window.shopPageDataApi = ") {
            return Ok(script);
        }
    }
    return UnexpectedError::new_as_box_result("data not found");
}

fn extract_store_owner_data(code: &String) -> BoxResult<String> {
    let js_code_base = String::from("window = {}; AES_CONFIG = {};\n");
    let js_inspect_code = "function getData() {return window.shopPageDataApi};\n";
    let js_code = js_code_base + &code + ";\n" + js_inspect_code;
    let mut script = match Script::from_string(&js_code.to_string()) {
        Err(e) => return UnexpectedError::new_as_box_result(&e.to_string()),
        Ok(v) => v,
    };
    let arg = 0;
    let value: Value = match script.call("getData", &arg) {
        Err(e) => return UnexpectedError::new_as_box_result(&e.to_string()),
        Ok(v) => v,
    };
    let data = match value {
        Value::String(v) => v,
        _ => return UnexpectedError::new_as_box_result("bad data"),
    };
    return Ok(data);
}

fn extract_store_owner_identity_from_url(url_str: &String) -> BoxResult<u64> {
    let url = Url::parse(url_str)?;
    let matched_param = url.query_pairs().find(|(key, _)| {
        return key == &Cow::Borrowed("sellerId");
    });
    let owner_id_str = match matched_param {
        None => return UnexpectedError::new_as_box_result("no query param sellerId"),
        Some((_, v)) => v.to_string(),
    };
    let owner_id = match owner_id_str.parse::<u64>() {
        Err(e) => return Err(Box::new(e)),
        Ok(v) => v,
    };
    return Ok(owner_id);
}

pub(super) fn extract_store(page: &String, owner_id: u64) -> PullResult<Store> {
    static STEP: &str = "extract_store";
    let html = Html::parse_document(&page);
    if is_feedback_page(&html) == false {
        return PullError::from_step(STEP, PullErrorKind::NoData).to_result();
    }
    let store = Store {
        identity: extract_store_identity(&html)?,
        owner_identity: owner_id,
        name: extract_store_name(&html)?,
        rating45_ratio: extract_rating45_ratio(&html)?,
        rating45_count: extract_rating45_count(&html)?,
        rating3_count: extract_rating3_count(&html)?,
        rating12_count: extract_rating12_count(&html)?,
        online_at: extract_online_timestamp(&html)?,
        modified_at: Utc::now().naive_utc(),
    };
    return Ok(store);
}

fn is_feedback_page(html: &Html) -> bool {
    let nodes = query_selector_all_html(html, "#feedback-detail.clearfix").unwrap();
    return nodes.len() == 1;
}

pub fn extract_store_identity(html: &Html) -> PullResult<u64> {
    static STEP: &str = "extract_store_identity";
    let node = match query_selector_html(
        html,
        "#feedback-summary > div.middle.middle-seller > table > tbody > tr:nth-child(1) > td > a",
    ) {
        None => {
            return {
                return PullError::from_step(STEP, PullErrorKind::NoData)
                    .set_skip(true)
                    .to_result();
            }
        }
        Some(v) => v,
    };
    let url = match node.value().attr("href") {
        None => {
            return PullError::from_step(STEP, PullErrorKind::NoData)
                .set_http_fragment(node.html().as_str())
                .to_result()
        }
        Some(v) => {
            if v == "#" {
                return PullError::from_step(STEP, PullErrorKind::NoData)
                    .set_message("nothing on store url")
                    .set_skip(true)
                    .to_result();
            }
            "https://".to_owned() + v
        }
    };
    let identity = match extract_store_identity_from_url(&url) {
        Err(e) => {
            return e
                .stack_step(STEP)
                .set_http_fragment(node.html().as_str())
                .to_result();
        }
        Ok(v) => v,
    };
    return Ok(identity);
}

fn extract_store_identity_from_url(url_str: &String) -> PullResult<u64> {
    static STEP: &str = "extract_store_identity_from_url";
    let url = match Url::parse(url_str) {
        Err(_) => return PullError::from_step(STEP, PullErrorKind::BadData).to_result(),
        Ok(v) => v,
    };
    let path_segments = match url.path_segments() {
        None => return PullError::from_step(STEP, PullErrorKind::BadData).to_result(),
        Some(v) => v.collect::<Vec<&str>>(),
    };
    if path_segments.len() != 2 || path_segments[0] != "store" {
        return PullError::from_step(STEP, PullErrorKind::BadData).to_result();
    }
    let id: u64 = match path_segments[1].parse() {
        Err(_) => return PullError::from_step(STEP, PullErrorKind::BadData).to_result(),
        Ok(v) => v,
    };
    return Ok(id);
}

fn extract_store_name(html: &Html) -> PullResult<String> {
    static STEP: &str = "extract_store_name";
    let node = match query_selector_html(
        html,
        "#feedback-summary > div.middle.middle-seller > table > tbody > tr:nth-child(1) > td > a",
    ) {
        None => return PullError::from_step(STEP, PullErrorKind::NoData).to_result(),
        Some(v) => v,
    };
    let name = get_inner_text_element(&node);
    return Ok(name);
}

fn extract_rating45_ratio(html: &Html) -> PullResult<f64> {
    static STEP: &str = "extract_rating45_ratio";
    let node = match query_selector_html(
        html,
        "#feedback-history > div.middle > table > tbody > tr:nth-child(5) > td:nth-child(4)",
    ) {
        None => return PullError::from_step(STEP, PullErrorKind::NoData).to_result(),
        Some(v) => v,
    };
    let rating_str = get_inner_text_element(&node);
    let rating_str_trimmed = rating_str.trim();
    if rating_str_trimmed == "-" {
        return Ok(0.0);
    }
    let rating = match rating_str_trimmed.replace("%", "").parse::<f64>() {
        Err(_) => {
            return PullError::from_step(STEP, PullErrorKind::BadData)
                .set_http_fragment(node.html().as_str())
                .to_result()
        }
        Ok(v) => v,
    };
    return Ok(rating);
}

fn extract_rating45_count(html: &Html) -> PullResult<u64> {
    static STEP: &str = "extract_rating45_count";
    let node = match query_selector_html(
        html,
        "#feedback-history > div.middle > table > tbody > tr:nth-child(2) > td:nth-child(4) > a",
    ) {
        None => return PullError::from_step(STEP, PullErrorKind::NoData).to_result(),
        Some(v) => v,
    };
    let count_str = get_inner_text_element(&node);
    let count_str_trimmed = count_str.trim();
    if count_str_trimmed == "-" {
        return Ok(0);
    }
    let count = match str_to_u64(count_str_trimmed) {
        Err(_) => return PullError::from_step(STEP, PullErrorKind::BadData).to_result(),
        Ok(v) => v,
    };
    return Ok(count);
}

fn extract_rating3_count(html: &Html) -> PullResult<u64> {
    static STEP: &str = "extract_rating3_count";
    let node = match query_selector_html(
        html,
        "#feedback-history > div.middle > table > tbody > tr:nth-child(3) > td:nth-child(4) > a",
    ) {
        None => return PullError::from_step(STEP, PullErrorKind::NoData).to_result(),
        Some(v) => v,
    };
    let count_str = get_inner_text_element(&node);
    let count_str_trimmed = count_str.trim();
    if count_str_trimmed == "-" {
        return Ok(0);
    }
    let count = match str_to_u64(count_str_trimmed) {
        Err(_) => return PullError::from_step(STEP, PullErrorKind::BadData).to_result(),
        Ok(v) => v,
    };
    return Ok(count);
}

fn extract_rating12_count(html: &Html) -> PullResult<u64> {
    static STEP: &str = "extract_rating12_count";
    let node = match query_selector_html(
        html,
        "#feedback-history > div.middle > table > tbody > tr:nth-child(4) > td:nth-child(4) > a",
    ) {
        None => return PullError::from_step(STEP, PullErrorKind::NoData).to_result(),
        Some(v) => v,
    };
    let count_str = get_inner_text_element(&node);
    let count_str_trimmed = count_str.trim();
    if count_str_trimmed == "-" {
        return Ok(0);
    }
    let count = match str_to_u64(count_str_trimmed) {
        Err(_) => return PullError::from_step(STEP, PullErrorKind::BadData).to_result(),
        Ok(v) => v,
    };
    return Ok(count);
}

fn extract_online_timestamp(html: &Html) -> PullResult<NaiveDateTime> {
    static STEP: &str = "extract_online_timestamp";

    let date_node = match query_selector_html(
        html,
        "#feedback-summary > div.middle.middle-seller > table > tbody > tr:nth-child(3) > td",
    ) {
        None => return PullError::from_step(STEP, PullErrorKind::NoData).to_result(),
        Some(v) => v,
    };
    let date_str = get_inner_text_element(&date_node);
    let datetime_str = date_str.trim().to_string() + " 00:00:00";
    let date = match NaiveDateTime::parse_from_str(datetime_str.as_str(), "%e %b %Y %H:%M:%S") {
        Err(_) => return PullError::from_step(STEP, PullErrorKind::BadData).to_result(),
        Ok(v) => v,
    };
    return Ok(date);
}
