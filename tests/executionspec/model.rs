use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use derive_setters::Setters;
use serde::{Deserialize, Serialize};
use tailcall::config::Source;
use tailcall::http::Method;
use url::Url;

//
pub struct Env {
    pub env: HashMap<String, String>,
}

//
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Annotation {
    Skip,
}

#[derive(Clone, Setters)]
pub struct ExecutionSpec {
    pub path: PathBuf,
    pub name: String,
    pub safe_name: String,

    pub server: Vec<(Source, String)>,
    pub mock: Option<Vec<Mock>>,
    pub env: Option<HashMap<String, String>>,
    pub test: Option<Vec<APIRequest>>,
    pub files: BTreeMap<String, String>,

    // Annotations for the runner
    pub runner: Option<Annotation>,

    pub check_identity: bool,
    pub sdl_error: bool,
}

//
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct APIRequest {
    #[serde(default)]
    pub method: Method,
    pub url: Url,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub body: serde_json::Value,
    #[serde(default)]
    pub test_traces: bool,
    #[serde(default)]
    pub test_metrics: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct APIResponse {
    #[serde(default = "default_status")]
    pub status: u16,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub body: serde_json::Value,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_body: Option<String>,
}

fn default_status() -> u16 {
    200
}

fn default_expected_hits() -> usize {
    1
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct UpstreamRequest(pub APIRequest);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpstreamResponse(pub APIResponse);

//
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Mock {
    pub request: UpstreamRequest,
    pub response: UpstreamResponse,
    #[serde(default = "default_expected_hits")]
    pub expected_hits: usize,
}

#[derive(Clone, Debug)]
pub struct ExecutionMock {
    pub mock: Mock,
    pub actual_hits: Arc<AtomicUsize>,
}

#[derive(Clone, Debug)]
pub struct MockHttpClient {
    pub mocks: Vec<ExecutionMock>,
    pub spec_path: String,
}

pub struct MockFileSystem {
    pub spec: ExecutionSpec,
}

//
#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct SDLError {
    pub message: String,
    pub trace: Vec<String>,
    pub description: Option<String>,
}

//
#[derive(Clone)]
pub struct TestFileIO {}

#[derive(Clone)]
pub struct TestEnvIO {
    pub vars: HashMap<String, String>,
}
