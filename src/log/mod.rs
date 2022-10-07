#[cfg(test)]
mod tests;

use chrono::Utc;
use std::fmt::{write, Arguments};
use std::io::{stdout, Stdout, Write};
use std::path::{Path, PathBuf};
use ureq::post;

pub static DATE_TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub struct Log<'a> {
    out: &'a mut dyn Write,
    namespace: String,
}

impl<'a> Log<'a> {
    pub fn new(namespace: &str, out: &'a mut dyn Write) -> Self {
        return Self {
            out: out,
            namespace: namespace.to_string(),
        };
    }

    pub fn info(&mut self, line: &str) {
        self.writeln("INFO", line);
    }

    pub fn error(&mut self, line: &str) {
        self.writeln("ERROR", line);
    }

    pub fn fork(name: &str, log: &'a mut Log) -> Log<'a> {
        let namespace = format!("{}.{}", log.namespace, name);
        return Self::new(namespace.as_str(), log.out);
    }

    fn writeln(&mut self, level: &str, line: &str) {
        let timestamp = Utc::now().format(DATE_TIME_FORMAT);
        match writeln!(
            self.out,
            "{} {: <6} - {} - {}",
            timestamp,
            level,
            self.namespace.as_str(),
            line
        ) {
            Err(e) => println!("ERROR: failed writing log: {}", e.to_string().as_str()),
            Ok(_) => {}
        }
    }
}

#[macro_export]
macro_rules! info {
    ($log: expr, $format: expr) => {
        $log.info(format!($format).as_str());
    };
    ($log: expr, $format: expr, $($arg: tt)*) => {
        $log.info(format!($format, $($arg)*).as_str());
    };
}

#[macro_export]
macro_rules! error {
    ($log: expr, $format: expr) => {
        $log.error(format!($format).as_str());
    };
    ($log: expr, $format: expr, $($arg: tt)*) => {
        $log.error(format!($format, $($arg)*).as_str());
    };
}

pub struct LogFileSystem {
    root: PathBuf,
}

impl LogFileSystem {
    pub fn new(root: PathBuf) -> Self {
        return Self { root };
    }

    pub fn get_path(&self, postfix: &str, child_path: Option<&Path>) -> PathBuf {
        let datetime = chrono::Utc::now().format("%Y%m%dT%H%M%S").to_string();
        let file_name = format!("{}_{}", datetime, postfix);
        return match child_path {
            None => self.root.join(file_name),
            Some(v) => self.root.join(v).join(file_name),
        };
    }
}

pub use info;
