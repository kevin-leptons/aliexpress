use serde::{Deserialize, Serialize};
use serde_repr::Serialize_repr;
use std::collections::VecDeque;
use std::error::Error;
use std::fmt::Debug;
use std::path::Path;
use std::{fmt, fs};
use ureq::Response;
use url::Url;

pub type BoxResult<T> = Result<T, Box<dyn Error>>;
pub type PullResult<T> = Result<T, PullError>;

#[derive(Debug, Clone)]
pub struct UnexpectedError {
    message: String,
}

impl fmt::Display for UnexpectedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for UnexpectedError {
    fn description(&self) -> &str {
        return &self.message;
    }
}

impl UnexpectedError {
    pub fn new(message: &str) -> UnexpectedError {
        return UnexpectedError {
            message: message.to_string(),
        };
    }

    pub fn new_as_box_result<T>(message: &str) -> BoxResult<T> {
        let e = UnexpectedError::new(message);
        return Err(Box::new(e));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PullErrorKind {
    Http = 1,
    NoData = 2,
    BadData = 3,
    Database = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullError {
    steps: VecDeque<String>,
    kind: PullErrorKind,
    message: Option<String>,
    http_url: Option<String>,
    http_status: Option<u16>,

    #[serde(skip_serializing)]
    http_body: Option<String>,
    http_transport_error: Option<String>,

    #[serde(skip_serializing)]
    http_fragment: Option<String>,

    pub skip: bool,
}

impl fmt::Display for PullError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:#?}", self)
    }
}

impl Error for PullError {
    fn description(&self) -> &str {
        return match &self.message {
            None => "no message from error",
            Some(v) => v,
        };
    }
}

impl PullError {
    pub fn from_http_error(step: &str, url: &Url, e: &ureq::Error) -> Self {
        return PullError {
            steps: VecDeque::from_iter([step.to_string()]),
            kind: PullErrorKind::Http,
            message: None,
            http_url: Some(url.to_string()),
            http_status: None,
            http_body: None,
            http_transport_error: Some(e.to_string()),
            http_fragment: None,
            skip: false,
        };
    }

    pub fn from_step(step: &str, kind: PullErrorKind) -> Self {
        return PullError {
            steps: VecDeque::from_iter([step.to_string()]),
            kind: kind,
            message: None,
            http_url: None,
            http_status: None,
            http_body: None,
            http_transport_error: None,
            http_fragment: None,
            skip: false,
        };
    }

    pub fn stack_step(mut self, step: &str) -> Self {
        self.steps.push_front(step.to_string());
        return self;
    }

    pub fn set_http_context(mut self, url: &Url, status: u16, body: &str) -> Self {
        self.http_url = Some(url.to_string());
        self.http_status = Some(status);
        self.http_body = Some(body.to_string());
        return self;
    }

    pub fn set_http_fragment(mut self, fragment: &str) -> Self {
        self.http_fragment = Some(fragment.to_string());
        return self;
    }

    pub fn set_message(mut self, message: &str) -> Self {
        self.message = Some(message.to_string());
        return self;
    }

    pub fn set_http_url(mut self, url: &Url) -> Self {
        self.http_url = Some(url.to_string());
        return self;
    }

    pub fn set_skip(mut self, skip: bool) -> Self {
        self.skip = skip;
        return self;
    }

    pub fn to_result<T>(self) -> PullResult<T> {
        return Err(self);
    }

    pub fn write(&self, dir_path: &Path) -> BoxResult<()> {
        fs::create_dir_all(dir_path)?;
        self.write_index_html(dir_path)?;
        self.write_error_json(dir_path)?;
        self.write_http_body_html(dir_path)?;
        self.write_http_fragment_html(dir_path)?;
        return Ok(());
    }

    fn write_error_json(&self, dir_path: &Path) -> BoxResult<()> {
        let file_path = dir_path.join("error.json");
        let data = serde_json::to_string_pretty(self)?;
        fs::write(file_path, data)?;
        return Ok(());
    }

    fn write_index_html(&self, dir_path: &Path) -> BoxResult<()> {
        let file_path = dir_path.join("index.html");
        let data = r#"
            <html>
                <body>
                    <ul>
                        <li>
                            <a href="error.json">error</a>
                        </li>
                        <li>
                            <a href="http_body.html">http_body</a>
                        </li>
                        <li>
                            <a href="http_fragment.html">http_fragment</a>
                        </li>
                    </ul>
                </body>
            </html>
        "#;
        fs::write(file_path, data)?;
        return Ok(());
    }

    fn write_http_body_html(&self, dir_path: &Path) -> BoxResult<()> {
        let file_path = dir_path.join("http_body.html");
        let data = match &self.http_body {
            None => "",
            Some(v) => v.as_str(),
        };
        fs::write(file_path, data)?;
        return Ok(());
    }

    fn write_http_fragment_html(&self, dir_path: &Path) -> BoxResult<()> {
        let file_path = dir_path.join("http_fragment.html");
        let data = match &self.http_fragment {
            None => "",
            Some(v) => v.as_str(),
        };
        fs::write(file_path, data)?;
        return Ok(());
    }
}
