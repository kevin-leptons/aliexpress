use js_sandbox::Script;

pub fn get_rhs_object(lhs: &str, script: &str, limit: usize) -> Vec<String> {
    let mut rhs_list = Vec::new();
    let mut from_index = 0;
    loop {
        if rhs_list.len() >= limit {
            break;
        }
        let (from, to) = match get_rhs_object_one(lhs, from_index, script) {
            None => break,
            Some(v) => v,
        };
        let rhs = script[from..to + 1].chars().collect();
        rhs_list.push(rhs);
        from_index = to;
    }
    return rhs_list;
}

fn get_rhs_object_one(lhs: &str, from: usize, script: &str) -> Option<(usize, usize)> {
    let target_script = &script[from..];
    let lhs_begin = match target_script.find(lhs) {
        None => return None,
        Some(v) => v,
    };
    let rhs_begin = match target_script[lhs_begin..].find("{") {
        None => return None,
        Some(v) => v + lhs_begin,
    };
    let mut count = 1usize;
    let mut rhs_end = rhs_begin;
    for symbol in target_script.chars().skip(rhs_begin + 1) {
        rhs_end = rhs_end + 1;
        if symbol == '{' {
            count = count + 1;
        }
        if symbol == '}' {
            count = count - 1;
        }
        if count == 0 {
            break;
        }
    }
    return Some((rhs_begin + from, rhs_end + from));
}
