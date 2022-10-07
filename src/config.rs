use crate::result::{BoxResult, UnexpectedError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_log_directory")]
    pub log_directory: PathBuf,

    #[serde(default = "default_report_directory")]
    pub report_directory: PathBuf,

    #[serde(default = "default_asset_directory")]
    pub asset_directory: PathBuf,

    #[serde(default = "default_mongo_endpoint")]
    pub mongo_endpoint: Url,

    pub pulling: PullingConfig,
    pub analyst: AnalystConfig,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PullingConfig {
    pub category: Vec<u64>,

    #[serde(default = "default_category_pages")]
    pub category_pages: u32,
}

impl Default for PullingConfig {
    fn default() -> Self {
        return Self {
            category: vec![],
            category_pages: 2,
        };
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalystConfig {
    pub miner_cost_lower: f64,
    pub miner_cost_upper: f64,
    pub miner_orders_lower: u64,
    pub miner_orders_upper: u64,
}

impl Default for AnalystConfig {
    fn default() -> Self {
        return Self {
            miner_cost_lower: 5.0,
            miner_cost_upper: 10.0,
            miner_orders_lower: 0,
            miner_orders_upper: 10,
        };
    }
}

fn default_mongo_endpoint() -> Url {
    return Url::parse("mongodb://localhost/dropshipping").unwrap();
}

fn default_category_pages() -> u32 {
    return 2;
}

fn default_log_directory() -> PathBuf {
    return PathBuf::from("log");
}

fn default_report_directory() -> PathBuf {
    return PathBuf::from("report");
}

fn default_asset_directory() -> PathBuf {
    return PathBuf::from("asset");
}

impl Config {
    pub fn new(
        log_directory: PathBuf,
        report_directory: PathBuf,
        asset_directory: PathBuf,
    ) -> BoxResult<Self> {
        let config = Config {
            log_directory: log_directory,
            mongo_endpoint: Url::parse("mongodb://localhost/dropshipping")?,
            report_directory: report_directory,
            asset_directory: asset_directory,
            pulling: PullingConfig::default(),
            analyst: AnalystConfig::default(),
        };
        return Ok(config);
    }

    pub fn from_json_file(file_path: Option<PathBuf>) -> BoxResult<Self> {
        return match file_path {
            Some(v) => Self::from_exact_file(&v),
            None => Self::from_default_files(),
        };
    }

    fn from_exact_file(file_path: &PathBuf) -> BoxResult<Self> {
        let json_str = fs::read_to_string(file_path)?;
        let config = match Self::from_json_string(&json_str) {
            Err(e) => {
                return UnexpectedError::new_as_box_result(
                    format!(
                        "bad configuration file: {}, err={}",
                        file_path.display(),
                        e.to_string().as_str()
                    )
                    .as_str(),
                )
            }
            Ok(v) => v,
        };
        if config.analyst.miner_cost_lower > config.analyst.miner_cost_upper {
            return UnexpectedError::new_as_box_result(
                "analyst.miner_cost_lower must not greater than miner_cost_upper",
            );
        }
        return Ok(config);
    }

    fn from_default_files() -> BoxResult<Self> {
        let config_name = "dropshipping";
        let file_name = "config.json";
        let file_paths = Vec::from([
            PathBuf::from(file_name),
            PathBuf::from("~/config").join(config_name).join(file_name),
            PathBuf::from("/etc").join(config_name).join(file_name),
        ]);
        for file_path in file_paths {
            let config = match Self::from_default_file(&file_path)? {
                None => continue,
                Some(v) => v,
            };
            return Ok(config);
        }
        return UnexpectedError::new_as_box_result("no configuration file");
    }

    fn from_json_string(data: &String) -> BoxResult<Self> {
        let config: Self = serde_json::from_str(data)?;
        return Ok(config);
    }

    fn from_default_file(file_path: &PathBuf) -> BoxResult<Option<Self>> {
        let json_str = match Self::read_default_file(file_path)? {
            None => return Ok(None),
            Some(v) => v,
        };
        let config = match Self::from_json_string(&json_str) {
            Err(e) => {
                return UnexpectedError::new_as_box_result(
                    format!(
                        "bad configuration file: {}, err={}",
                        file_path.display(),
                        e.to_string().as_str()
                    )
                    .as_str(),
                )
            }
            Ok(v) => v,
        };
        return Ok(Some(config));
    }

    fn read_default_file(file_path: &PathBuf) -> BoxResult<Option<String>> {
        let data = match fs::read_to_string(file_path) {
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    return Ok(None);
                }
                return Err(Box::new(e));
            }
            Ok(v) => v,
        };
        return Ok(Some(data));
    }
}
