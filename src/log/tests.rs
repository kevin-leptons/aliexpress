use crate::error;
use crate::log::{info, Log};
use std::io::{stdout, Write};

#[test]
fn test_info() {
    let mut out: Vec<u8> = Vec::new();
    let mut log = Log::new("foo.bar.com", &mut out);
    info!(log, "hello world");
    let line = String::from_utf8(out).unwrap();
    let data = line[19..].to_string();
    assert_eq!(data, " INFO   - foo.bar.com - hello world\n");
}

#[test]
fn test_info_with_arguments() {
    let mut out: Vec<u8> = Vec::new();
    let mut log = Log::new("foo.bar.com", &mut out);
    info!(log, "hello {}", "world");
    let line = String::from_utf8(out).unwrap();
    let data = line[19..].to_string();
    assert_eq!(data, " INFO   - foo.bar.com - hello world\n");
}

#[test]
fn test_error() {
    let mut out: Vec<u8> = Vec::new();
    let mut log = Log::new("foo.bar.com", &mut out);
    error!(log, "hello world");
    let line = String::from_utf8(out).unwrap();
    let data = line[19..].to_string();
    assert_eq!(data, " ERROR  - foo.bar.com - hello world\n");
}

#[test]
fn test_error_with_arguments() {
    let mut out: Vec<u8> = Vec::new();
    let mut log = Log::new("foo.bar.com", &mut out);
    error!(log, "hello {}", "world");
    let line = String::from_utf8(out).unwrap();
    let data = line[19..].to_string();
    assert_eq!(data, " ERROR  - foo.bar.com - hello world\n");
}

#[test]
fn test_fork() {
    let mut out: Vec<u8> = Vec::new();
    let mut parent = Log::new("foo.bar", &mut out);
    let mut child = Log::fork("com", &mut parent);
    info!(child, "hello world");
    let line = String::from_utf8(out).unwrap();
    let data = line[19..].to_string();
    assert_eq!(data, " INFO   - foo.bar.com - hello world\n");
}

#[test]
fn test_fork_mutability() {
    let mut out: Vec<u8> = Vec::new();
    let mut parent = Log::new("foo.bar", &mut out);
    let mut child = Log::fork("com", &mut parent);
    info!(child, "hello world");
    let mut grant_child = Log::fork("net", &mut child);
    info!(grant_child, "hello world");
    info!(parent, "hello world");
}
