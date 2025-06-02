use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures_util::future::LocalBoxFuture;
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::OpenOptions,
    future::{ready, Ready},
    io::Write,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    pub timestamp: u64,
    pub client_ip: String,
    pub method: String,
    pub uri: String,
    pub user_agent: Option<String>,
    pub status_code: u16,
    pub response_time_ms: u128,
    pub request_size: usize,
    pub response_size: usize,
    pub status_category: StatusCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatusCategory {
    Success,    // 2xx
    Redirect,   // 3xx
    ClientError, // 4xx
    ServerError, // 5xx
    Other,
}

impl StatusCategory {
    fn from_status_code(code: u16) -> Self {
        match code {
            200..=299 => StatusCategory::Success,
            300..=399 => StatusCategory::Redirect,
            400..=499 => StatusCategory::ClientError,
            500..=599 => StatusCategory::ServerError,
            _ => StatusCategory::Other,
        }
    }

    fn emoji(&self) -> &'static str {
        match self {
            StatusCategory::Success => "✅",
            StatusCategory::Redirect => "↩️",
            StatusCategory::ClientError => "❌",
            StatusCategory::ServerError => "💥",
            StatusCategory::Other => "❓",
        }
    }
}

#[derive(Clone)]
pub struct RequestLoggerConfig {
    pub log_to_console: bool,
    pub log_to_file: bool,
    pub log_file_path: String,
}

impl Default for RequestLoggerConfig {
    fn default() -> Self {
        Self {
            log_to_console: true,
            log_to_file: false,
            log_file_path: "requests.log".to_string(),
        }
    }
}

impl RequestLoggerConfig {
    pub fn from_env() -> Self {
        Self {
            log_to_console: env::var("LOG_REQUESTS_CONSOLE")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            log_to_file: env::var("LOG_REQUESTS_FILE")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            log_file_path: env::var("LOG_REQUESTS_FILE_PATH")
                .unwrap_or_else(|_| "requests.log".to_string()),
        }
    }
}

pub struct RequestLogger {
    config: RequestLoggerConfig,
}

impl RequestLogger {
    pub fn new(config: RequestLoggerConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(RequestLoggerConfig::from_env())
    }

    fn log_request(&self, log: &RequestLog) {
        if self.config.log_to_console {
            self.log_to_console(log);
        }

        if self.config.log_to_file {
            if let Err(e) = self.log_to_file(log) {
                error!("Failed to write to log file: {}", e);
            }
        }
    }

    fn log_to_console(&self, log: &RequestLog) {
        let log_message = format!(
            "{} {} {} {} - {} {}ms [{}->{}] {}",
            log.status_category.emoji(),
            log.method,
            log.uri,
            log.client_ip,
            log.status_code,
            log.response_time_ms,
            log.request_size,
            log.response_size,
            log.user_agent.as_deref().unwrap_or("Unknown")
        );

        match log.status_category {
            StatusCategory::Success => info!("{}", log_message),
            StatusCategory::Redirect => info!("{}", log_message),
            StatusCategory::ClientError => warn!("{}", log_message),
            StatusCategory::ServerError => error!("{}", log_message),
            StatusCategory::Other => warn!("{}", log_message),
        };
    }

    fn log_to_file(&self, log: &RequestLog) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.log_file_path)?;

        let log_entry = format!(
            "{} [{}] {} {} {} - {} {}ms [{}->{}] {}\n",
            log.timestamp,
            log.status_category.emoji(),
            log.method,
            log.uri,
            log.client_ip,
            log.status_code,
            log.response_time_ms,
            log.request_size,
            log.response_size,
            log.user_agent.as_deref().unwrap_or("Unknown")
        );

        file.write_all(log_entry.as_bytes())?;
        file.flush()?;
        Ok(())
    }
}

impl<S, B> Transform<S, ServiceRequest> for RequestLogger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RequestLoggerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestLoggerMiddleware {
            service: Rc::new(service),
            config: self.config.clone(),
        }))
    }
}

pub struct RequestLoggerMiddleware<S> {
    service: Rc<S>,
    config: RequestLoggerConfig,
}

impl<S, B> Service<ServiceRequest> for RequestLoggerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start_time = SystemTime::now();
        let client_ip = req
            .connection_info()
            .peer_addr()
            .unwrap_or("unknown")
            .to_string();
        let method = req.method().to_string();
        let uri = req.uri().to_string();
        let user_agent = req
            .headers()
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        // Get request size (approximate)
        let request_size = req
            .headers()
            .get("content-length")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        let service = Rc::clone(&self.service);
        let config = self.config.clone();

        Box::pin(async move {
            let res = service.call(req).await?;
            
            let elapsed = start_time
                .elapsed()
                .unwrap_or_default()
                .as_millis();

            let status_code = res.status().as_u16();
            let status_category = StatusCategory::from_status_code(status_code);

            // Get response size (approximate)
            let response_size = res
                .headers()
                .get("content-length")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);

            let log = RequestLog {
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                client_ip,
                method,
                uri,
                user_agent,
                status_code,
                response_time_ms: elapsed,
                request_size,
                response_size,
                status_category,
            };

            let logger = RequestLogger::new(config);
            logger.log_request(&log);

            Ok(res)
        })
    }
}