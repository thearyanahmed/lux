use super::error::TranspileError;
use super::ir::*;
use std::collections::HashMap;

pub fn transpile(ast: &crate::parser::ast::Ast) -> Result<Blueprint, TranspileError> {
    let mut meta = ProjectMeta::default();
    meta.unlock_mode = "sequential".to_string();
    let mut config = Config::default();
    let mut phases = Vec::new();

    for item in &ast.blueprint.items {
        match item {
            crate::parser::ast::AstItem::Block(block) => match block.block_type.as_str() {
                "config" => config = resolve_config(block)?,
                "phase" => phases.push(resolve_phase(block)?),
                "features" => meta.features = resolve_features(block)?,
                _ => {}
            },
            crate::parser::ast::AstItem::Property(prop) => {
                apply_project_meta_property(&mut meta, prop)?;
            }
            crate::parser::ast::AstItem::Line(_) => {}
        }
    }

    Ok(Blueprint {
        name: ast.blueprint.name.clone(),
        meta,
        config,
        phases,
    })
}

fn resolve_config(block: &crate::parser::ast::Block) -> Result<Config, TranspileError> {
    let mut config = Config::default();
    for item in &block.items {
        if let crate::parser::ast::AstItem::Property(prop) = item {
            match prop.key.as_str() {
                "host" => {
                    config.host = prop
                        .value
                        .as_str()
                        .ok_or_else(|| TranspileError::new("config.host must be a string"))?
                        .to_string();
                }
                "port" => {
                    let n = prop
                        .value
                        .as_i64()
                        .ok_or_else(|| TranspileError::new("config.port must be an integer"))?;
                    config.port = Some(n as u16);
                }
                "timeout" => {
                    let s = prop.value.as_str().ok_or_else(|| {
                        TranspileError::new("config.timeout must be a duration string")
                    })?;
                    config.timeout = parse_duration(s)
                        .ok_or_else(|| TranspileError::new(format!("invalid timeout: {s}")))?;
                }
                _ => {
                    if let Some(v) = prop.value.as_str() {
                        config.env.insert(prop.key.clone(), v.to_string());
                    }
                }
            }
        }
    }
    Ok(config)
}

fn resolve_phase(block: &crate::parser::ast::Block) -> Result<Phase, TranspileError> {
    let name = block
        .name
        .clone()
        .ok_or_else(|| TranspileError::new("phase must have a name"))?;
    let mut depends_on = Vec::new();
    let mut steps = Vec::new();

    for item in &block.items {
        match item {
            crate::parser::ast::AstItem::Block(b) if b.block_type == "step" => {
                steps.push(resolve_step(b)?);
            }
            crate::parser::ast::AstItem::Property(p) if p.key == "depends-on" => {
                if let Some(dep) = p.value.as_str() {
                    depends_on.push(dep.to_string());
                }
            }
            _ => {}
        }
    }

    Ok(Phase {
        name,
        depends_on,
        steps,
    })
}

fn resolve_step(block: &crate::parser::ast::Block) -> Result<Step, TranspileError> {
    let name = block
        .name
        .clone()
        .ok_or_else(|| TranspileError::new("step must have a name"))?;

    let mut meta = StepMeta::default();
    let mut requires = Vec::new();
    let mut timeout = None;
    let mut retry = None;
    let mut inputs = Vec::new();
    let mut probe = None;
    let mut expectations = Vec::new();
    let mut captures = Vec::new();
    let mut headers = HashMap::new();

    for item in &block.items {
        match item {
            crate::parser::ast::AstItem::Property(prop) => match prop.key.as_str() {
                "slug" => meta.slug = prop.value.as_str().map(|s| s.to_string()),
                "description" => meta.description = prop.value.as_str().map(|s| s.to_string()),
                "points" => meta.points = prop.value.as_i64().unwrap_or(0) as u32,
                "scores" => meta.scores = prop.value.as_str().map(|s| s.to_string()),
                "is_free" => meta.is_free = prop.value.as_bool().unwrap_or(false),
                "visibility_level" => {
                    meta.visibility_level = prop.value.as_i64().unwrap_or(0) as u8
                }
                "abandoned_deduction" => {
                    meta.abandoned_deduction = prop.value.as_i64().unwrap_or(0) as u32
                }
                "requires" => {
                    if let Some(s) = prop.value.as_str() {
                        requires.push(s.to_string());
                    }
                }
                "timeout" => {
                    if let Some(s) = prop.value.as_str() {
                        timeout = parse_duration(s);
                    }
                }
                key if key.starts_with("retry") => {
                    retry = parse_retry_from_property(prop)?;
                }
                _ => {}
            },
            crate::parser::ast::AstItem::Block(b) => match b.block_type.as_str() {
                "expect" => {
                    let (exps, caps) = resolve_expect_block(b)?;
                    expectations = exps;
                    captures = caps;
                }
                "input" => {
                    inputs = resolve_input_block(b)?;
                }
                "hints" => {
                    meta.hints = resolve_hints_block(b)?;
                }
                "headers" => {
                    headers = resolve_headers_block(b)?;
                }
                _ => {}
            },
            crate::parser::ast::AstItem::Line(line) => {
                let trimmed = line.content.trim();
                if trimmed.starts_with("probe") {
                    probe = Some(resolve_probe_line(trimmed)?);
                } else if trimmed.starts_with("retry") {
                    retry = parse_retry_from_line(trimmed)?;
                } else if trimmed.starts_with("timeout") {
                    timeout = parse_timeout_from_line(trimmed);
                } else if trimmed.starts_with("requires") {
                    if let Some(var) = trimmed.strip_prefix("requires") {
                        requires.push(var.trim().trim_start_matches(':').trim().to_string());
                    }
                }
            }
        }
    }

    let probe = probe.ok_or_else(|| TranspileError::new(format!("step '{name}' has no probe")))?;

    Ok(Step {
        name,
        meta,
        requires,
        timeout,
        retry,
        inputs,
        probe,
        expectations,
        captures,
        headers,
    })
}

fn resolve_probe_line(line: &str) -> Result<Probe, TranspileError> {
    let rest = line
        .strip_prefix("probe ")
        .ok_or_else(|| TranspileError::new("expected 'probe' keyword"))?
        .trim();

    if rest.starts_with("tcp ") || rest == "tcp" {
        let port_str = rest.strip_prefix("tcp").unwrap_or("").trim();
        let port: u16 = port_str
            .parse()
            .map_err(|_| TranspileError::new(format!("invalid tcp port: '{port_str}'")))?;
        return Ok(Probe::Tcp(TcpProbe { port }));
    }

    if rest.starts_with("udp ") {
        let addr = rest.strip_prefix("udp").unwrap_or("").trim();
        if let Some((host, port_str)) = addr.rsplit_once(':') {
            let port: u16 = port_str
                .parse()
                .map_err(|_| TranspileError::new(format!("invalid udp port: '{port_str}'")))?;
            return Ok(Probe::Udp(UdpProbe {
                host: host.to_string(),
                port,
            }));
        }
        return Err(TranspileError::new(format!(
            "invalid udp address: '{addr}'"
        )));
    }

    if rest.starts_with("http ") {
        return resolve_http_probe(rest.strip_prefix("http ").unwrap_or("").trim());
    }

    if rest.starts_with("docker ") {
        let args = rest.strip_prefix("docker ").unwrap_or("").trim();
        let parts = shell_split(args);
        return Ok(Probe::Exec(ExecProbe {
            command: "docker".to_string(),
            args: parts,
        }));
    }

    if rest.starts_with("exec ") {
        let exec_rest = rest.strip_prefix("exec ").unwrap_or("").trim();
        let parts = shell_split(exec_rest);
        if parts.is_empty() {
            return Err(TranspileError::new("probe exec requires a command"));
        }
        return Ok(Probe::Exec(ExecProbe {
            command: parts[0].clone(),
            args: parts[1..].to_vec(),
        }));
    }

    if rest.starts_with("file ") {
        let path = rest.strip_prefix("file ").unwrap_or("").trim().to_string();
        return Ok(Probe::File(FileProbe { path }));
    }

    if rest.starts_with("process ") {
        let name = rest
            .strip_prefix("process ")
            .unwrap_or("")
            .trim()
            .to_string();
        return Ok(Probe::Process(ProcessProbe { name }));
    }

    Err(TranspileError::new(format!("unknown probe type: '{rest}'")))
}

fn resolve_http_probe(rest: &str) -> Result<Probe, TranspileError> {
    let parts: Vec<&str> = rest.splitn(2, ' ').collect();
    if parts.is_empty() {
        return Err(TranspileError::new("http probe needs METHOD and path"));
    }

    let method = HttpMethod::from_str(parts[0])
        .ok_or_else(|| TranspileError::new(format!("unknown HTTP method: {}", parts[0])))?;

    if parts.len() < 2 {
        return Err(TranspileError::new("http probe needs a path"));
    }

    let rest = parts[1].trim();
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.is_empty() {
        return Err(TranspileError::new("http probe needs a path"));
    }

    let path = tokens[0].to_string();
    let mut body = None;
    let mut mode = HttpMode::Single;
    let mut idx = 1;

    // look for JSON body
    if idx < tokens.len() {
        let maybe_body = tokens[idx..].join(" ");
        if maybe_body.starts_with('{') {
            if let Some(end) = find_matching_brace(&maybe_body) {
                body = Some(maybe_body[..=end].to_string());
                let body_str = &maybe_body[..=end];
                idx += body_str.split_whitespace().count();
            }
        } else if maybe_body.starts_with('"') {
            if let Some(end) = maybe_body[1..].find('"') {
                body = Some(maybe_body[1..=end].to_string());
                idx += 1;
            }
        }
    }

    // mode keywords
    while idx < tokens.len() {
        match tokens[idx] {
            "concurrent" => {
                idx += 1;
                if idx < tokens.len() {
                    if let Ok(n) = tokens[idx].parse::<u32>() {
                        mode = HttpMode::Concurrent { clients: n };
                        idx += 1;
                    }
                }
            }
            "keepalive" => {
                idx += 1;
                if idx < tokens.len() {
                    if let Ok(n) = tokens[idx].parse::<u32>() {
                        mode = HttpMode::Keepalive { requests: n };
                        idx += 1;
                    }
                }
            }
            "pipelined" => {
                idx += 1;
                if idx < tokens.len() {
                    if let Ok(n) = tokens[idx].parse::<u32>() {
                        mode = HttpMode::Pipelined { requests: n };
                        idx += 1;
                    }
                }
            }
            "burst" => {
                idx += 1;
                if idx < tokens.len() {
                    if let Ok(n) = tokens[idx].parse::<u32>() {
                        idx += 1;
                        let mut window_ms = 1000;
                        if idx < tokens.len() && tokens[idx] == "window" {
                            idx += 1;
                            if idx < tokens.len() {
                                if let Some(d) = parse_duration(tokens[idx]) {
                                    window_ms = d.as_millis() as u64;
                                    idx += 1;
                                }
                            }
                        }
                        mode = HttpMode::Burst {
                            count: n,
                            window_ms,
                        };
                    }
                }
            }
            "chunked" => {
                mode = HttpMode::Chunked;
                idx += 1;
            }
            _ => {
                if body.is_none() {
                    body = Some(tokens[idx..].join(" "));
                }
                break;
            }
        }
    }

    Ok(Probe::Http(HttpProbe {
        method,
        path,
        body,
        headers: HashMap::new(),
        mode,
    }))
}

fn resolve_expect_block(
    block: &crate::parser::ast::Block,
) -> Result<(Vec<Expectation>, Vec<Capture>), TranspileError> {
    let mut expectations = Vec::new();
    let mut captures = Vec::new();

    for item in &block.items {
        match item {
            crate::parser::ast::AstItem::Property(prop) => {
                let (field_str, op) = parse_field_and_op(&prop.key);
                let expected = property_value_to_expected(&prop.value, &op)?;
                expectations.push(Expectation {
                    field: FieldPath::from_dotted(&field_str),
                    op,
                    expected,
                });
            }
            crate::parser::ast::AstItem::Line(line) => {
                let trimmed = line.content.trim();
                if trimmed.starts_with("capture") {
                    if let Some(cap) = parse_capture_line(trimmed)? {
                        captures.push(cap);
                    }
                } else if trimmed.starts_with("all ") {
                    let rest = trimmed.strip_prefix("all ").unwrap_or("").trim();
                    if let Some((key, val)) = rest.split_once(':') {
                        let expected = parse_string_to_expected(val.trim())?;
                        expectations.push(Expectation {
                            field: FieldPath::simple(key.trim()),
                            op: Op::All,
                            expected,
                        });
                    }
                }
            }
            crate::parser::ast::AstItem::Block(_) => {}
        }
    }

    Ok((expectations, captures))
}

fn parse_field_and_op(key: &str) -> (String, Op) {
    if let Some(f) = key.strip_suffix(" contains") {
        return (f.to_string(), Op::Contains);
    }
    if let Some(f) = key.strip_suffix(" starts-with") {
        return (f.to_string(), Op::StartsWith);
    }
    if let Some(f) = key.strip_suffix(" matches-file") {
        return (f.to_string(), Op::MatchesFile);
    }
    if let Some(f) = key.strip_suffix(" matches") {
        return (f.to_string(), Op::Matches);
    }
    if let Some(f) = key.strip_suffix(" present") {
        return (f.to_string(), Op::Present);
    }
    if let Some(f) = key.strip_suffix(" absent") {
        return (f.to_string(), Op::Absent);
    }
    if let Some(f) = key.strip_suffix(" >=") {
        return (f.to_string(), Op::Gte);
    }
    if let Some(f) = key.strip_suffix(" <=") {
        return (f.to_string(), Op::Lte);
    }
    if let Some(f) = key.strip_suffix(" >") {
        return (f.to_string(), Op::Gt);
    }
    if let Some(f) = key.strip_suffix(" <") {
        return (f.to_string(), Op::Lt);
    }
    (key.to_string(), Op::Eq)
}

fn property_value_to_expected(
    val: &crate::parser::ast::PropertyValue,
    op: &Op,
) -> Result<ExpectedValue, TranspileError> {
    use crate::parser::ast::PropertyValue;
    match op {
        Op::Present | Op::Absent => Ok(ExpectedValue::Bool(true)),
        Op::Matches => {
            if let Some(s) = val.as_str() {
                let pattern = s
                    .strip_prefix('/')
                    .and_then(|s| s.strip_suffix('/'))
                    .unwrap_or(s);
                Ok(ExpectedValue::Regex(pattern.to_string()))
            } else {
                Err(TranspileError::new(
                    "matches operator requires a regex value",
                ))
            }
        }
        Op::MatchesFile => {
            if let Some(s) = val.as_str() {
                Ok(ExpectedValue::FilePath(s.to_string()))
            } else {
                Err(TranspileError::new("matches-file requires a file path"))
            }
        }
        _ => match val {
            PropertyValue::Int(n) => Ok(ExpectedValue::Int(*n)),
            PropertyValue::Bool(b) => Ok(ExpectedValue::Bool(*b)),
            PropertyValue::String(s) => {
                if s.starts_with('$') {
                    Ok(ExpectedValue::Variable(s.clone()))
                } else if s.ends_with('s') || s.ends_with("ms") {
                    if let Some(d) = parse_duration(s) {
                        Ok(ExpectedValue::DurationMs(d.as_millis() as u64))
                    } else {
                        Ok(ExpectedValue::Str(s.clone()))
                    }
                } else {
                    Ok(ExpectedValue::Str(s.clone()))
                }
            }
            PropertyValue::Float(f) => Ok(ExpectedValue::Int(*f as i64)),
            PropertyValue::MultiLine(s) => Ok(ExpectedValue::Str(s.clone())),
        },
    }
}

fn parse_string_to_expected(val: &str) -> Result<ExpectedValue, TranspileError> {
    if let Ok(n) = val.parse::<i64>() {
        return Ok(ExpectedValue::Int(n));
    }
    if val == "true" {
        return Ok(ExpectedValue::Bool(true));
    }
    if val == "false" {
        return Ok(ExpectedValue::Bool(false));
    }
    Ok(ExpectedValue::Str(
        val.trim_matches('"').trim_matches('\'').to_string(),
    ))
}

fn parse_capture_line(line: &str) -> Result<Option<Capture>, TranspileError> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 4 || parts[0] != "capture" {
        return Ok(None);
    }

    let as_idx = parts
        .iter()
        .position(|&p| p == "as")
        .ok_or_else(|| TranspileError::new(format!("capture line missing 'as': {line}")))?;

    if as_idx + 1 >= parts.len() {
        return Err(TranspileError::new(format!(
            "capture line missing variable: {line}"
        )));
    }

    let field_str = parts[1..as_idx].join(" ");
    let var = parts[as_idx + 1].to_string();

    let field = if field_str.contains(" line ") {
        let base = field_str.split(" line ").next().unwrap_or(&field_str);
        FieldPath::from_dotted(base)
    } else {
        FieldPath::from_dotted(&field_str)
    };

    Ok(Some(Capture {
        field,
        variable: var,
    }))
}

fn resolve_input_block(
    block: &crate::parser::ast::Block,
) -> Result<Vec<InputDecl>, TranspileError> {
    let mut inputs = Vec::new();
    for item in &block.items {
        if let crate::parser::ast::AstItem::Property(prop) = item {
            let value_type = match prop.value.as_str().unwrap_or("string") {
                "number" => InputType::Number,
                "bool" => InputType::Bool,
                other if other.starts_with("number") => InputType::Number,
                _ => InputType::Str,
            };
            let validation = if let Some(s) = prop.value.as_str() {
                if let Some(rest) = s.strip_prefix("string matches ") {
                    Some(InputValidation::Matches(rest.trim_matches('/').to_string()))
                } else {
                    None
                }
            } else {
                None
            };

            inputs.push(InputDecl {
                name: prop.key.clone(),
                value_type,
                validation,
            });
        }
    }
    Ok(inputs)
}

fn resolve_hints_block(block: &crate::parser::ast::Block) -> Result<Vec<Hint>, TranspileError> {
    let mut hints = Vec::new();
    for item in &block.items {
        if let crate::parser::ast::AstItem::Line(line) = item {
            let text = line
                .content
                .trim()
                .trim_start_matches('-')
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            if !text.is_empty() {
                hints.push(Hint {
                    text,
                    unlock_criteria: None,
                    points_deduction: 0,
                });
            }
        }
    }
    Ok(hints)
}

fn resolve_headers_block(
    block: &crate::parser::ast::Block,
) -> Result<HashMap<String, String>, TranspileError> {
    let mut headers = HashMap::new();
    for item in &block.items {
        if let crate::parser::ast::AstItem::Property(prop) = item {
            if let Some(v) = prop.value.as_str() {
                headers.insert(prop.key.clone(), v.to_string());
            }
        }
    }
    Ok(headers)
}

fn resolve_features(block: &crate::parser::ast::Block) -> Result<Vec<Feature>, TranspileError> {
    let mut features = Vec::new();
    for item in &block.items {
        if let crate::parser::ast::AstItem::Line(line) = item {
            let text = line
                .content
                .trim()
                .trim_start_matches('-')
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            let parts: Vec<&str> = text.splitn(3, '|').collect();
            if parts.len() >= 3 {
                features.push(Feature {
                    title: parts[0].trim().into(),
                    description: parts[1].trim().into(),
                    icon: parts[2].trim().into(),
                });
            } else if !text.is_empty() {
                features.push(Feature {
                    title: text.into(),
                    description: String::new(),
                    icon: String::new(),
                });
            }
        }
    }
    Ok(features)
}

fn apply_project_meta_property(
    meta: &mut ProjectMeta,
    prop: &crate::parser::ast::Property,
) -> Result<(), TranspileError> {
    match prop.key.as_str() {
        "slug" => meta.slug = prop.value.as_str().map(|s| s.to_string()),
        "short_description" => meta.short_description = prop.value.as_str().map(|s| s.to_string()),
        "long_description" => meta.long_description = prop.value.as_str().map(|s| s.to_string()),
        "headline" => meta.headline = prop.value.as_str().map(|s| s.to_string()),
        "runner_image" => meta.runner_image = prop.value.as_str().map(|s| s.to_string()),
        "markdown" => meta.markdown = prop.value.as_str().map(|s| s.to_string()),
        "is_published" => meta.is_published = prop.value.as_bool().unwrap_or(false),
        "is_featured" => meta.is_featured = prop.value.as_bool().unwrap_or(false),
        "is_challenge" => meta.is_challenge = prop.value.as_bool().unwrap_or(false),
        "unlock_mode" => meta.unlock_mode = prop.value.as_str().unwrap_or("sequential").to_string(),
        "featured_order" => meta.featured_order = prop.value.as_i64().map(|n| n as u32),
        "published_at" => meta.published_at = prop.value.as_str().map(|s| s.to_string()),
        "related_course_slug" => {
            meta.related_course_slug = prop.value.as_str().map(|s| s.to_string())
        }
        _ => {}
    }
    Ok(())
}

fn parse_retry_from_property(
    prop: &crate::parser::ast::Property,
) -> Result<Option<RetryConfig>, TranspileError> {
    if let Some(n) = prop.value.as_i64() {
        return Ok(Some(RetryConfig {
            max_attempts: n as u32,
            delay: std::time::Duration::from_secs(1),
        }));
    }
    if let Some(s) = prop.value.as_str() {
        return parse_retry_string(s);
    }
    Ok(None)
}

fn parse_retry_from_line(line: &str) -> Result<Option<RetryConfig>, TranspileError> {
    let rest = line
        .strip_prefix("retry")
        .unwrap_or(line)
        .trim()
        .strip_prefix(':')
        .unwrap_or(line)
        .trim();
    parse_retry_string(rest)
}

fn parse_retry_string(s: &str) -> Result<Option<RetryConfig>, TranspileError> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(None);
    }
    let max_attempts = parts[0]
        .parse::<u32>()
        .map_err(|_| TranspileError::new(format!("invalid retry count: {}", parts[0])))?;
    let mut delay = std::time::Duration::from_secs(1);
    if parts.len() >= 3 && parts[1] == "delay" {
        delay = parse_duration(parts[2])
            .ok_or_else(|| TranspileError::new(format!("invalid delay: {}", parts[2])))?;
    }
    Ok(Some(RetryConfig {
        max_attempts,
        delay,
    }))
}

fn parse_timeout_from_line(line: &str) -> Option<std::time::Duration> {
    let rest = line
        .strip_prefix("timeout")
        .unwrap_or(line)
        .trim()
        .strip_prefix(':')
        .unwrap_or(line)
        .trim();
    parse_duration(rest)
}

fn shell_split(input: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    for ch in input.chars() {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            ' ' if !in_single && !in_double => {
                if !current.is_empty() {
                    result.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}

fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;
    for (i, ch) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match ch {
            '\\' => escape = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::grammar::parse as parse_bp;

    fn transpile_str(input: &str) -> Result<Blueprint, TranspileError> {
        let ast = parse_bp(input).map_err(|e| TranspileError::new(e.to_string()))?;
        transpile(&ast)
    }

    #[test]
    fn test_minimal_transpile() {
        let bp = transpile_str(
            r#"
blueprint "Test" {
    config { timeout: 10s }
    phase "basics" {
        step "check port" {
            probe tcp 4221
            expect { connected: true }
        }
    }
}
"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(bp.name, "Test");
        assert_eq!(bp.config.timeout, std::time::Duration::from_secs(10));
        assert_eq!(bp.phases.len(), 1);
        assert!(matches!(
            &bp.phases[0].steps[0].probe,
            Probe::Tcp(TcpProbe { port: 4221 })
        ));
    }

    #[test]
    fn test_http_probe() {
        let bp = transpile_str(
            r#"
blueprint "T" {
    phase "r" {
        step "root" {
            probe http GET /
            expect { status: 200 }
        }
    }
}
"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        match &bp.phases[0].steps[0].probe {
            Probe::Http(p) => {
                assert_eq!(p.method, HttpMethod::GET);
                assert_eq!(p.path, "/");
            }
            other => panic!("expected HttpProbe, got {other:?}"),
        }
    }

    #[test]
    fn test_exec_probe() {
        let bp = transpile_str(
            r#"
blueprint "T" {
    phase "e" {
        step "run" {
            probe exec ./your-docker run echo hello
            expect { stdout: "hello" exit: 0 }
        }
    }
}
"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        match &bp.phases[0].steps[0].probe {
            Probe::Exec(p) => {
                assert_eq!(p.command, "./your-docker");
                assert_eq!(p.args, vec!["run", "echo", "hello"]);
            }
            other => panic!("expected ExecProbe, got {other:?}"),
        }
    }

    #[test]
    fn test_docker_probe_is_exec() {
        let bp = transpile_str(
            r#"
blueprint "T" {
    phase "d" {
        step "inspect" {
            probe docker inspect nginx-1 --format '{{.State.Status}}'
            expect { exit: 0 }
        }
    }
}
"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        match &bp.phases[0].steps[0].probe {
            Probe::Exec(p) => {
                assert_eq!(p.command, "docker");
                assert!(p.args.contains(&"inspect".to_string()));
            }
            other => panic!("expected ExecProbe, got {other:?}"),
        }
    }

    #[test]
    fn test_capture() {
        let bp = transpile_str(
            r#"
blueprint "T" {
    phase "t" {
        step "cap" {
            probe docker inspect nginx-1
            expect {
                exit: 0
                capture stdout as $container_id
            }
        }
    }
}
"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(bp.phases[0].steps[0].captures.len(), 1);
        assert_eq!(bp.phases[0].steps[0].captures[0].variable, "$container_id");
    }

    #[test]
    fn test_input() {
        let bp = transpile_str(
            r#"
blueprint "T" {
    phase "t" {
        step "confirm" {
            input { container-id: string }
            probe docker inspect nginx-1
            expect { stdout: "created" }
        }
    }
}
"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(bp.phases[0].steps[0].inputs.len(), 1);
        assert_eq!(bp.phases[0].steps[0].inputs[0].name, "container-id");
    }

    #[test]
    fn test_depends_on() {
        let bp = transpile_str(
            r#"
blueprint "T" {
    phase "first" {
        step "s" { probe tcp 80 expect { connected: true } }
    }
    phase "second" {
        depends-on: "first"
        step "s" { probe tcp 81 expect { connected: true } }
    }
}
"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(bp.phases[1].depends_on, vec!["first"]);
    }

    #[test]
    fn test_operators() {
        let bp = transpile_str(
            r#"
blueprint "T" {
    phase "t" {
        step "ops" {
            probe http GET /
            expect {
                status: 200
                body contains: "hello"
                body.json.count > 5
                duration < 10s
                header.Server present
            }
        }
    }
}
"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let exps = &bp.phases[0].steps[0].expectations;
        assert!(exps.iter().any(|e| e.op == Op::Eq));
        assert!(exps.iter().any(|e| e.op == Op::Contains));
        assert!(exps.iter().any(|e| e.op == Op::Gt));
        assert!(exps.iter().any(|e| e.op == Op::Lt));
        assert!(exps.iter().any(|e| e.op == Op::Present));
    }

    #[test]
    fn test_container_lifecycle_full() {
        let bp = transpile_str(
            r#"
blueprint "Container Lifecycle" {
    config { timeout: 10s }
    phase "create" {
        step "exists" {
            probe docker inspect nginx-1 --format '{{.State.Status}}'
            expect { exit: 0 stdout: "created" }
        }
        step "capture id" {
            probe docker inspect nginx-1 --format '{{.ID}}'
            expect {
                stdout matches: /^[a-f0-9]{64}$/
                capture stdout as $container_id
            }
        }
        step "confirm id" {
            input { container-id: string }
            probe docker inspect nginx-1 --format '{{.ID}}'
            expect {
                capture stdout as $real_id
                $container-id matches: /^[a-f0-9]{64}$/
                $container-id: $real_id
            }
        }
    }
    phase "running" {
        depends-on: "create"
        step "is running" {
            probe docker inspect nginx-1 --format '{{.State.Status}}'
            expect { stdout: "running" }
        }
    }
    phase "stopped" {
        depends-on: "running"
        step "is exited" {
            probe docker inspect nginx-1 --format '{{.State.Status}}'
            expect { stdout: "exited" }
        }
    }
}
"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(bp.name, "Container Lifecycle");
        assert_eq!(bp.phases.len(), 3);
        assert_eq!(bp.phases[0].steps.len(), 3);
        assert_eq!(bp.phases[0].steps[2].inputs.len(), 1);
        assert_eq!(bp.phases[0].steps[1].captures.len(), 1);
    }

    #[test]
    fn test_project_metadata() {
        let bp = transpile_str(
            r#"
blueprint "HTTP Server" {
    slug: build-your-own-http-server
    is_published: true
    is_featured: true
    runner_image: local|go|rust|c
    features {
        - "Raw TCP Sockets | Start at the bottom | plug"
        - "Protocol Parsing | Parse by hand | code"
    }
    phase "test" {
        step "s1" { probe tcp 4221 expect { connected: true } }
    }
}
"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(bp.meta.slug.as_deref(), Some("build-your-own-http-server"));
        assert!(bp.meta.is_published);
        assert_eq!(bp.meta.features.len(), 2);
        assert_eq!(bp.meta.features[0].title, "Raw TCP Sockets");
    }

    #[test]
    fn test_concurrent_mode() {
        let bp = transpile_str(
            r#"
blueprint "T" {
    phase "t" {
        step "c" {
            probe http GET / concurrent 10
            expect { all status: 200 }
        }
    }
}
"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        match &bp.phases[0].steps[0].probe {
            Probe::Http(p) => assert!(matches!(p.mode, HttpMode::Concurrent { clients: 10 })),
            other => panic!("expected HttpProbe, got {other:?}"),
        }
    }

    #[test]
    fn test_shell_split() {
        let result = shell_split("inspect nginx-1 --format '{{.State.Status}}'");
        assert_eq!(
            result,
            vec!["inspect", "nginx-1", "--format", "{{.State.Status}}"]
        );
    }
}
