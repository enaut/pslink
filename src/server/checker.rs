//! Health checks for link targets: the HTTP check itself, the periodic sweep and persistence helpers.

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use dioxus::logger::tracing::{debug, info, warn};
use reqwest::{
    Client, Method, Response, Url,
    header::{HeaderMap, LOCATION},
    redirect::Policy,
};
use tokio::{net::lookup_host, sync::Semaphore, task::JoinSet};

use crate::{
    domain::models::{CheckStatus, LinkCheck},
    server::{db, state::AppState},
};

/// Maximum duration of a single request.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// Maximum number of redirects that are followed.
const MAX_REDIRECTS: usize = 10;
/// Number of targets that are checked at the same time during a sweep.
const PARALLEL_CHECKS: usize = 5;
/// Delay between the server start and the first sweep.
const FIRST_SWEEP_DELAY: Duration = Duration::from_secs(30);

#[cfg(test)]
static ALLOWED_TEST_ADDRS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashSet<SocketAddr>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

/// Build the HTTP client used for all link checks.
///
/// # Panics
/// Panics if the TLS backend of the HTTP client cannot be initialized.
#[must_use]
pub fn build_client() -> Client {
    // reqwest is built without a default TLS provider, use `ring` like sqlx does.
    // This fails if a provider is already installed, which is fine.
    let _ = rustls::crypto::ring::default_provider().install_default();
    Client::builder()
        .user_agent(concat!(
            "Mozilla/5.0 (compatible; Pslink-LinkChecker/",
            env!("CARGO_PKG_VERSION"),
            ")"
        ))
        .timeout(REQUEST_TIMEOUT)
        .redirect(Policy::none())
        .build()
        .expect("Failed to build the HTTP client for link checks")
}

/// Check whether an IPv4 address is globally routable (rejecting loopback, private, link-local,
/// unspecified, broadcast, documentation, multicast, CGNAT, benchmark, and reserved ranges).
const fn is_allowed_ipv4(v4: Ipv4Addr) -> bool {
    let octets = v4.octets();
    let is_zero_network = octets[0] == 0; // 0.0.0.0/8 ("this" network)
    let is_shared_cgnat = octets[0] == 100 && (octets[1] & 0b1100_0000) == 64; // 100.64.0.0/10
    let is_ietf_protocol = octets[0] == 192 && octets[1] == 0 && octets[2] == 0; // 192.0.0.0/24
    let is_benchmark = octets[0] == 198 && (octets[1] & 0b1111_1110) == 18; // 198.18.0.0/15
    let is_reserved = (octets[0] & 0b1111_0000) == 240; // 240.0.0.0/4

    !v4.is_loopback()
        && !v4.is_private()
        && !v4.is_link_local()
        && !v4.is_unspecified()
        && !v4.is_broadcast()
        && !v4.is_documentation()
        && !v4.is_multicast()
        && !is_zero_network
        && !is_shared_cgnat
        && !is_ietf_protocol
        && !is_benchmark
        && !is_reserved
}

/// Check whether an IP address is globally routable (rejecting loopback, private, link-local,
/// unique-local, unspecified, multicast, documentation, and IPv4-mapped/compatible non-public IPs).
fn is_allowed_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_allowed_ipv4(v4),
        IpAddr::V6(v6) => {
            if let Some(mapped_v4) = v6.to_ipv4_mapped() {
                return is_allowed_ipv4(mapped_v4);
            }
            if let Some(compat_v4) = v6.to_ipv4() {
                return is_allowed_ipv4(compat_v4);
            }
            let segments = v6.segments();
            let is_documentation = segments[0] == 0x2001 && segments[1] == 0x0db8; // 2001:db8::/32
            !v6.is_loopback()
                && !v6.is_unspecified()
                && !v6.is_unique_local()
                && !v6.is_unicast_link_local()
                && !v6.is_multicast()
                && !is_documentation
        }
    }
}

fn is_allowed_socket_addr(addr: SocketAddr) -> bool {
    #[cfg(test)]
    if ALLOWED_TEST_ADDRS
        .lock()
        .is_ok_and(|set| set.contains(&addr))
    {
        return true;
    }
    is_allowed_ip(addr.ip())
}

/// Classify the final HTTP status code of a target.
///
/// 2xx/3xx is fine, 401/403/429 means "reachable but blocks us" and anything else is a failure.
#[must_use]
pub const fn classify_status(status: u16) -> CheckStatus {
    match status {
        401 | 403 | 429 => CheckStatus::Warning,
        200..=399 => CheckStatus::Ok,
        _ => CheckStatus::Failed,
    }
}

/// Decide from the response headers if the page may be shown inside an `<iframe>`.
fn is_embeddable(headers: &HeaderMap) -> bool {
    if headers.contains_key("x-frame-options") {
        return false;
    }
    for csp in headers.get_all("content-security-policy") {
        let Ok(csp) = csp.to_str() else { continue };
        for directive in csp.split(';') {
            let mut parts = directive.split_whitespace();
            if parts
                .next()
                .is_some_and(|name| name.eq_ignore_ascii_case("frame-ancestors"))
            {
                // Only an explicit wildcard allows every embedder.
                return parts.any(|source| source == "*");
            }
        }
    }
    true
}

#[derive(Debug)]
enum SendError {
    DisallowedAddress,
    InvalidUrl,
    TooManyRedirects,
    ConnectFailed,
    RequestFailed,
    Request(reqwest::Error),
}

impl SendError {
    fn describe(self) -> String {
        match self {
            Self::DisallowedAddress => "disallowed target address".to_string(),
            Self::InvalidUrl => "invalid target url".to_string(),
            Self::TooManyRedirects => "too many redirects".to_string(),
            Self::ConnectFailed => "connection failed".to_string(),
            Self::RequestFailed => "request failed".to_string(),
            Self::Request(error) => {
                let sanitized = error.without_url();
                debug!("Target check request error: {}", sanitized);
                describe_error(&sanitized)
            }
        }
    }
}

/// A generic client-facing description of a request error without raw cause chains.
fn describe_error(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        return "timeout".to_string();
    }
    if error.is_redirect() {
        return "too many redirects".to_string();
    }
    if error.is_connect() {
        "connection failed".to_string()
    } else {
        "request failed".to_string()
    }
}

async fn validate_destination(url: &Url) -> Result<(), SendError> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err(SendError::DisallowedAddress);
    }
    let host_str = url.host_str().ok_or(SendError::InvalidUrl)?;
    let port = url.port_or_known_default().ok_or(SendError::InvalidUrl)?;
    let host = host_str
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(host_str);

    if host.eq_ignore_ascii_case("localhost") || host.to_ascii_lowercase().ends_with(".localhost") {
        return Err(SendError::DisallowedAddress);
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        if !is_allowed_socket_addr(SocketAddr::new(ip, port)) {
            return Err(SendError::DisallowedAddress);
        }
        return Ok(());
    }

    let addrs = lookup_host((host, port))
        .await
        .map_err(|_| SendError::ConnectFailed)?;
    let mut found_any = false;
    for addr in addrs {
        found_any = true;
        if !is_allowed_socket_addr(addr) {
            return Err(SendError::DisallowedAddress);
        }
    }
    if !found_any {
        return Err(SendError::ConnectFailed);
    }
    Ok(())
}

async fn send(client: &Client, method: Method, url: &str) -> Result<Response, SendError> {
    let mut current_url = Url::parse(url).map_err(|_| SendError::InvalidUrl)?;
    for hop in 0..=MAX_REDIRECTS {
        validate_destination(&current_url).await?;
        let response = client
            .request(method.clone(), current_url.clone())
            .send()
            .await
            .map_err(SendError::Request)?;

        if matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308)
            && let Some(location) = response.headers().get(LOCATION)
        {
            if hop == MAX_REDIRECTS {
                return Err(SendError::TooManyRedirects);
            }
            let location_str = location.to_str().map_err(|_| SendError::RequestFailed)?;
            current_url = current_url
                .join(location_str)
                .map_err(|_| SendError::InvalidUrl)?;
            continue;
        }
        return Ok(response);
    }
    Err(SendError::TooManyRedirects)
}

/// Check one target url. This never fails: problems are part of the returned [`LinkCheck`].
pub async fn check_url(client: &Client, url: &str) -> LinkCheck {
    let started = std::time::Instant::now();

    let mut result = send(client, Method::HEAD, url).await;
    // Many servers do not implement or block HEAD requests so try again with a real GET.
    let retry_with_get = match &result {
        Ok(response) => matches!(response.status().as_u16(), 403 | 405 | 501),
        Err(_) => false,
    };
    if retry_with_get {
        result = send(client, Method::GET, url).await;
    }

    let duration_ms = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
    let checked_at = chrono::Utc::now().naive_utc();

    match result {
        Ok(response) => {
            let status = response.status().as_u16();
            LinkCheck {
                checked_at,
                status: classify_status(status),
                http_status: Some(status),
                error: None,
                final_url: Some(response.url().to_string()),
                duration_ms,
                embeddable: Some(is_embeddable(response.headers())),
            }
        }
        Err(error) => LinkCheck {
            checked_at,
            status: CheckStatus::Failed,
            http_status: None,
            error: Some(error.describe()),
            final_url: None,
            duration_ms,
            embeddable: None,
        },
    }
}

/// Check the target of a link and store the result in the history of the link.
pub async fn check_and_store(state: &AppState, link_id: i64, target: &str) -> LinkCheck {
    let check = check_url(&state.http, target).await;
    if let Err(e) =
        db::insert_link_check(&state.db, link_id, &check, state.config.check_history).await
    {
        warn!("Failed to store the check of link {}: {}", link_id, e);
    }
    check
}

/// Check the link in the background (used after create/modify).
pub fn spawn_check(state: AppState, link_id: i64, target: String) {
    tokio::spawn(async move {
        check_and_store(&state, link_id, &target).await;
    });
}

/// Check the targets of all links.
pub async fn sweep(state: &AppState) {
    let targets = match db::list_link_targets(&state.db).await {
        Ok(targets) => targets,
        Err(e) => {
            warn!("Could not load the links for the periodic check: {}", e);
            return;
        }
    };
    info!("Checking the targets of {} links", targets.len());

    let permits = Arc::new(Semaphore::new(PARALLEL_CHECKS));
    let mut tasks = JoinSet::new();
    for (link_id, target) in targets {
        let Ok(permit) = permits.clone().acquire_owned().await else {
            break;
        };
        let state = state.clone();
        tasks.spawn(async move {
            check_and_store(&state, link_id, &target).await;
            drop(permit);
        });
    }
    while tasks.join_next().await.is_some() {}
    info!("Finished checking the link targets");
}

/// Start the background task that periodically checks all link targets.
///
/// Does nothing if the configured interval is `0`.
pub fn spawn_periodic_checker(state: AppState) {
    let hours = state.config.check_interval_hours;
    if hours == 0 {
        info!("The periodic check of the link targets is disabled");
        return;
    }
    tokio::spawn(async move {
        tokio::time::sleep(FIRST_SWEEP_DELAY).await;
        let mut interval = tokio::time::interval(Duration::from_secs(hours.saturating_mul(3600)));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            sweep(&state).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    /// Start a tiny http server whose behaviour depends on the requested path.
    async fn start_test_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        ALLOWED_TEST_ADDRS.lock().unwrap().insert(addr);
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 4096];
                    let n = socket.read(&mut buf).await.unwrap_or(0);
                    let request = String::from_utf8_lossy(&buf[..n]).to_string();
                    let mut parts = request.lines().next().unwrap_or("").split(' ');
                    let method = parts.next().unwrap_or("");
                    let path = parts.next().unwrap_or("");
                    let response = match (method, path) {
                        (_, "/ok") => "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        (_, "/missing") => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        (_, "/error") => "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        (_, "/blocked") => "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        ("HEAD", "/nohead") => "HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        ("GET", "/nohead") => "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        (_, "/loop") => "HTTP/1.1 302 Found\r\nLocation: /loop\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        (_, "/redirect") => "HTTP/1.1 301 Moved Permanently\r\nLocation: /ok\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        (_, "/redirect-private") => "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        (_, "/framed") => "HTTP/1.1 200 OK\r\nX-Frame-Options: DENY\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        (_, "/csp") => "HTTP/1.1 200 OK\r\nContent-Security-Policy: default-src 'self'; frame-ancestors 'self'\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        (_, "/csp-any") => "HTTP/1.1 200 OK\r\nContent-Security-Policy: frame-ancestors *\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        _ => "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    };
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.shutdown().await;
                });
            }
        });
        format!("http://{addr}")
    }

    /// Needs internet access: `cargo test --features server -- --ignored`
    #[tokio::test]
    #[ignore = "needs internet access"]
    async fn test_check_url_https_real_site() {
        let client = build_client();
        let ok = check_url(&client, "https://example.com/").await;
        assert_eq!(ok.status, CheckStatus::Ok, "{ok:?}");

        let missing = check_url(&client, "https://example.com/this-does-not-exist").await;
        assert_eq!(missing.status, CheckStatus::Failed, "{missing:?}");

        let bad_cert = check_url(&client, "https://expired.badssl.com/").await;
        assert_eq!(bad_cert.status, CheckStatus::Failed, "{bad_cert:?}");
    }

    #[test]
    fn test_classify_status() {
        assert_eq!(classify_status(200), CheckStatus::Ok);
        assert_eq!(classify_status(302), CheckStatus::Ok);
        assert_eq!(classify_status(401), CheckStatus::Warning);
        assert_eq!(classify_status(403), CheckStatus::Warning);
        assert_eq!(classify_status(429), CheckStatus::Warning);
        assert_eq!(classify_status(404), CheckStatus::Failed);
        assert_eq!(classify_status(410), CheckStatus::Failed);
        assert_eq!(classify_status(500), CheckStatus::Failed);
    }

    #[tokio::test]
    async fn test_check_url_classification() {
        let base = start_test_server().await;
        let client = build_client();

        let ok = check_url(&client, &format!("{base}/ok")).await;
        assert_eq!(ok.status, CheckStatus::Ok);
        assert_eq!(ok.http_status, Some(200));
        assert_eq!(ok.embeddable, Some(true));

        let missing = check_url(&client, &format!("{base}/missing")).await;
        assert_eq!(missing.status, CheckStatus::Failed);
        assert_eq!(missing.http_status, Some(404));

        let error = check_url(&client, &format!("{base}/error")).await;
        assert_eq!(error.status, CheckStatus::Failed);

        let blocked = check_url(&client, &format!("{base}/blocked")).await;
        assert_eq!(blocked.status, CheckStatus::Warning);
        assert_eq!(blocked.http_status, Some(403));

        let nohead = check_url(&client, &format!("{base}/nohead")).await;
        assert_eq!(nohead.status, CheckStatus::Ok);

        let redirect = check_url(&client, &format!("{base}/redirect")).await;
        assert_eq!(redirect.status, CheckStatus::Ok);
        assert_eq!(redirect.final_url, Some(format!("{base}/ok")));
    }

    #[tokio::test]
    async fn test_check_url_errors() {
        let base = start_test_server().await;
        let client = build_client();

        let looping = check_url(&client, &format!("{base}/loop")).await;
        assert_eq!(looping.status, CheckStatus::Failed);
        assert_eq!(looping.http_status, None);
        assert_eq!(looping.error.as_deref(), Some("too many redirects"));

        // Closed port on an allowed test socket address returns generic "connection failed"
        // without leaking raw OS error causes.
        let closed_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let closed_addr = closed_listener.local_addr().unwrap();
        ALLOWED_TEST_ADDRS.lock().unwrap().insert(closed_addr);
        drop(closed_listener);

        let refused = check_url(&client, &format!("http://{closed_addr}/")).await;
        assert_eq!(refused.status, CheckStatus::Failed);
        assert_eq!(refused.error.as_deref(), Some("connection failed"));
    }

    #[tokio::test]
    async fn test_check_url_rejects_disallowed_destinations() {
        let base = start_test_server().await;
        let client = build_client();

        let disallowed = [
            "http://127.0.0.1:1/",
            "http://localhost/",
            "http://sub.localhost/",
            "http://[::1]/",
            "http://10.0.0.1/",
            "http://172.16.0.1/",
            "http://192.168.1.1/",
            "http://169.254.169.254/",
            "http://[fe80::1]/",
            "http://[fc00::1]/",
            "http://[fd00::1]/",
            "http://0.0.0.0/",
            "http://[::]/",
            "http://[::ffff:127.0.0.1]/",
            "http://[::ffff:169.254.169.254]/",
        ];

        for target in disallowed {
            let check = check_url(&client, target).await;
            assert_eq!(check.status, CheckStatus::Failed, "target={target}");
            assert_eq!(
                check.error.as_deref(),
                Some("disallowed target address"),
                "target={target}"
            );
        }

        let redirected = check_url(&client, &format!("{base}/redirect-private")).await;
        assert_eq!(redirected.status, CheckStatus::Failed);
        assert_eq!(
            redirected.error.as_deref(),
            Some("disallowed target address")
        );
    }

    #[tokio::test]
    async fn test_check_url_embeddable() {
        let base = start_test_server().await;
        let client = build_client();

        assert_eq!(
            check_url(&client, &format!("{base}/framed")).await.embeddable,
            Some(false)
        );
        assert_eq!(
            check_url(&client, &format!("{base}/csp")).await.embeddable,
            Some(false)
        );
        assert_eq!(
            check_url(&client, &format!("{base}/csp-any")).await.embeddable,
            Some(true)
        );
    }

    #[tokio::test]
    async fn test_sweep_stores_checks_for_all_links() {
        use crate::{
            domain::models::{Role, Secret},
            server::{
                cli::{MIGRATOR, Protocol, ServerConfig},
                db::{NewLink, NewUser},
            },
        };

        let base = start_test_server().await;
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        let secret = Secret::new("test-secret-key".to_string());
        NewUser::new("admin".into(), "a@b.c".into(), "adminpass", &secret)
            .unwrap()
            .insert_user(&pool, Role::Admin)
            .await
            .unwrap();
        for (code, path) in [("good", "ok"), ("bad", "missing")] {
            NewLink {
                title: code.to_string(),
                target: format!("{base}/{path}"),
                code: code.to_string(),
                author: 1,
                created_at: chrono::Utc::now().naive_utc(),
            }
            .insert(&pool)
            .await
            .unwrap();
        }
        let state = AppState::new(
            pool,
            ServerConfig {
                db: "test.db".into(),
                public_url: String::new(),
                empty_forward_url: String::new(),
                brand_name: String::new(),
                internal_ip: "127.0.0.1".into(),
                port: 0,
                protocol: Protocol::Http,
                secret: secret.clone(),
                data_download_secret: secret,
                check_interval_hours: 6,
                check_history: 2,
            },
        );

        // Three sweeps, but only two checks per link are kept.
        for _ in 0..3 {
            sweep(&state).await;
        }
        for (code, expected) in [("good", CheckStatus::Ok), ("bad", CheckStatus::Failed)] {
            let link = db::get_link_by_code(&state.db, code).await.unwrap();
            let history = db::list_link_checks(&state.db, link.id).await.unwrap();
            assert_eq!(history.len(), 2);
            assert!(history.iter().all(|c| c.status == expected));
        }
    }
}
