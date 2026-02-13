use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

// --- core blueprint types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub name: String,
    pub meta: ProjectMeta,
    pub config: Config,
    pub phases: Vec<Phase>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub slug: Option<String>,
    pub short_description: Option<String>,
    pub long_description: Option<String>,
    pub headline: Option<String>,
    pub runner_image: Option<String>,
    pub markdown: Option<String>,
    pub is_published: bool,
    pub is_featured: bool,
    pub is_challenge: bool,
    pub unlock_mode: String,
    pub featured_order: Option<u32>,
    pub published_at: Option<String>,
    pub related_course_slug: Option<String>,
    pub features: Vec<Feature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub title: String,
    pub description: String,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: Option<u16>,
    #[serde(with = "duration_serde")]
    pub timeout: Duration,
    pub env: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: None,
            timeout: Duration::from_secs(30),
            env: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    pub name: String,
    pub depends_on: Vec<String>,
    pub steps: Vec<Step>,
}

// --- step types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub name: String,
    pub meta: StepMeta,
    pub requires: Vec<String>,
    #[serde(with = "option_duration_serde")]
    pub timeout: Option<Duration>,
    pub retry: Option<RetryConfig>,
    pub inputs: Vec<InputDecl>,
    pub probe: Probe,
    pub expectations: Vec<Expectation>,
    pub captures: Vec<Capture>,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepMeta {
    pub slug: Option<String>,
    pub description: Option<String>,
    pub points: u32,
    pub scores: Option<String>,
    pub is_free: bool,
    pub visibility_level: u8,
    pub abandoned_deduction: u32,
    pub hints: Vec<Hint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hint {
    pub text: String,
    pub unlock_criteria: Option<String>,
    pub points_deduction: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    #[serde(with = "duration_serde")]
    pub delay: Duration,
}

// --- probe types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Probe {
    Tcp(TcpProbe),
    Udp(UdpProbe),
    Http(HttpProbe),
    Exec(ExecProbe),
    File(FileProbe),
    Process(ProcessProbe),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpProbe {
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpProbe {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpProbe {
    pub method: HttpMethod,
    pub path: String,
    pub body: Option<String>,
    pub headers: HashMap<String, String>,
    pub mode: HttpMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecProbe {
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileProbe {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessProbe {
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

impl HttpMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(Self::GET),
            "POST" => Some(Self::POST),
            "PUT" => Some(Self::PUT),
            "DELETE" => Some(Self::DELETE),
            "PATCH" => Some(Self::PATCH),
            "HEAD" => Some(Self::HEAD),
            "OPTIONS" => Some(Self::OPTIONS),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::GET => "GET",
            Self::POST => "POST",
            Self::PUT => "PUT",
            Self::DELETE => "DELETE",
            Self::PATCH => "PATCH",
            Self::HEAD => "HEAD",
            Self::OPTIONS => "OPTIONS",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpMode {
    Single,
    Concurrent { clients: u32 },
    Keepalive { requests: u32 },
    Pipelined { requests: u32 },
    Burst { count: u32, window_ms: u64 },
    Chunked,
}

impl Default for HttpMode {
    fn default() -> Self {
        Self::Single
    }
}

// --- expect types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expectation {
    pub field: FieldPath,
    pub op: Op,
    pub expected: ExpectedValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    Eq,
    Contains,
    StartsWith,
    Matches,
    MatchesFile,
    Present,
    Absent,
    Gt,
    Lt,
    Gte,
    Lte,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExpectedValue {
    Int(i64),
    Str(String),
    Bool(bool),
    DurationMs(u64),
    Regex(String),
    FilePath(String),
    Variable(String),
}

impl ExpectedValue {
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capture {
    pub field: FieldPath,
    pub variable: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FieldPath {
    pub segments: Vec<PathSegment>,
}

impl FieldPath {
    pub fn simple(name: &str) -> Self {
        Self {
            segments: vec![PathSegment::Key(name.to_string())],
        }
    }

    pub fn from_dotted(path: &str) -> Self {
        let segments = path
            .split('.')
            .map(|part| {
                // check for array index: field[0]
                if let Some(bracket_pos) = part.find('[') {
                    let key = &part[..bracket_pos];
                    let idx_str = &part[bracket_pos + 1..part.len() - 1];
                    let mut segs = vec![PathSegment::Key(key.to_string())];
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        segs.push(PathSegment::Index(idx));
                    }
                    segs
                } else {
                    vec![PathSegment::Key(part.to_string())]
                }
            })
            .flatten()
            .collect();

        Self { segments }
    }

    pub fn to_string(&self) -> String {
        self.segments
            .iter()
            .map(|s| match s {
                PathSegment::Key(k) => k.clone(),
                PathSegment::Index(i) => format!("[{i}]"),
            })
            .collect::<Vec<_>>()
            .join(".")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PathSegment {
    Key(String),
    Index(usize),
}

// --- input types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputDecl {
    pub name: String,
    pub value_type: InputType,
    pub validation: Option<InputValidation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputType {
    Str,
    Number,
    Bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputValidation {
    Matches(String),
    Min(i64),
    Max(i64),
    OneOf(Vec<String>),
}

// --- result types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintResult {
    pub name: String,
    pub status: Status,
    pub phases: Vec<PhaseResult>,
    pub duration_ms: u64,
    pub captured: HashMap<String, Value>,
    pub input_provided: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseResult {
    pub name: String,
    pub status: Status,
    pub steps: Vec<StepResult>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub name: String,
    pub status: Status,
    pub expectations: Vec<ExpectResult>,
    pub captures: Vec<(String, Value)>,
    pub input_matched: Option<bool>,
    pub duration_ms: u64,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectResult {
    pub field: String,
    pub op: Op,
    pub status: Status,
    pub actual: Option<Value>,
    pub expected_display: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Passed,
    Failed,
    Skipped,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(n) => Some(*n),
            Self::Float(f) => Some(*f as i64),
            Self::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Int(n) => Some(*n as f64),
            Self::String(s) => s.parse().ok(),
            _ => None,
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => write!(f, "{s}"),
            Self::Int(n) => write!(f, "{n}"),
            Self::Float(n) => write!(f, "{n}"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Null => write!(f, "null"),
        }
    }
}

// --- probe result (runtime) ---

#[derive(Debug, Clone, Default)]
pub struct ProbeResult {
    pub fields: HashMap<String, Value>,
    pub raw_stdout: Option<String>,
    pub duration_ms: u64,
}

impl ProbeResult {
    pub fn get(&self, path: &FieldPath) -> Option<&Value> {
        if path.segments.len() == 1 {
            if let PathSegment::Key(key) = &path.segments[0] {
                return self.fields.get(key);
            }
        }
        // for nested paths, convert to dotted key
        let key = path.to_string();
        self.fields.get(&key)
    }
}

// --- serde helpers for Duration ---

mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(d: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(d.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let ms = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(ms))
    }
}

mod option_duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(d: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match d {
            Some(d) => serializer.serialize_some(&(d.as_millis() as u64)),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let ms: Option<u64> = Option::deserialize(deserializer)?;
        Ok(ms.map(Duration::from_millis))
    }
}

/// parse a duration string like "10s", "200ms", "5m"
pub fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    if let Some(rest) = s.strip_suffix("ms") {
        rest.trim().parse::<u64>().ok().map(Duration::from_millis)
    } else if let Some(rest) = s.strip_suffix('s') {
        rest.trim().parse::<u64>().ok().map(Duration::from_secs)
    } else if let Some(rest) = s.strip_suffix('m') {
        rest.trim()
            .parse::<u64>()
            .ok()
            .map(|m| Duration::from_secs(m * 60))
    } else {
        // try as plain seconds
        s.parse::<u64>().ok().map(Duration::from_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("10s"), Some(Duration::from_secs(10)));
        assert_eq!(parse_duration("200ms"), Some(Duration::from_millis(200)));
        assert_eq!(parse_duration("5m"), Some(Duration::from_secs(300)));
    }

    #[test]
    fn test_field_path_simple() {
        let fp = FieldPath::simple("status");
        assert_eq!(fp.segments.len(), 1);
        assert_eq!(fp.to_string(), "status");
    }

    #[test]
    fn test_field_path_dotted() {
        let fp = FieldPath::from_dotted("body.json.status");
        assert_eq!(fp.segments.len(), 3);
        assert_eq!(fp.to_string(), "body.json.status");
    }

    #[test]
    fn test_field_path_with_index() {
        let fp = FieldPath::from_dotted("body.json.users[0].name");
        assert_eq!(fp.segments.len(), 5);
    }

    #[test]
    fn test_probe_result_get() {
        let mut pr = ProbeResult::default();
        pr.fields.insert("status".to_string(), Value::Int(200));
        pr.fields
            .insert("body".to_string(), Value::String("hello".to_string()));

        let v = pr.get(&FieldPath::simple("status"));
        assert!(matches!(v, Some(Value::Int(200))));
    }

    #[test]
    fn test_value_display() {
        assert_eq!(format!("{}", Value::String("hello".into())), "hello");
        assert_eq!(format!("{}", Value::Int(42)), "42");
        assert_eq!(format!("{}", Value::Bool(true)), "true");
    }
}
