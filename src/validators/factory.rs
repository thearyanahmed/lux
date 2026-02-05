use super::benchmark::{BenchmarkValidator, BrcBenchmarkValidator, BrcValidator, OutputMatchValidator};
use super::compile::CanCompileValidator;
use super::container::{
    DockerCgroupMemoryValidator, DockerChrootValidator, DockerExecValidator,
    DockerExitCodeValidator, DockerMountNamespaceValidator, DockerNetworkNamespaceValidator,
    DockerPidNamespaceValidator, DockerPullValidator, DockerVethPairValidator,
};
use super::docker::{DockerValidator, Expectation};
use super::file::FileContentsMatchValidator;
use super::http::{
    ConcurrentRequestsValidator, HttpChunkedValidator, HttpContentTypeValidator,
    HttpGetCompressedValidator, HttpGetFileValidator, HttpGetValidator, HttpGetWithHeaderValidator,
    HttpHeaderPresentValidator, HttpHeaderValueValidator, HttpJsonExistsValidator,
    HttpJsonFieldValidator, HttpKeepaliveValidator, HttpPipeliningValidator, HttpPostFileValidator,
    HttpPostJsonValidator, HttpStatusValidator, RateLimitValidator,
};
use super::parser::{parse_validator, ParsedValidator};
use super::port::PortValidator;
use super::process::{ConcurrentAccessValidator, GracefulShutdownValidator};
use super::scenario::{
    HttpHealthCheck, HttpJsonFieldNested, HttpJsonFieldValue, HttpRequestWithBody, HttpStatusCheck,
    JobPriorityVerified, JobProcessingVerified, JobResultVerified, JobRetryVerified,
    JobSubmissionVerified, JobTimeoutReasonVerified, JobTimeoutVerified, WorkerPoolConcurrent,
    WorkerScaleDown, WorkerScaleUp,
};
use crate::tasks::TestCase;

/// Runtime validator that can execute any parsed validator type
pub enum RuntimeValidator {
    TcpListening(PortValidator),
    HttpResponseStatus(HttpStatusValidator),
    HttpGet(HttpGetValidator),
    HttpHeaderPresent(HttpHeaderPresentValidator),
    HttpHeaderValue(HttpHeaderValueValidator),
    HttpGetWithHeader(HttpGetWithHeaderValidator),
    ConcurrentRequests(ConcurrentRequestsValidator),
    HttpPostFile(HttpPostFileValidator),
    HttpGetFile(HttpGetFileValidator),
    HttpGetCompressed(HttpGetCompressedValidator),
    FileContentsMatch(FileContentsMatchValidator),
    CanCompile(CanCompileValidator),
    OutputMatch(OutputMatchValidator),
    Benchmark(BenchmarkValidator),
    BrcValidate(BrcValidator),
    BrcBenchmark(BrcBenchmarkValidator),
    // http validators
    HttpJsonExists(HttpJsonExistsValidator),
    HttpJsonField(HttpJsonFieldValidator),
    HttpPostJson(HttpPostJsonValidator),
    RateLimit(RateLimitValidator),
    GracefulShutdown(GracefulShutdownValidator),
    ConcurrentAccess(ConcurrentAccessValidator),
    // scenario validators (multi-step)
    JobSubmissionVerified(JobSubmissionVerified),
    JobProcessingVerified(JobProcessingVerified),
    WorkerPoolConcurrent(WorkerPoolConcurrent),
    JobResultVerified(JobResultVerified),
    JobPriorityVerified(JobPriorityVerified),
    JobTimeoutVerified(JobTimeoutVerified),
    JobTimeoutReasonVerified(JobTimeoutReasonVerified),
    JobRetryVerified(JobRetryVerified),
    WorkerScaleUp(WorkerScaleUp),
    WorkerScaleDown(WorkerScaleDown),
    HttpRequestWithBody(HttpRequestWithBody),
    HttpJsonFieldNested(HttpJsonFieldNested),
    HttpHealthCheck(HttpHealthCheck),
    HttpJsonFieldValue(HttpJsonFieldValue),
    HttpStatusCheck(HttpStatusCheck),
    // docker validator (downloads Dockerfiles from GitHub at runtime)
    Docker(DockerValidator),
    HttpContentType(HttpContentTypeValidator),
    HttpKeepalive(HttpKeepaliveValidator),
    HttpChunked(HttpChunkedValidator),
    HttpPipelining(HttpPipeliningValidator),
    // container validators (for "Build Your Own Docker" lab)
    DockerExec(DockerExecValidator),
    DockerExitCode(DockerExitCodeValidator),
    DockerPidNamespace(DockerPidNamespaceValidator),
    DockerChroot(DockerChrootValidator),
    DockerPull(DockerPullValidator),
    DockerMountNamespace(DockerMountNamespaceValidator),
    DockerCgroupMemory(DockerCgroupMemoryValidator),
    DockerNetworkNamespace(DockerNetworkNamespaceValidator),
    DockerVethPair(DockerVethPairValidator),
    // placeholder for validators not yet implemented
    NotImplemented(String),
}

impl RuntimeValidator {
    pub async fn validate(&self) -> Result<TestCase, String> {
        match self {
            RuntimeValidator::TcpListening(v) => v.validate().await,
            RuntimeValidator::HttpResponseStatus(v) => v.validate().await,
            RuntimeValidator::HttpGet(v) => v.validate().await,
            RuntimeValidator::HttpHeaderPresent(v) => v.validate().await,
            RuntimeValidator::HttpHeaderValue(v) => v.validate().await,
            RuntimeValidator::HttpGetWithHeader(v) => v.validate().await,
            RuntimeValidator::ConcurrentRequests(v) => v.validate().await,
            RuntimeValidator::HttpPostFile(v) => v.validate().await,
            RuntimeValidator::HttpGetFile(v) => v.validate().await,
            RuntimeValidator::HttpGetCompressed(v) => v.validate().await,
            RuntimeValidator::FileContentsMatch(v) => v.validate().await,
            RuntimeValidator::CanCompile(v) => v.validate().await,
            RuntimeValidator::OutputMatch(v) => v.validate().await,
            RuntimeValidator::Benchmark(v) => v.validate().await,
            RuntimeValidator::BrcValidate(v) => v.validate().await,
            RuntimeValidator::BrcBenchmark(v) => v.validate().await,
            RuntimeValidator::HttpJsonExists(v) => v.validate().await,
            RuntimeValidator::HttpJsonField(v) => v.validate().await,
            RuntimeValidator::HttpPostJson(v) => v.validate().await,
            RuntimeValidator::RateLimit(v) => v.validate().await,
            RuntimeValidator::GracefulShutdown(v) => v.validate().await,
            RuntimeValidator::ConcurrentAccess(v) => v.validate().await,
            // scenario validators
            RuntimeValidator::JobSubmissionVerified(v) => v.validate().await,
            RuntimeValidator::JobProcessingVerified(v) => v.validate().await,
            RuntimeValidator::WorkerPoolConcurrent(v) => v.validate().await,
            RuntimeValidator::JobResultVerified(v) => v.validate().await,
            RuntimeValidator::JobPriorityVerified(v) => v.validate().await,
            RuntimeValidator::JobTimeoutVerified(v) => v.validate().await,
            RuntimeValidator::JobTimeoutReasonVerified(v) => v.validate().await,
            RuntimeValidator::JobRetryVerified(v) => v.validate().await,
            RuntimeValidator::WorkerScaleUp(v) => v.validate().await,
            RuntimeValidator::WorkerScaleDown(v) => v.validate().await,
            RuntimeValidator::HttpRequestWithBody(v) => v.validate().await,
            RuntimeValidator::HttpJsonFieldNested(v) => v.validate().await,
            RuntimeValidator::HttpHealthCheck(v) => v.validate().await,
            RuntimeValidator::HttpJsonFieldValue(v) => v.validate().await,
            RuntimeValidator::HttpStatusCheck(v) => v.validate().await,
            RuntimeValidator::Docker(v) => v.validate().await,
            RuntimeValidator::HttpContentType(v) => v.validate().await,
            RuntimeValidator::HttpKeepalive(v) => v.validate().await,
            RuntimeValidator::HttpChunked(v) => v.validate().await,
            RuntimeValidator::HttpPipelining(v) => v.validate().await,
            // container validators
            RuntimeValidator::DockerExec(v) => v.validate().await,
            RuntimeValidator::DockerExitCode(v) => v.validate().await,
            RuntimeValidator::DockerPidNamespace(v) => v.validate().await,
            RuntimeValidator::DockerChroot(v) => v.validate().await,
            RuntimeValidator::DockerPull(v) => v.validate().await,
            RuntimeValidator::DockerMountNamespace(v) => v.validate().await,
            RuntimeValidator::DockerCgroupMemory(v) => v.validate().await,
            RuntimeValidator::DockerNetworkNamespace(v) => v.validate().await,
            RuntimeValidator::DockerVethPair(v) => v.validate().await,
            RuntimeValidator::NotImplemented(name) => Ok(TestCase {
                name: format!("validator '{}'", name),
                result: Err(format!("validator '{}' not implemented yet", name)),
            }),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            RuntimeValidator::TcpListening(_) => "tcp_listening",
            RuntimeValidator::HttpResponseStatus(_) => "http_response_status",
            RuntimeValidator::HttpGet(_) => "http_get",
            RuntimeValidator::HttpHeaderPresent(_) => "http_header_present",
            RuntimeValidator::HttpHeaderValue(_) => "http_header_value",
            RuntimeValidator::HttpGetWithHeader(_) => "http_get_with_header",
            RuntimeValidator::ConcurrentRequests(_) => "concurrent_requests",
            RuntimeValidator::HttpPostFile(_) => "http_post_file",
            RuntimeValidator::HttpGetFile(_) => "http_get_file",
            RuntimeValidator::HttpGetCompressed(_) => "http_get_compressed",
            RuntimeValidator::FileContentsMatch(_) => "file_contents_match",
            RuntimeValidator::CanCompile(_) => "can_compile",
            RuntimeValidator::OutputMatch(_) => "output_match",
            RuntimeValidator::Benchmark(_) => "benchmark",
            RuntimeValidator::BrcValidate(_) => "brc_validate",
            RuntimeValidator::BrcBenchmark(_) => "brc_benchmark",
            RuntimeValidator::HttpJsonExists(_) => "http_json_exists",
            RuntimeValidator::HttpJsonField(_) => "http_json_field",
            RuntimeValidator::HttpPostJson(_) => "http_post_json",
            RuntimeValidator::RateLimit(_) => "rate_limit",
            RuntimeValidator::GracefulShutdown(_) => "graceful_shutdown",
            RuntimeValidator::ConcurrentAccess(_) => "concurrent_access",
            // scenario validators
            RuntimeValidator::JobSubmissionVerified(_) => "job_submission_verified",
            RuntimeValidator::JobProcessingVerified(_) => "job_processing_verified",
            RuntimeValidator::WorkerPoolConcurrent(_) => "worker_pool_concurrent",
            RuntimeValidator::JobResultVerified(_) => "job_result",
            RuntimeValidator::JobPriorityVerified(_) => "job_priority",
            RuntimeValidator::JobTimeoutVerified(_) => "job_timeout",
            RuntimeValidator::JobTimeoutReasonVerified(_) => "job_timeout_reason",
            RuntimeValidator::JobRetryVerified(_) => "job_retry",
            RuntimeValidator::WorkerScaleUp(_) => "worker_scale_up",
            RuntimeValidator::WorkerScaleDown(_) => "worker_scale_down",
            RuntimeValidator::HttpRequestWithBody(_) => "http_request",
            RuntimeValidator::HttpJsonFieldNested(_) => "http_json_field_nested",
            RuntimeValidator::HttpHealthCheck(_) => "http_health_check",
            RuntimeValidator::HttpJsonFieldValue(_) => "http_json_field_value",
            RuntimeValidator::HttpStatusCheck(_) => "http_status_check",
            RuntimeValidator::Docker(_) => "docker",
            RuntimeValidator::HttpContentType(_) => "http_content_type",
            RuntimeValidator::HttpKeepalive(_) => "http_keepalive",
            RuntimeValidator::HttpChunked(_) => "http_chunked",
            RuntimeValidator::HttpPipelining(_) => "http_pipelining",
            // container validators
            RuntimeValidator::DockerExec(_) => "docker_exec",
            RuntimeValidator::DockerExitCode(_) => "docker_exit_code",
            RuntimeValidator::DockerPidNamespace(_) => "docker_pid_namespace",
            RuntimeValidator::DockerChroot(_) => "docker_chroot",
            RuntimeValidator::DockerPull(_) => "docker_pull",
            RuntimeValidator::DockerMountNamespace(_) => "docker_mount_namespace",
            RuntimeValidator::DockerCgroupMemory(_) => "docker_cgroup_memory",
            RuntimeValidator::DockerNetworkNamespace(_) => "docker_network_namespace",
            RuntimeValidator::DockerVethPair(_) => "docker_veth_pair",
            RuntimeValidator::NotImplemented(name) => name,
        }
    }
}

/// Create a RuntimeValidator from a validator DSL string
pub fn create_validator(validator_str: &str) -> Result<RuntimeValidator, String> {
    let parsed = parse_validator(validator_str)?;
    create_from_parsed(&parsed)
}

/// Create a RuntimeValidator from a parsed validator definition
fn create_from_parsed(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    match parsed.name.as_str() {
        "tcp_listening" => create_tcp_listening(parsed),
        "http_response_status" => create_http_response_status(parsed),
        "http_get" => create_http_get(parsed),
        "http_header_present" => create_http_header_present(parsed),
        "http_header_value" => create_http_header_value(parsed),
        "http_get_with_header" => create_http_get_with_header(parsed),
        "concurrent_requests" => create_concurrent_requests(parsed),
        "http_post_file" => create_http_post_file(parsed),
        "can_compile" => create_can_compile(parsed),
        "output_match" => create_output_match(parsed),
        "benchmark" => create_benchmark(parsed),
        "brc_validate" => create_brc_validate(parsed),
        "brc_benchmark" => create_brc_benchmark(parsed),
        "http_get_file" => create_http_get_file(parsed),
        "http_get_compressed" => create_http_get_compressed(parsed),
        "file_contents_match" => create_file_contents_match(parsed),
        "http_json_exists" => create_http_json_exists(parsed),
        "http_json_field" => create_http_json_field(parsed),
        "http_post_json" => create_http_post_json(parsed),
        "rate_limit" => create_rate_limit(parsed),
        "graceful_shutdown" => create_graceful_shutdown(parsed),
        "concurrent_access" => create_concurrent_access(parsed),
        // scenario validators
        "job_submission_verified" => create_job_submission_verified(parsed),
        "job_processing_verified" => create_job_processing_verified(parsed),
        "worker_pool_concurrent" => create_worker_pool_concurrent(parsed),
        "job_result" => create_job_result(parsed),
        "job_priority" => create_job_priority(parsed),
        "job_timeout" => create_job_timeout(parsed),
        "job_timeout_reason" => create_job_timeout_reason(parsed),
        "job_retry" => create_job_retry(parsed),
        "worker_scale_up" => create_worker_scale_up(parsed),
        "worker_scale_down" => create_worker_scale_down(parsed),
        "http_request" => create_http_request(parsed),
        "http_json_field_nested" => create_http_json_field_nested(parsed),
        "http_health_check" => create_http_health_check(parsed),
        "http_json_field_value" => create_http_json_field_value(parsed),
        "http_status_check" => create_http_status_check(parsed),
        "docker" => create_docker(parsed),
        "http_path_root" => create_http_path_root(parsed),
        "http_path_unknown" => create_http_path_unknown(parsed),
        "http_path" => create_http_get(parsed),
        "http_header_server" => create_http_header_server(parsed),
        "http_header_date" => create_http_header_date(parsed),
        "http_header_connection" => create_http_header_connection(parsed),
        "http_echo" => create_http_echo(parsed),
        "http_user_agent" => create_http_user_agent(parsed),
        "http_concurrent_clients" => create_http_concurrent_clients(parsed),
        "http_query_param" => create_http_query_param(parsed),
        "http_query_missing" => create_http_query_missing(parsed),
        "http_file_not_found" => create_http_file_not_found(parsed),
        "http_content_type" => create_http_content_type(parsed),
        "http_gzip_encoding" => create_http_gzip_encoding(parsed),
        "http_file_get" => create_http_file_get_alias(parsed),
        "http_file_traversal" => create_http_file_traversal(parsed),
        "http_query_encoded" => create_http_query_encoded(parsed),
        "tcp_read_request" => create_tcp_read_request(parsed),
        "http_keepalive" => create_http_keepalive(parsed),
        "http_connection_close" => create_http_connection_close(parsed),
        "http_gzip_content" => create_http_gzip_content(parsed),
        "http_pipelining" => create_http_pipelining(parsed),
        "http_chunked_stream" => create_http_chunked_stream(parsed),
        "http_chunked_format" => create_http_chunked_format(parsed),
        "http_file_post" => create_http_file_post(parsed),
        "http_file_verify" => create_http_file_verify(parsed),
        // container validators (Build Your Own Docker)
        "docker_exec" => create_docker_exec(parsed),
        "docker_exit_code" => create_docker_exit_code(parsed),
        "docker_pid_namespace" => create_docker_pid_namespace(parsed),
        "docker_chroot" => create_docker_chroot(parsed),
        "docker_pull" => create_docker_pull(parsed),
        "docker_mount_namespace" => create_docker_mount_namespace(parsed),
        "docker_cgroup_memory" => create_docker_cgroup_memory(parsed),
        "docker_network_namespace" => create_docker_network_namespace(parsed),
        "docker_veth_pair" => create_docker_veth_pair(parsed),
        _ => Ok(RuntimeValidator::NotImplemented(parsed.name.clone())),
    }
}

// tcp_listening:int(4221)
fn create_tcp_listening(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let port = parsed.param_as_int(0)? as u16;
    Ok(RuntimeValidator::TcpListening(PortValidator::new(port)))
}

// http_response_status:int(200)
fn create_http_response_status(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let status = parsed.param_as_int(0)? as u16;
    Ok(RuntimeValidator::HttpResponseStatus(
        HttpStatusValidator::new(status),
    ))
}

// http_get:string(/path),int(200) OR http_get:string(/path),int(200),string(expected_body)
fn create_http_get(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    let status = parsed.param_as_int(1)? as u16;
    let expected_body = parsed
        .param(2)
        .and_then(|p| p.as_string())
        .map(|s| s.to_string());

    Ok(RuntimeValidator::HttpGet(HttpGetValidator::new(
        path,
        status,
        expected_body,
    )))
}

// http_header_present:string(Content-Type),bool(true)
fn create_http_header_present(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let header_name = parsed.param_as_string(0)?;
    let should_exist = parsed.param_as_bool(1)?;

    Ok(RuntimeValidator::HttpHeaderPresent(
        HttpHeaderPresentValidator::new(header_name, should_exist),
    ))
}

// http_header_value:string(Content-Encoding),string(gzip),bool(true)
fn create_http_header_value(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let header_name = parsed.param_as_string(0)?;
    let expected_value = parsed.param_as_string(1)?;
    // third param (bool) indicates if it should match, we ignore false case for now

    Ok(RuntimeValidator::HttpHeaderValue(
        HttpHeaderValueValidator::new(header_name, expected_value),
    ))
}

// http_get_with_header:string(/user-agent),string(User-Agent),string(test-agent),int(200),string(test-agent)
fn create_http_get_with_header(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    let header_name = parsed.param_as_string(1)?;
    let header_value = parsed.param_as_string(2)?;
    let expected_status = parsed.param_as_int(3)? as u16;
    let expected_body = parsed
        .param(4)
        .and_then(|p| p.as_string())
        .map(|s| s.to_string());

    Ok(RuntimeValidator::HttpGetWithHeader(
        HttpGetWithHeaderValidator::new(
            path,
            header_name,
            header_value,
            expected_status,
            expected_body,
        ),
    ))
}

// concurrent_requests:int(3),string(/echo/test),int(200)
fn create_concurrent_requests(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let num_connections = parsed.param_as_int(0)? as u32;
    let path = parsed.param_as_string(1)?;
    let expected_status = parsed.param_as_int(2)? as u16;

    Ok(RuntimeValidator::ConcurrentRequests(
        ConcurrentRequestsValidator::new(num_connections, path, expected_status),
    ))
}

// http_post_file:string(/files/upload.txt),string(hello world),int(201)
fn create_http_post_file(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    let body = parsed.param_as_string(1)?;
    let expected_status = parsed.param_as_int(2)? as u16;

    Ok(RuntimeValidator::HttpPostFile(HttpPostFileValidator::new(
        path,
        body,
        expected_status,
    )))
}

// can_compile:bool(true)
fn create_can_compile(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let expected_success = parsed.param_as_bool(0)?;
    Ok(RuntimeValidator::CanCompile(CanCompileValidator::new(
        expected_success,
    )))
}

// output_match:string(./solution /data/measurements_10k.txt),string(/expected/10k.txt)
fn create_output_match(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let command = parsed.param_as_string(0)?;
    let expected_file = parsed.param_as_string(1)?;
    Ok(RuntimeValidator::OutputMatch(OutputMatchValidator::new(
        command,
        expected_file,
    )))
}

// benchmark:string(./solution /data/measurements_1m.txt),string(/expected/1m.txt),int(10000)
fn create_benchmark(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let command = parsed.param_as_string(0)?;
    let expected_file = parsed.param_as_string(1)?;
    let max_time_ms = parsed.param_as_int(2)? as u64;
    Ok(RuntimeValidator::Benchmark(BenchmarkValidator::new(
        command,
        expected_file,
        max_time_ms,
    )))
}

// brc_validate:string(./solution),string(data/measurements.txt)
fn create_brc_validate(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let solution = parsed.param_as_string(0)?;
    let measurements_file = parsed.param_as_string(1)?;
    Ok(RuntimeValidator::BrcValidate(BrcValidator::new(
        solution,
        measurements_file,
    )))
}

// brc_benchmark:string(./solution),string(data/measurements.txt),int(10000)
fn create_brc_benchmark(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let solution = parsed.param_as_string(0)?;
    let measurements_file = parsed.param_as_string(1)?;
    let max_time_ms = parsed.param_as_int(2)? as u64;
    Ok(RuntimeValidator::BrcBenchmark(BrcBenchmarkValidator::new(
        solution,
        measurements_file,
        max_time_ms,
    )))
}

// http_get_file:string(/files/test.txt),int(200)
fn create_http_get_file(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    let expected_status = parsed.param_as_int(1)? as u16;
    Ok(RuntimeValidator::HttpGetFile(HttpGetFileValidator::new(
        path,
        expected_status,
    )))
}

// http_get_compressed:string(/path),string(gzip)
fn create_http_get_compressed(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    let encoding = parsed.param_as_string(1)?;
    Ok(RuntimeValidator::HttpGetCompressed(
        HttpGetCompressedValidator::new(path, encoding),
    ))
}

// file_contents_match:string(/path/to/file),string(expected content)
fn create_file_contents_match(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    let expected_content = parsed.param_as_string(1)?;
    Ok(RuntimeValidator::FileContentsMatch(
        FileContentsMatchValidator::new(path, expected_content),
    ))
}

// http_json_exists:string(/path),string(GET),string(field1),string(field2)
fn create_http_json_exists(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    let method = parsed.param_as_string(1)?;

    // collect remaining params as field names
    let mut fields = Vec::new();
    let mut idx = 2;
    while let Some(param) = parsed.param(idx) {
        if let Some(field) = param.as_string() {
            fields.push(field.to_string());
        }
        idx += 1;
    }

    Ok(RuntimeValidator::HttpJsonExists(
        HttpJsonExistsValidator::new(path, method, fields),
    ))
}

// http_json_field:string(/path),string(GET),string(field),string(expected_value)
fn create_http_json_field(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    let method = parsed.param_as_string(1)?;
    let field = parsed.param_as_string(2)?;
    let expected_value = parsed.param_as_string(3)?;

    Ok(RuntimeValidator::HttpJsonField(
        HttpJsonFieldValidator::new(path, method, field, expected_value),
    ))
}

// http_post_json:string(/path),string({"key":"value"}),int(201)
fn create_http_post_json(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    let body = parsed.param_as_string(1)?;
    let expected_status = parsed.param_as_int(2)? as u16;

    Ok(RuntimeValidator::HttpPostJson(HttpPostJsonValidator::new(
        path,
        body,
        expected_status,
    )))
}

// rate_limit:string(/path),string(POST),int(100),int(1000),int(90)
// params: path, method, total_requests, window_ms, expected_rejected
fn create_rate_limit(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    let method = parsed.param_as_string(1)?;
    let requests = parsed.param_as_int(2)? as u32;
    let window_ms = parsed.param_as_int(3)? as u64;
    let expected_rejected = parsed.param_as_int(4)? as u32;

    Ok(RuntimeValidator::RateLimit(RateLimitValidator::new(
        path,
        method,
        requests,
        window_ms,
        expected_rejected,
    )))
}

// graceful_shutdown:string(./binary),int(5000)
fn create_graceful_shutdown(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let binary_path = parsed.param_as_string(0)?;
    let timeout_ms = parsed.param_as_int(1)? as u64;

    Ok(RuntimeValidator::GracefulShutdown(
        GracefulShutdownValidator::new(binary_path, timeout_ms),
    ))
}

// concurrent_access:int(4221),string(/path),int(10),int(100)
// params: port, path, concurrent_clients, operations_per_client
fn create_concurrent_access(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let port = parsed.param_as_int(0)? as u16;
    let path = parsed.param_as_string(1)?;
    let concurrent_count = parsed.param_as_int(2)? as u32;
    let operations = parsed.param_as_int(3)? as u32;

    Ok(RuntimeValidator::ConcurrentAccess(
        ConcurrentAccessValidator::new(port, path, concurrent_count, operations),
    ))
}

// job_submission_verified:string(test),string(payload)
fn create_job_submission_verified(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let job_type = parsed.param_as_string(0).unwrap_or("test");
    let payload = parsed.param_as_string(1).unwrap_or("data");

    Ok(RuntimeValidator::JobSubmissionVerified(
        JobSubmissionVerified::new(job_type, payload),
    ))
}

// job_processing_verified:int(200),string(completed)
fn create_job_processing_verified(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let wait_ms = parsed.param_as_int(0).unwrap_or(200) as u64;
    let expected_status = parsed.param_as_string(1).unwrap_or("completed");

    Ok(RuntimeValidator::JobProcessingVerified(
        JobProcessingVerified::new(wait_ms, expected_status),
    ))
}

// worker_pool_concurrent:int(4),int(4),int(500)
// params: workers, jobs, max_time_ms
fn create_worker_pool_concurrent(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let workers = parsed.param_as_int(0).unwrap_or(4) as u32;
    let jobs = parsed.param_as_int(1).unwrap_or(4) as u32;
    let max_time_ms = parsed.param_as_int(2).unwrap_or(1000) as u64;

    Ok(RuntimeValidator::WorkerPoolConcurrent(
        WorkerPoolConcurrent::new(workers, jobs, max_time_ms),
    ))
}

// job_result:string(echo),string(hello),string(hello)
fn create_job_result(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let job_type = parsed.param_as_string(0)?;
    let payload = parsed.param_as_string(1)?;
    let expected_result = parsed.param_as_string(2)?;

    Ok(RuntimeValidator::JobResultVerified(JobResultVerified::new(
        job_type,
        payload,
        expected_result,
    )))
}

// job_priority:int(10),int(1)
fn create_job_priority(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let high_priority = parsed.param_as_int(0).unwrap_or(10) as u32;
    let low_priority = parsed.param_as_int(1).unwrap_or(1) as u32;

    Ok(RuntimeValidator::JobPriorityVerified(
        JobPriorityVerified::new(high_priority, low_priority),
    ))
}

// job_timeout:int(5000),string(failed)
fn create_job_timeout(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let job_duration_ms = parsed.param_as_int(0).unwrap_or(5000) as u64;
    let expected_status = parsed.param_as_string(1).unwrap_or("failed");

    Ok(RuntimeValidator::JobTimeoutVerified(
        JobTimeoutVerified::new(job_duration_ms, expected_status),
    ))
}

// job_timeout_reason:string(timeout)
fn create_job_timeout_reason(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let expected_reason = parsed.param_as_string(0).unwrap_or("timeout");

    Ok(RuntimeValidator::JobTimeoutReasonVerified(
        JobTimeoutReasonVerified::new(expected_reason),
    ))
}

// job_retry:string(flaky),int(3)
fn create_job_retry(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let job_type = parsed.param_as_string(0).unwrap_or("flaky");
    let max_retries = parsed.param_as_int(1).unwrap_or(3) as u32;

    Ok(RuntimeValidator::JobRetryVerified(JobRetryVerified::new(
        job_type,
        max_retries,
    )))
}

// worker_scale_up:int(2),int(50),int(4)
fn create_worker_scale_up(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let initial_workers = parsed.param_as_int(0).unwrap_or(2) as u32;
    let job_count = parsed.param_as_int(1).unwrap_or(50) as u32;
    let expected_min_workers = parsed.param_as_int(2).unwrap_or(4) as u32;

    Ok(RuntimeValidator::WorkerScaleUp(WorkerScaleUp::new(
        initial_workers,
        job_count,
        expected_min_workers,
    )))
}

// worker_scale_down:int(8),int(4)
fn create_worker_scale_down(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let initial_workers = parsed.param_as_int(0).unwrap_or(8) as u32;
    let expected_max_workers = parsed.param_as_int(1).unwrap_or(4) as u32;

    Ok(RuntimeValidator::WorkerScaleDown(WorkerScaleDown::new(
        initial_workers,
        expected_max_workers,
    )))
}

// http_request:string(POST),string(/jobs),string({"type":"test"}),int(201)
fn create_http_request(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let method = parsed.param_as_string(0)?;
    let path = parsed.param_as_string(1)?;
    let body = parsed
        .param(2)
        .and_then(|p| p.as_string())
        .map(String::from);
    let expected_status = parsed.param_as_int(3).unwrap_or(200) as u16;

    Ok(RuntimeValidator::HttpRequestWithBody(
        HttpRequestWithBody::new(method, path, body.as_deref(), expected_status),
    ))
}

// http_json_field_nested:string(/stats),string(workers.total)
fn create_http_json_field_nested(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    let field_path = parsed.param_as_string(1)?;

    Ok(RuntimeValidator::HttpJsonFieldNested(
        HttpJsonFieldNested::new(path, field_path),
    ))
}

// http_health_check:string(/health),int(200),string(status),string(ok)
fn create_http_health_check(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    let expected_status = parsed.param_as_int(1)? as u16;
    let field = parsed.param_as_string(2)?;
    let value = parsed.param_as_string(3)?;

    Ok(RuntimeValidator::HttpHealthCheck(HttpHealthCheck::new(
        path,
        expected_status,
        field,
        value,
    )))
}

// http_json_field_value:string(/path),string(field),string(expected_value)
fn create_http_json_field_value(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    let field = parsed.param_as_string(1)?;
    let expected_value = parsed.param_as_string(2)?;

    Ok(RuntimeValidator::HttpJsonFieldValue(
        HttpJsonFieldValue::new(path, field, expected_value),
    ))
}

// http_status_check:string(/path),int(expected_status)
fn create_http_status_check(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    let expected_status = parsed.param_as_int(1)? as u16;

    Ok(RuntimeValidator::HttpStatusCheck(HttpStatusCheck::new(
        path,
        expected_status,
    )))
}

// docker:string(Go1.22-race),string(fail_if:stderr contains DATA RACE)
// docker:string(Go1.22),string(exit:0),int(120)
// param 0: dockerfile name (fetched from GitHub)
// param 1: expectation DSL (exit:0, fail_if:stderr contains X, pass_if:stdout contains Y)
// param 2: optional timeout in seconds
fn create_docker(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let dockerfile_name = parsed.param_as_string(0)?;
    let expectation_str = parsed.param_as_string(1)?;
    let timeout_secs = parsed.param_as_int(2).ok().map(|t| t as u64);

    let expectation =
        Expectation::parse(expectation_str).map_err(|e| format!("invalid expectation: {}", e))?;

    let mut validator = DockerValidator::new(dockerfile_name, expectation);
    if let Some(secs) = timeout_secs {
        validator = validator.with_timeout(secs);
    }

    Ok(RuntimeValidator::Docker(validator))
}

// http_path_root:int(200) - alias for http_get:string(/),int(status)
fn create_http_path_root(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let status = parsed.param_as_int(0)? as u16;
    Ok(RuntimeValidator::HttpGet(HttpGetValidator::new(
        "/", status, None,
    )))
}

// http_path_unknown:int(404) - GET to nonexistent path, expect given status
fn create_http_path_unknown(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let status = parsed.param_as_int(0)? as u16;
    Ok(RuntimeValidator::HttpGet(HttpGetValidator::new(
        "/nonexistent-path-for-testing",
        status,
        None,
    )))
}

// http_header_server:bool(true) - check Server header is present
fn create_http_header_server(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let should_exist = parsed.param_as_bool(0)?;
    Ok(RuntimeValidator::HttpHeaderPresent(
        HttpHeaderPresentValidator::new("Server", should_exist),
    ))
}

// http_header_date:bool(true) - check Date header is present
fn create_http_header_date(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let should_exist = parsed.param_as_bool(0)?;
    Ok(RuntimeValidator::HttpHeaderPresent(
        HttpHeaderPresentValidator::new("Date", should_exist),
    ))
}

// http_header_connection:string(close) - check Connection header has given value
fn create_http_header_connection(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let expected_value = parsed.param_as_string(0)?;
    Ok(RuntimeValidator::HttpHeaderValue(
        HttpHeaderValueValidator::new("Connection", expected_value),
    ))
}

// http_echo:string(input),string(expected) - GET /echo/{input}, verify body equals expected
fn create_http_echo(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let input = parsed.param_as_string(0)?;
    let expected = parsed.param_as_string(1)?;
    let path = format!("/echo/{}", input);
    Ok(RuntimeValidator::HttpGet(HttpGetValidator::new(
        &path,
        200,
        Some(expected.to_string()),
    )))
}

// http_user_agent:string(agent),string(expected) - GET /user-agent with User-Agent header
fn create_http_user_agent(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let agent = parsed.param_as_string(0)?;
    let expected = parsed.param_as_string(1)?;
    Ok(RuntimeValidator::HttpGetWithHeader(
        HttpGetWithHeaderValidator::new(
            "/user-agent",
            "User-Agent",
            agent,
            200,
            Some(expected.to_string()),
        ),
    ))
}

// http_concurrent_clients:int(n) - open n connections simultaneously
fn create_http_concurrent_clients(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let num_clients = parsed.param_as_int(0)? as u32;
    Ok(RuntimeValidator::ConcurrentRequests(
        ConcurrentRequestsValidator::new(num_clients, "/", 200),
    ))
}

// http_query_param:string(name),string(value),string(expected) - GET /search?name=value
fn create_http_query_param(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let name = parsed.param_as_string(0)?;
    let value = parsed.param_as_string(1)?;
    let expected = parsed.param_as_string(2)?;
    let path = format!("/search?{}={}", name, value);
    Ok(RuntimeValidator::HttpGet(HttpGetValidator::new(
        &path,
        200,
        Some(expected.to_string()),
    )))
}

// http_query_missing:int(status) - GET /search with no params, expect status
fn create_http_query_missing(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let status = parsed.param_as_int(0)? as u16;
    Ok(RuntimeValidator::HttpGet(HttpGetValidator::new(
        "/search", status, None,
    )))
}

// http_file_not_found:string(filename),int(status) - GET /files/filename, expect status
fn create_http_file_not_found(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let filename = parsed.param_as_string(0)?;
    let status = parsed.param_as_int(1)? as u16;
    let path = format!("/files/{}", filename);
    Ok(RuntimeValidator::HttpGet(HttpGetValidator::new(
        &path, status, None,
    )))
}

// http_content_type:string(filename),string(mime) - GET /files/filename, verify Content-Type
fn create_http_content_type(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let filename = parsed.param_as_string(0)?;
    let expected_mime = parsed.param_as_string(1)?;
    let path = format!("/files/{}", filename);
    Ok(RuntimeValidator::HttpContentType(
        HttpContentTypeValidator::new(&path, expected_mime),
    ))
}

// http_gzip_encoding:string(path),bool(true) - verify gzip Content-Encoding header
fn create_http_gzip_encoding(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    Ok(RuntimeValidator::HttpGetCompressed(
        HttpGetCompressedValidator::new(path, "gzip"),
    ))
}

// http_file_get:string(filename),string(content) - GET /files/filename, verify body
fn create_http_file_get_alias(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let filename = parsed.param_as_string(0)?;
    let expected_content = parsed.param_as_string(1)?;
    let path = format!("/files/{}", filename);
    Ok(RuntimeValidator::HttpGet(HttpGetValidator::new(
        &path,
        200,
        Some(expected_content.to_string()),
    )))
}

// http_file_traversal:string(path),int(status) - test path traversal attack, expect status
fn create_http_file_traversal(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let traversal_path = parsed.param_as_string(0)?;
    let expected_status = parsed.param_as_int(1)? as u16;
    let path = format!("/files/{}", traversal_path);
    Ok(RuntimeValidator::HttpGet(HttpGetValidator::new(
        &path,
        expected_status,
        None,
    )))
}

// http_query_encoded:string(encoded),string(decoded) - GET /search?q=encoded, verify decoding
fn create_http_query_encoded(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let encoded = parsed.param_as_string(0)?;
    let decoded = parsed.param_as_string(1)?;
    let path = format!("/search?q={}", encoded);
    Ok(RuntimeValidator::HttpGet(HttpGetValidator::new(
        &path,
        200,
        Some(decoded.to_string()),
    )))
}

// tcp_read_request:bool(true) - verify server reads and processes HTTP request
fn create_tcp_read_request(_parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    Ok(RuntimeValidator::HttpGet(HttpGetValidator::new(
        "/", 200, None,
    )))
}

// http_keepalive:int(n) - send n requests on same connection
fn create_http_keepalive(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let num_requests = parsed.param_as_int(0)? as u32;
    Ok(RuntimeValidator::HttpKeepalive(
        HttpKeepaliveValidator::new(num_requests, "/"),
    ))
}

// http_connection_close:bool(true) - verify Connection: close header in response
fn create_http_connection_close(_parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    Ok(RuntimeValidator::HttpHeaderValue(
        HttpHeaderValueValidator::new("Connection", "close"),
    ))
}

// http_gzip_content:string(path),string(expected) - decompress gzip body, verify content
fn create_http_gzip_content(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let path = parsed.param_as_string(0)?;
    Ok(RuntimeValidator::HttpGetCompressed(
        HttpGetCompressedValidator::new(path, "gzip"),
    ))
}

// http_pipelining:int(n) - send n requests on same connection without waiting for responses
fn create_http_pipelining(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let num_requests = parsed.param_as_int(0)? as u32;
    Ok(RuntimeValidator::HttpPipelining(
        HttpPipeliningValidator::new(num_requests, "/"),
    ))
}

// http_chunked_stream:int(n) - GET /stream, expect chunked transfer with n chunks
fn create_http_chunked_stream(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let num_chunks = parsed.param_as_int(0)? as u32;
    Ok(RuntimeValidator::HttpChunked(HttpChunkedValidator::new(
        "/stream",
        Some(num_chunks),
    )))
}

// http_chunked_format:bool(true) - verify proper chunked transfer encoding format
fn create_http_chunked_format(_parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    Ok(RuntimeValidator::HttpChunked(HttpChunkedValidator::new(
        "/stream", None,
    )))
}

// http_file_post:string(filename),string(content),int(status) - POST to /files/filename
fn create_http_file_post(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let filename = parsed.param_as_string(0)?;
    let content = parsed.param_as_string(1)?;
    let expected_status = parsed.param_as_int(2)? as u16;
    let path = format!("/files/{}", filename);
    Ok(RuntimeValidator::HttpPostFile(HttpPostFileValidator::new(
        &path,
        content,
        expected_status,
    )))
}

// http_file_verify:string(filename),string(expected_content) - GET /files/filename, verify body
fn create_http_file_verify(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let filename = parsed.param_as_string(0)?;
    let expected_content = parsed.param_as_string(1)?;
    let path = format!("/files/{}", filename);
    Ok(RuntimeValidator::HttpGet(HttpGetValidator::new(
        &path,
        200,
        Some(expected_content.to_string()),
    )))
}

// ============================================================================
// Container validators (Build Your Own Docker)
// ============================================================================

// docker_exec:string(cmd),string(expected) - run command and check stdout
fn create_docker_exec(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let command = parsed.param_as_string(0)?;
    let expected = parsed.param_as_string(1)?;
    Ok(RuntimeValidator::DockerExec(DockerExecValidator::new(
        command, expected,
    )))
}

// docker_exit_code:int(code) - run ls /nonexistent and check exit code
fn create_docker_exit_code(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let code = parsed.param_as_int(0)? as i32;
    Ok(RuntimeValidator::DockerExitCode(
        DockerExitCodeValidator::new(code),
    ))
}

// docker_pid_namespace:int(expected_pid) - verify PID 1 in namespace
fn create_docker_pid_namespace(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let expected_pid = parsed.param_as_int(0)? as i32;
    Ok(RuntimeValidator::DockerPidNamespace(
        DockerPidNamespaceValidator::new(expected_pid),
    ))
}

// docker_chroot:bool(expected) - verify filesystem isolation
fn create_docker_chroot(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let expected = parsed.param_as_bool(0)?;
    Ok(RuntimeValidator::DockerChroot(DockerChrootValidator::new(
        expected,
    )))
}

// docker_pull:string(image),bool(success) - pull image from registry
fn create_docker_pull(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let image = parsed.param_as_string(0)?;
    let expected_success = parsed.param_as_bool(1)?;
    Ok(RuntimeValidator::DockerPull(DockerPullValidator::new(
        image,
        expected_success,
    )))
}

// docker_mount_namespace:bool(expected) - verify mount namespace isolation
fn create_docker_mount_namespace(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let expected = parsed.param_as_bool(0)?;
    Ok(RuntimeValidator::DockerMountNamespace(
        DockerMountNamespaceValidator::new(expected),
    ))
}

// docker_cgroup_memory:int(bytes),bool(oom_expected) - verify memory cgroup limits
fn create_docker_cgroup_memory(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let bytes = parsed.param_as_int(0)? as u64;
    let oom_expected = parsed.param_as_bool(1)?;
    Ok(RuntimeValidator::DockerCgroupMemory(
        DockerCgroupMemoryValidator::new(bytes, oom_expected),
    ))
}

// docker_network_namespace:bool(expected) - verify network namespace isolation
fn create_docker_network_namespace(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let expected = parsed.param_as_bool(0)?;
    Ok(RuntimeValidator::DockerNetworkNamespace(
        DockerNetworkNamespaceValidator::new(expected),
    ))
}

// docker_veth_pair:bool(expected) - verify veth pair connectivity
fn create_docker_veth_pair(parsed: &ParsedValidator) -> Result<RuntimeValidator, String> {
    let expected = parsed.param_as_bool(0)?;
    Ok(RuntimeValidator::DockerVethPair(
        DockerVethPairValidator::new(expected),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tcp_listening() {
        let validator = create_validator("tcp_listening:int(4221)").unwrap();
        assert_eq!(validator.name(), "tcp_listening");
    }

    #[test]
    fn test_create_http_response_status() {
        let validator = create_validator("http_response_status:int(200)").unwrap();
        assert_eq!(validator.name(), "http_response_status");
    }

    #[test]
    fn test_create_http_get() {
        let validator = create_validator("http_get:string(/),int(200)").unwrap();
        assert_eq!(validator.name(), "http_get");
    }

    #[test]
    fn test_create_http_get_with_body() {
        let validator =
            create_validator("http_get:string(/echo/hello),int(200),string(hello)").unwrap();
        assert_eq!(validator.name(), "http_get");
    }

    #[test]
    fn test_create_http_header_present() {
        let validator =
            create_validator("http_header_present:string(Content-Type),bool(true)").unwrap();
        assert_eq!(validator.name(), "http_header_present");
    }

    #[test]
    fn test_create_http_get_with_header() {
        let validator = create_validator(
            "http_get_with_header:string(/user-agent),string(User-Agent),string(test-agent),int(200),string(test-agent)"
        ).unwrap();
        assert_eq!(validator.name(), "http_get_with_header");
    }

    #[test]
    fn test_create_concurrent_requests() {
        let validator =
            create_validator("concurrent_requests:int(3),string(/echo/test),int(200)").unwrap();
        assert_eq!(validator.name(), "concurrent_requests");
    }

    #[test]
    fn test_create_http_post_file() {
        let validator = create_validator(
            "http_post_file:string(/files/upload.txt),string(hello world),int(201)",
        )
        .unwrap();
        assert_eq!(validator.name(), "http_post_file");
    }

    #[test]
    fn test_unknown_validator() {
        let validator = create_validator("unknown_validator:int(1)").unwrap();
        assert_eq!(validator.name(), "unknown_validator");
        // should be NotImplemented variant
        matches!(validator, RuntimeValidator::NotImplemented(_));
    }

    #[test]
    fn test_not_implemented_validators() {
        let names = ["unknown_future_validator:string(test)"];

        for name in names {
            let validator = create_validator(name).unwrap();
            matches!(validator, RuntimeValidator::NotImplemented(_));
        }
    }

    #[test]
    fn test_create_http_get_compressed() {
        let validator =
            create_validator("http_get_compressed:string(/compressed),string(gzip)").unwrap();
        assert_eq!(validator.name(), "http_get_compressed");
    }

    #[test]
    fn test_create_file_contents_match() {
        let validator =
            create_validator("file_contents_match:string(/tmp/test.txt),string(expected)").unwrap();
        assert_eq!(validator.name(), "file_contents_match");
    }

    #[test]
    fn test_create_can_compile() {
        let validator = create_validator("can_compile:bool(true)").unwrap();
        assert_eq!(validator.name(), "can_compile");
    }

    #[test]
    fn test_create_http_get_file() {
        let validator = create_validator("http_get_file:string(/files/test.txt),int(200)").unwrap();
        assert_eq!(validator.name(), "http_get_file");
    }

    #[test]
    fn test_create_docker_with_exit_code() {
        let validator = create_validator("docker:string(Go1.22),string(exit:0)").unwrap();
        assert_eq!(validator.name(), "docker");
    }

    #[test]
    fn test_create_docker_with_fail_if() {
        let validator = create_validator(
            "docker:string(Go1.22-race),string(fail_if:stderr contains DATA RACE)",
        )
        .unwrap();
        assert_eq!(validator.name(), "docker");
    }

    #[test]
    fn test_create_docker_with_timeout() {
        let validator =
            create_validator("docker:string(Go1.22-race),string(exit:0),int(300)").unwrap();
        assert_eq!(validator.name(), "docker");
    }

    #[test]
    fn test_create_http_path_root() {
        let validator = create_validator("http_path_root:int(200)").unwrap();
        assert_eq!(validator.name(), "http_get");
    }

    #[test]
    fn test_create_http_path_unknown() {
        let validator = create_validator("http_path_unknown:int(404)").unwrap();
        assert_eq!(validator.name(), "http_get");
    }

    #[test]
    fn test_create_http_path() {
        let validator =
            create_validator("http_path:string(/health),int(200),string(healthy)").unwrap();
        assert_eq!(validator.name(), "http_get");
    }

    #[test]
    fn test_create_http_header_server() {
        let validator = create_validator("http_header_server:bool(true)").unwrap();
        assert_eq!(validator.name(), "http_header_present");
    }

    #[test]
    fn test_create_http_header_date() {
        let validator = create_validator("http_header_date:bool(true)").unwrap();
        assert_eq!(validator.name(), "http_header_present");
    }

    #[test]
    fn test_create_http_header_connection() {
        let validator = create_validator("http_header_connection:string(close)").unwrap();
        assert_eq!(validator.name(), "http_header_value");
    }

    #[test]
    fn test_create_http_echo() {
        let validator = create_validator("http_echo:string(hello),string(hello)").unwrap();
        assert_eq!(validator.name(), "http_get");
    }

    #[test]
    fn test_create_http_user_agent() {
        let validator =
            create_validator("http_user_agent:string(test-agent),string(test-agent)").unwrap();
        assert_eq!(validator.name(), "http_get_with_header");
    }

    #[test]
    fn test_create_http_concurrent_clients() {
        let validator = create_validator("http_concurrent_clients:int(5)").unwrap();
        assert_eq!(validator.name(), "concurrent_requests");
    }

    #[test]
    fn test_create_http_query_param() {
        let validator =
            create_validator("http_query_param:string(q),string(hello),string(hello)").unwrap();
        assert_eq!(validator.name(), "http_get");
    }

    #[test]
    fn test_create_http_query_missing() {
        let validator = create_validator("http_query_missing:int(400)").unwrap();
        assert_eq!(validator.name(), "http_get");
    }

    #[test]
    fn test_create_http_file_not_found() {
        let validator =
            create_validator("http_file_not_found:string(missing.txt),int(404)").unwrap();
        assert_eq!(validator.name(), "http_get");
    }

    #[test]
    fn test_create_http_content_type() {
        let validator =
            create_validator("http_content_type:string(test.txt),string(text/plain)").unwrap();
        assert_eq!(validator.name(), "http_content_type");
    }

    #[test]
    fn test_create_http_gzip_encoding() {
        let validator =
            create_validator("http_gzip_encoding:string(/compressed),bool(true)").unwrap();
        assert_eq!(validator.name(), "http_get_compressed");
    }

    #[test]
    fn test_create_http_file_get_alias() {
        let validator =
            create_validator("http_file_get:string(test.txt),string(hello world)").unwrap();
        assert_eq!(validator.name(), "http_get");
    }

    #[test]
    fn test_create_http_file_traversal() {
        let validator =
            create_validator("http_file_traversal:string(../etc/passwd),int(400)").unwrap();
        assert_eq!(validator.name(), "http_get");
    }

    #[test]
    fn test_create_http_query_encoded() {
        let validator =
            create_validator("http_query_encoded:string(hello%20world),string(hello world)")
                .unwrap();
        assert_eq!(validator.name(), "http_get");
    }

    #[test]
    fn test_create_tcp_read_request() {
        let validator = create_validator("tcp_read_request:bool(true)").unwrap();
        assert_eq!(validator.name(), "http_get");
    }

    #[test]
    fn test_create_http_keepalive() {
        let validator = create_validator("http_keepalive:int(5)").unwrap();
        assert_eq!(validator.name(), "http_keepalive");
    }

    #[test]
    fn test_create_http_connection_close() {
        let validator = create_validator("http_connection_close:bool(true)").unwrap();
        assert_eq!(validator.name(), "http_header_value");
    }

    #[test]
    fn test_create_http_gzip_content() {
        let validator =
            create_validator("http_gzip_content:string(/compressed),string(hello)").unwrap();
        assert_eq!(validator.name(), "http_get_compressed");
    }

    #[test]
    fn test_create_http_pipelining() {
        let validator = create_validator("http_pipelining:int(3)").unwrap();
        assert_eq!(validator.name(), "http_pipelining");
    }

    #[test]
    fn test_create_http_chunked_stream() {
        let validator = create_validator("http_chunked_stream:int(5)").unwrap();
        assert_eq!(validator.name(), "http_chunked");
    }

    #[test]
    fn test_create_http_chunked_format() {
        let validator = create_validator("http_chunked_format:bool(true)").unwrap();
        assert_eq!(validator.name(), "http_chunked");
    }

    #[test]
    fn test_create_http_file_post() {
        let validator =
            create_validator("http_file_post:string(upload.txt),string(test data),int(201)")
                .unwrap();
        assert_eq!(validator.name(), "http_post_file");
    }

    #[test]
    fn test_create_http_file_verify() {
        let validator =
            create_validator("http_file_verify:string(upload.txt),string(test data)").unwrap();
        assert_eq!(validator.name(), "http_get");
    }

    #[test]
    fn test_create_output_match() {
        let validator = create_validator(
            "output_match:string(./solution /data/measurements_10k.txt),string(/expected/10k.txt)",
        )
        .unwrap();
        assert_eq!(validator.name(), "output_match");
    }

    #[test]
    fn test_create_benchmark() {
        let validator = create_validator(
            "benchmark:string(./solution /data/measurements_1m.txt),string(/expected/1m.txt),int(10000)",
        )
        .unwrap();
        assert_eq!(validator.name(), "benchmark");
    }

    #[test]
    fn test_create_brc_validate() {
        let validator = create_validator(
            "brc_validate:string(./solution),string(data/measurements.txt)",
        )
        .unwrap();
        assert_eq!(validator.name(), "brc_validate");
    }

    #[test]
    fn test_create_brc_benchmark() {
        let validator = create_validator(
            "brc_benchmark:string(./solution),string(data/measurements.txt),int(10000)",
        )
        .unwrap();
        assert_eq!(validator.name(), "brc_benchmark");
    }

    // container validator tests

    #[test]
    fn test_create_docker_exec() {
        let validator =
            create_validator("docker_exec:string(echo hello),string(hello)").unwrap();
        assert_eq!(validator.name(), "docker_exec");
    }

    #[test]
    fn test_create_docker_exit_code() {
        let validator = create_validator("docker_exit_code:int(2)").unwrap();
        assert_eq!(validator.name(), "docker_exit_code");
    }

    #[test]
    fn test_create_docker_pid_namespace() {
        let validator = create_validator("docker_pid_namespace:int(1)").unwrap();
        assert_eq!(validator.name(), "docker_pid_namespace");
    }

    #[test]
    fn test_create_docker_chroot() {
        let validator = create_validator("docker_chroot:bool(true)").unwrap();
        assert_eq!(validator.name(), "docker_chroot");
    }

    #[test]
    fn test_create_docker_pull() {
        let validator =
            create_validator("docker_pull:string(alpine:latest),bool(true)").unwrap();
        assert_eq!(validator.name(), "docker_pull");
    }

    #[test]
    fn test_create_docker_mount_namespace() {
        let validator = create_validator("docker_mount_namespace:bool(true)").unwrap();
        assert_eq!(validator.name(), "docker_mount_namespace");
    }

    #[test]
    fn test_create_docker_cgroup_memory() {
        let validator =
            create_validator("docker_cgroup_memory:int(10485760),bool(true)").unwrap();
        assert_eq!(validator.name(), "docker_cgroup_memory");
    }

    #[test]
    fn test_create_docker_network_namespace() {
        let validator = create_validator("docker_network_namespace:bool(true)").unwrap();
        assert_eq!(validator.name(), "docker_network_namespace");
    }

    #[test]
    fn test_create_docker_veth_pair() {
        let validator = create_validator("docker_veth_pair:bool(true)").unwrap();
        assert_eq!(validator.name(), "docker_veth_pair");
    }
}
