use crate::result::BoxResult;

pub fn str_to_u64(value: &str) -> BoxResult<u64> {
    let regular_str = value.replace(" ", "").replace(",", "");
    let result = regular_str.parse::<u64>()?;
    return Ok(result);
}

pub fn str_to_f64(value: &str) -> BoxResult<f64> {
    let regular_str = value.replace(" ", "").replace(",", "");
    let result = regular_str.parse::<f64>()?;
    return Ok(result);
}
