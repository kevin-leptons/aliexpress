use crate::result::{BoxResult, UnexpectedError};
use scraper;
use scraper::element_ref::Select;
use scraper::selector::Selector;
use scraper::{ElementRef, Html};

pub struct HtmlSelectRef<'a> {
    pub value: scraper::html::Select<'a, 'a>,
    // selector: &'a Selector,
}

pub fn query_selector_all_html<'a>(
    html: &'a Html,
    selector: &str,
) -> BoxResult<Vec<ElementRef<'a>>> {
    let parsed_selector = match Selector::parse(selector) {
        Err(_) => return UnexpectedError::new_as_box_result("bad css selector"),
        Ok(v) => v,
    };
    let elements = html.select(&parsed_selector).collect::<Vec<ElementRef>>();
    return Ok(elements);
}

pub fn query_selector_html<'a>(html: &'a Html, selector: &str) -> Option<ElementRef<'a>> {
    let parsed_selector = Selector::parse(selector).unwrap();
    return html.select(&parsed_selector).nth(0);
}

pub fn query_selector_all_element<'a>(
    element: &'a ElementRef,
    selector: &str,
) -> BoxResult<Vec<ElementRef<'a>>> {
    let parsed_selector = match Selector::parse(selector) {
        Err(_) => return UnexpectedError::new_as_box_result("bad css selector"),
        Ok(v) => v,
    };
    let elements = element.select(&parsed_selector).collect();
    return Ok(elements);
}

pub fn query_selector_element<'a>(root: &'a ElementRef, selector: &str) -> Option<ElementRef<'a>> {
    let parsed_selector = match Selector::parse(selector) {
        Err(e) => panic!("bad css selector"),
        Ok(v) => v,
    };
    let node = match root.select(&parsed_selector).nth(0) {
        None => return None,
        Some(v) => v,
    };
    return Some(node);
}

pub fn get_inner_text_element(node: &ElementRef) -> String {
    return node.text().collect::<Vec<&str>>().join(" ");
}
