// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 Sebastien Rousseau

// src/server.rs

//! Core HTTP server runtime.
//!
//! Use this module when you need a static-first HTTP server with predictable request parsing,
//! policy-aware response generation, and portable runtime behavior across macOS, Linux, and WSL.
//!
//! The primary entrypoints are [`Server`] and [`ServerBuilder`].
//!

use crate::error::ServerError;
use crate::request::Request;
use crate::response::Response;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::net::{IpAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, Once, OnceLock};
use std::thread;
use std::time::{Duration, Instant, UNIX_EPOCH};

static SHUTDOWN_SIGNAL_SLOT: OnceLock<
    Mutex<Option<Arc<ShutdownSignal>>>,
> = OnceLock::new();
static SIGNAL_HANDLER_INSTALL: Once = Once::new();
static RATE_LIMIT_STATE: OnceLock<
    Mutex<HashMap<IpAddr, Vec<Instant>>>,
> = OnceLock::new();
static METRIC_REQUESTS_TOTAL: AtomicUsize = AtomicUsize::new(0);
static METRIC_RESPONSES_4XX: AtomicUsize = AtomicUsize::new(0);
static METRIC_RESPONSES_5XX: AtomicUsize = AtomicUsize::new(0);
static METRIC_RATE_LIMITED: AtomicUsize = AtomicUsize::new(0);

/// Serves static HTTP content with configurable runtime policies.
///
/// You use `Server` as the main entrypoint to bind an address, map requests to files
/// under a document root, and apply response policies such as CORS, cache hints, and
/// simple rate limiting.
///
/// For most production setups, prefer [`Server::builder`] so optional settings are
/// explicit and readable.
///
/// # Examples
///
/// ```rust
/// use http_handle::Server;
///
/// let server = Server::new("127.0.0.1:8080", ".");
/// assert_eq!(server.address(), "127.0.0.1:8080");
/// ```
///
/// # Panics
///
/// This type does not panic on construction.
#[doc(alias = "http server")]
#[doc(alias = "static file server")]
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize,
)]
pub struct Server {
    address: String,
    document_root: PathBuf,
    /// Canonicalized `document_root` cached at build time. Skipped from
    /// serde so the wire shape of `Server` is unchanged; recomputed on
    /// deserialize via `Default`.
    #[serde(skip, default)]
    canonical_document_root: PathBuf,
    cors_enabled: Option<bool>,
    cors_origins: Option<Vec<String>>,
    custom_headers: Option<HashMap<String, String>>,
    request_timeout: Option<Duration>,
    connection_timeout: Option<Duration>,
    rate_limit_per_minute: Option<usize>,
    static_cache_ttl_secs: Option<u64>,
}

/// Builds a [`Server`] with optional policy and timeout configuration.
///
/// You use `ServerBuilder` when you want a fluent, explicit configuration surface for
/// CORS, custom headers, timeouts, and rate limiting.
///
/// # Examples
///
/// ```rust
/// use http_handle::Server;
///
/// let server = Server::builder()
///     .address("127.0.0.1:8080")
///     .document_root(".")
///     .enable_cors()
///     .build()
///     .expect("valid builder config");
///
/// assert_eq!(server.address(), "127.0.0.1:8080");
/// ```
///
/// # Errors
///
/// Builder finalization fails when required fields are missing.
///
/// # Panics
///
/// This type does not panic under normal usage.
#[doc(alias = "builder")]
#[doc(alias = "configuration")]
#[derive(Clone, Debug, Default)]
pub struct ServerBuilder {
    address: Option<String>,
    document_root: Option<PathBuf>,
    cors_enabled: Option<bool>,
    cors_origins: Option<Vec<String>>,
    custom_headers: Option<HashMap<String, String>>,
    request_timeout: Option<Duration>,
    connection_timeout: Option<Duration>,
    rate_limit_per_minute: Option<usize>,
    static_cache_ttl_secs: Option<u64>,
}

impl ServerBuilder {
    /// Creates a new builder with no required fields set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::server::ServerBuilder;
    ///
    /// let builder = ServerBuilder::new();
    /// let _ = builder;
    /// assert_eq!(2 + 2, 4);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    #[doc(alias = "new builder")]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the bind address (`ip:port`) for the server.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::Server;
    ///
    /// let server = Server::builder()
    ///     .address("127.0.0.1:8080")
    ///     .document_root(".")
    ///     .build()
    ///     .expect("builder should succeed");
    /// assert_eq!(server.address(), "127.0.0.1:8080");
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    #[doc(alias = "bind address")]
    pub fn address(mut self, address: &str) -> Self {
        self.address = Some(address.to_string());
        self
    }

    /// Sets the document root directory used for file resolution.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::Server;
    ///
    /// let server = Server::builder()
    ///     .address("127.0.0.1:8080")
    ///     .document_root(".")
    ///     .build()
    ///     .expect("builder should succeed");
    /// assert_eq!(server.document_root().as_path(), std::path::Path::new("."));
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    #[doc(alias = "document root")]
    pub fn document_root(mut self, path: &str) -> Self {
        self.document_root = Some(PathBuf::from(path));
        self
    }

    /// Enables CORS with default settings
    pub fn enable_cors(mut self) -> Self {
        self.cors_enabled = Some(true);
        self
    }

    /// Disables CORS
    pub fn disable_cors(mut self) -> Self {
        self.cors_enabled = Some(false);
        self
    }

    /// Sets allowed CORS origins
    pub fn cors_origins(mut self, origins: Vec<String>) -> Self {
        self.cors_origins = Some(origins);
        self.cors_enabled = Some(true); // Auto-enable CORS when origins are set
        self
    }

    /// Adds a custom header that will be included in all responses
    pub fn custom_header(mut self, name: &str, value: &str) -> Self {
        let mut headers = self.custom_headers.unwrap_or_default();
        let _ = headers.insert(name.to_string(), value.to_string());
        self.custom_headers = Some(headers);
        self
    }

    /// Sets multiple custom headers
    pub fn custom_headers(
        mut self,
        headers: HashMap<String, String>,
    ) -> Self {
        self.custom_headers = Some(headers);
        self
    }

    /// Sets the request timeout duration
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = Some(timeout);
        self
    }

    /// Sets the connection timeout duration
    pub fn connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = Some(timeout);
        self
    }

    /// Sets a simple per-IP request rate limit per minute.
    pub fn rate_limit_per_minute(mut self, requests: usize) -> Self {
        self.rate_limit_per_minute = Some(requests.max(1));
        self
    }

    /// Sets a default static cache max-age (in seconds).
    pub fn static_cache_ttl_secs(mut self, ttl: u64) -> Self {
        self.static_cache_ttl_secs = Some(ttl);
        self
    }

    /// Finalizes builder state into a [`Server`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::Server;
    ///
    /// let ok = Server::builder()
    ///     .address("127.0.0.1:8080")
    ///     .document_root(".")
    ///     .build();
    /// assert!(ok.is_ok());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `Err` when:
    /// - the address was not provided.
    /// - the document root was not provided.
    ///
    /// # Panics
    ///
    /// This function does not panic.
    #[doc(alias = "finalize")]
    pub fn build(self) -> Result<Server, &'static str> {
        let address = self.address.ok_or("Address is required")?;
        let document_root =
            self.document_root.ok_or("Document root is required")?;
        // Canonicalize once at build time so the request hot path no longer
        // issues two fs::canonicalize syscalls per request.
        let canonical_document_root = fs::canonicalize(&document_root)
            .unwrap_or_else(|_| document_root.clone());

        Ok(Server {
            address,
            document_root,
            canonical_document_root,
            cors_enabled: self.cors_enabled,
            cors_origins: self.cors_origins,
            custom_headers: self.custom_headers,
            request_timeout: self.request_timeout,
            connection_timeout: self.connection_timeout,
            rate_limit_per_minute: self.rate_limit_per_minute,
            static_cache_ttl_secs: self.static_cache_ttl_secs,
        })
    }
}

/// Holds shutdown state and coordination for graceful server termination
#[derive(Debug, Clone)]
pub struct ShutdownSignal {
    /// Flag indicating if shutdown has been requested
    pub should_shutdown: Arc<AtomicBool>,
    /// Counter tracking active connections
    pub active_connections: Arc<AtomicUsize>,
    /// Maximum time to wait for connections to drain during shutdown
    pub shutdown_timeout: Duration,
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new(Duration::from_secs(30))
    }
}

impl ShutdownSignal {
    /// Creates a new shutdown signal with the specified timeout
    pub fn new(shutdown_timeout: Duration) -> Self {
        Self {
            should_shutdown: Arc::new(AtomicBool::new(false)),
            active_connections: Arc::new(AtomicUsize::new(0)),
            shutdown_timeout,
        }
    }

    /// Signals that shutdown should begin
    pub fn shutdown(&self) {
        self.should_shutdown.store(true, Ordering::SeqCst);
        println!(
            "🛑 Shutdown signal received. Waiting for active connections to finish..."
        );
    }

    /// Check if shutdown has been requested
    pub fn is_shutdown_requested(&self) -> bool {
        self.should_shutdown.load(Ordering::SeqCst)
    }

    /// Increment the active connection counter
    pub fn connection_started(&self) {
        let _ = self.active_connections.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrement the active connection counter
    pub fn connection_finished(&self) {
        let _ = self.active_connections.fetch_sub(1, Ordering::SeqCst);
    }

    /// Get the current number of active connections
    pub fn active_connection_count(&self) -> usize {
        self.active_connections.load(Ordering::SeqCst)
    }

    /// Wait for all connections to drain or timeout to expire
    pub fn wait_for_shutdown(&self) -> bool {
        let start_time = Instant::now();

        while self.active_connection_count() > 0
            && start_time.elapsed() < self.shutdown_timeout
        {
            let remaining = self
                .shutdown_timeout
                .saturating_sub(start_time.elapsed());
            println!(
                "⏳ Waiting for {} active connection(s) to finish... ({:.1}s remaining)",
                self.active_connection_count(),
                remaining.as_secs_f32()
            );

            // Sleep in short intervals to avoid overshooting small timeouts.
            thread::sleep(remaining.min(Duration::from_millis(50)));
        }

        let remaining_connections = self.active_connection_count();
        if remaining_connections > 0 {
            println!(
                "⚠️  Shutdown timeout reached. {} connection(s) will be forcibly terminated.",
                remaining_connections
            );
            false
        } else {
            println!("✅ All connections closed gracefully.");
            true
        }
    }
}

/// A simple thread pool for handling concurrent connections efficiently
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Sender<Job>,
}

impl std::fmt::Debug for ThreadPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThreadPool")
            .field("workers", &self.workers)
            .field("sender", &"<Sender<Job>>")
            .finish()
    }
}

/// Represents a job that can be executed by the thread pool
type Job = Box<dyn FnOnce() + Send + 'static>;

/// A worker thread that processes jobs from the thread pool queue
#[derive(Debug)]
struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl ThreadPool {
    /// Creates a new ThreadPool with the specified number of threads.
    ///
    /// # Arguments
    /// * `size` - The number of threads in the pool
    ///
    /// # Panics
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        // Return configured thread_pool instance
        ThreadPool { workers, sender }
    }

    /// Execute a job on the thread pool.
    ///
    /// # Arguments
    /// * `f` - The closure to execute
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Close the job channel first so workers exit their recv() loop.
        let (replacement_sender, _replacement_receiver) =
            mpsc::channel();
        let old_sender =
            std::mem::replace(&mut self.sender, replacement_sender);
        drop(old_sender);

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let job = receiver.lock().unwrap().recv();

                match job {
                    Ok(job) => {
                        job();
                    }
                    Err(_) => {
                        println!(
                            "Worker {} disconnected; shutting down.",
                            id
                        );
                        break;
                    }
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

/// Holds the connection pool state for managing database or external connections
#[derive(Debug)]
pub struct ConnectionPool {
    max_connections: usize,
    active_connections: Arc<AtomicUsize>,
}

impl ConnectionPool {
    /// Creates a new connection pool with the specified maximum connections
    pub fn new(max_connections: usize) -> Self {
        // Initialize connection_pool with bounded resources
        Self {
            max_connections,
            active_connections: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Attempts to acquire a connection from the pool
    pub fn acquire(&self) -> Result<ConnectionGuard, io::Error> {
        #[allow(deprecated_in_future)]
        let reserved = self.active_connections.fetch_update(
            Ordering::SeqCst,
            Ordering::SeqCst,
            |current| {
                if current < self.max_connections {
                    Some(current + 1)
                } else {
                    None
                }
            },
        );
        if reserved.is_err() {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Connection pool exhausted",
            ));
        }
        Ok(ConnectionGuard {
            pool: Arc::clone(&self.active_connections),
        })
    }

    /// Returns the current number of active connections
    pub fn active_count(&self) -> usize {
        self.active_connections.load(Ordering::SeqCst)
    }
}

/// RAII guard for connection pool resources
#[derive(Debug)]
pub struct ConnectionGuard {
    pool: Arc<AtomicUsize>,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let _ = self.pool.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Server {
    /// Creates a server using the minimal required configuration.
    ///
    /// Use this constructor when you want a quick default path. For advanced runtime
    /// policy, prefer [`Server::builder`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::Server;
    ///
    /// let server = Server::new("127.0.0.1:8080", ".");
    /// assert_eq!(server.address(), "127.0.0.1:8080");
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    #[doc(alias = "constructor")]
    pub fn new(address: &str, document_root: &str) -> Self {
        let document_root = PathBuf::from(document_root);
        let canonical_document_root = fs::canonicalize(&document_root)
            .unwrap_or_else(|_| document_root.clone());
        Server {
            address: address.to_string(),
            document_root,
            canonical_document_root,
            cors_enabled: None,
            cors_origins: None,
            custom_headers: None,
            request_timeout: None,
            connection_timeout: None,
            rate_limit_per_minute: None,
            static_cache_ttl_secs: None,
        }
    }

    /// Returns a fluent builder for optional server policies.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_handle::Server;
    ///
    /// let server = Server::builder()
    ///     .address("127.0.0.1:8080")
    ///     .document_root(".")
    ///     .build()
    ///     .expect("builder should succeed");
    /// assert_eq!(server.address(), "127.0.0.1:8080");
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic.
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    /// Starts a blocking HTTP/1.1 listener loop.
    ///
    /// On Linux, macOS, and Windows, this binds a `TcpListener` and accepts connections
    /// in a thread-per-connection model.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_handle::Server;
    ///
    /// let server = Server::new("127.0.0.1:8080", ".");
    /// let _ = server.start();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `Err` if binding fails or the listener cannot be configured.
    ///
    /// # Panics
    ///
    /// This function does not intentionally panic.
    #[doc(alias = "listen")]
    #[doc(alias = "serve")]
    pub fn start(&self) -> io::Result<()> {
        let listener = TcpListener::bind(&self.address)?;
        println!("❯ Server is now running at http://{}", self.address);
        println!("  Document root: {}", self.document_root.display());
        println!("  Press Ctrl+C to stop the server.");

        Self::run_basic_accept_loop(listener.incoming(), self.clone());

        Ok(())
    }

    /// Starts the server with OS-signal-aware graceful shutdown.
    ///
    /// On macOS/Linux, this responds to `SIGINT`/`SIGTERM` via the installed signal handler.
    /// On Windows, `Ctrl+C` triggers equivalent shutdown behavior through the same handler API.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_handle::Server;
    /// use std::time::Duration;
    ///
    /// let server = Server::new("127.0.0.1:8080", ".");
    /// let _ = server.start_with_graceful_shutdown(Duration::from_secs(5));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `Err` when binding or socket configuration fails.
    ///
    /// # Panics
    ///
    /// This function does not intentionally panic.
    #[doc(alias = "graceful shutdown")]
    pub fn start_with_graceful_shutdown(
        &self,
        shutdown_timeout: Duration,
    ) -> io::Result<()> {
        let shutdown = Arc::new(ShutdownSignal::new(shutdown_timeout));
        self.start_with_shutdown_signal(shutdown)
    }

    /// Starts the server with caller-managed shutdown coordination.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_handle::{Server, ShutdownSignal};
    /// use std::sync::Arc;
    /// use std::time::Duration;
    ///
    /// let server = Server::new("127.0.0.1:8080", ".");
    /// let signal = Arc::new(ShutdownSignal::new(Duration::from_secs(2)));
    /// let _ = server.start_with_shutdown_signal(signal);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `Err` when binding or listener configuration fails.
    ///
    /// # Panics
    ///
    /// This function does not intentionally panic.
    #[doc(alias = "shutdown signal")]
    pub fn start_with_shutdown_signal(
        &self,
        shutdown: Arc<ShutdownSignal>,
    ) -> io::Result<()> {
        self.start_with_shutdown_signal_and_ready(shutdown, |_| {})
    }

    /// Starts the server with a shutdown signal and reports the actual bound address.
    ///
    /// This is useful when binding to port `0` in tests and callers need the kernel-assigned
    /// port before sending requests.
    ///
    /// # Arguments
    ///
    /// * `shutdown` - The shutdown signal to coordinate graceful termination
    /// * `on_ready` - Callback invoked once with the actual bound `ip:port`
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an I/O error.
    pub fn start_with_shutdown_signal_and_ready<F>(
        &self,
        shutdown: Arc<ShutdownSignal>,
        on_ready: F,
    ) -> io::Result<()>
    where
        F: FnOnce(String),
    {
        // Install signal handlers
        Self::install_signal_handlers(shutdown.clone());

        let listener = TcpListener::bind(&self.address)?;
        let bound_address = listener.local_addr()?.to_string();
        on_ready(bound_address.clone());
        println!("❯ Server is now running at http://{}", bound_address);
        println!("  Document root: {}", self.document_root.display());
        println!("  Press Ctrl+C to stop the server gracefully.");

        // Set a short timeout on the listener to allow checking shutdown signal
        listener.set_nonblocking(true)?;

        loop {
            // Check if shutdown was requested
            if shutdown.is_shutdown_requested() {
                println!(
                    "🛑 Shutdown requested. Stopping new connections..."
                );
                break;
            }

            match listener.accept() {
                Ok((stream, _addr)) => Self::run_tracked_accept(
                    stream,
                    self.clone(),
                    shutdown.clone(),
                ),
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // No connection waiting, sleep briefly and continue
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => Self::log_listener_error(e),
            }
        }

        // Wait for existing connections to finish
        let graceful = shutdown.wait_for_shutdown();

        if graceful {
            println!("✅ Server shut down gracefully.");
        } else {
            println!(
                "⚠️  Server shut down with active connections remaining."
            );
        }

        Ok(())
    }

    /// Installs signal handlers for graceful shutdown
    ///
    /// # Arguments
    ///
    /// * `shutdown` - The shutdown signal to trigger when signals are received
    fn install_signal_handlers(shutdown: Arc<ShutdownSignal>) {
        let slot =
            SHUTDOWN_SIGNAL_SLOT.get_or_init(|| Mutex::new(None));

        // Update the active shutdown signal for this server run.
        if let Ok(mut guard) = slot.lock() {
            *guard = Some(shutdown);
        }

        // Register the OS signal handler once per process.
        SIGNAL_HANDLER_INSTALL.call_once(|| {
            let _ = ctrlc::set_handler(Self::handle_shutdown_signal);
        });
    }

    fn handle_shutdown_signal() {
        if let Some(slot) = SHUTDOWN_SIGNAL_SLOT.get() {
            Self::trigger_shutdown_from_slot(slot);
        }
    }

    fn trigger_shutdown_from_slot(
        slot: &Mutex<Option<Arc<ShutdownSignal>>>,
    ) {
        if let Ok(guard) = slot.lock()
            && let Some(shutdown_signal) = guard.as_ref()
        {
            shutdown_signal.shutdown();
        }
    }

    /// Starts the server with thread pooling for better resource management under load.
    ///
    /// This method uses a fixed-size thread pool to handle connections, preventing
    /// resource exhaustion under high load by limiting the number of concurrent threads.
    ///
    /// # Arguments
    ///
    /// * `thread_pool_size` - The number of worker threads in the pool
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an I/O error.
    pub fn start_with_thread_pool(
        &self,
        thread_pool_size: usize,
    ) -> io::Result<()> {
        let thread_pool = ThreadPool::new(thread_pool_size);
        let listener = TcpListener::bind(&self.address)?;

        println!("❯ Server is now running at http://{}", self.address);
        println!("  Document root: {}", self.document_root.display());
        println!("  Thread pool size: {} workers", thread_pool_size);
        println!("  Press Ctrl+C to stop the server.");

        Self::run_thread_pool_accept_loop(
            listener.incoming(),
            self.clone(),
            &thread_pool,
        );

        Ok(())
    }

    /// Starts the server with both thread pooling and connection pooling for optimal resource management.
    ///
    /// This method provides the highest level of resource control by combining:
    /// - Fixed-size thread pool to limit concurrent worker threads
    /// - Connection pool to limit the number of simultaneously processed connections
    /// - Graceful degradation when limits are reached
    ///
    /// # Arguments
    ///
    /// * `thread_pool_size` - The number of worker threads in the pool
    /// * `max_connections` - The maximum number of concurrent connections to process
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an I/O error.
    pub fn start_with_pooling(
        &self,
        thread_pool_size: usize,
        max_connections: usize,
    ) -> io::Result<()> {
        let thread_pool = ThreadPool::new(thread_pool_size);
        let connection_pool =
            Arc::new(ConnectionPool::new(max_connections));
        let listener = TcpListener::bind(&self.address)?;

        println!("❯ Server is now running at http://{}", self.address);
        println!("  Document root: {}", self.document_root.display());
        println!("  Thread pool size: {} workers", thread_pool_size);
        println!("  Max concurrent connections: {}", max_connections);
        println!("  Press Ctrl+C to stop the server.");

        Self::run_pooling_accept_loop(
            listener.incoming(),
            self.clone(),
            &thread_pool,
            connection_pool,
        );

        Ok(())
    }

    fn log_connection_result(result: Result<(), ServerError>) {
        if let Err(error) = result {
            eprintln!("Error handling connection: {}", error);
        }
    }

    fn log_listener_error(error: io::Error) {
        eprintln!("Connection error: {}", error);
    }

    fn run_tracked_accept(
        stream: TcpStream,
        server: Server,
        shutdown: Arc<ShutdownSignal>,
    ) {
        shutdown.connection_started();
        let _ = thread::spawn(move || {
            let result =
                handle_connection_tracked(stream, &server, &shutdown);
            shutdown.connection_finished();
            Self::log_connection_result(result);
        });
    }

    fn run_basic_accept_loop<I>(incoming: I, server: Server)
    where
        I: IntoIterator<Item = io::Result<TcpStream>>,
    {
        for stream in incoming {
            match stream {
                Ok(stream) => {
                    let server = server.clone();
                    let _ = thread::spawn(move || {
                        Self::log_connection_result(handle_connection(
                            stream, &server,
                        ));
                    });
                }
                Err(error) => Self::log_listener_error(error),
            }
        }
    }

    fn run_thread_pool_accept_loop<I>(
        incoming: I,
        server: Server,
        thread_pool: &ThreadPool,
    ) where
        I: IntoIterator<Item = io::Result<TcpStream>>,
    {
        for stream in incoming {
            match stream {
                Ok(stream) => {
                    let server = server.clone();
                    thread_pool.execute(move || {
                        Self::log_connection_result(handle_connection(
                            stream, &server,
                        ));
                    });
                }
                Err(error) => Self::log_listener_error(error),
            }
        }
    }

    fn run_pooling_accept_loop<I>(
        incoming: I,
        server: Server,
        thread_pool: &ThreadPool,
        connection_pool: Arc<ConnectionPool>,
    ) where
        I: IntoIterator<Item = io::Result<TcpStream>>,
    {
        for stream in incoming {
            match stream {
                Ok(stream) => {
                    let server = server.clone();
                    let pool_clone = Arc::clone(&connection_pool);
                    thread_pool.execute(move || match pool_clone.acquire() {
                        Ok(_guard) => Self::log_connection_result(
                            handle_connection(stream, &server),
                        ),
                        Err(_) => {
                            if let Err(error) =
                                send_service_unavailable(stream)
                            {
                                eprintln!(
                                    "Error sending service unavailable: {}",
                                    error
                                );
                            }
                        }
                    });
                }
                Err(error) => Self::log_listener_error(error),
            }
        }
    }

    // Getter methods for configuration fields (needed for testing)

    /// Returns the CORS enabled setting
    pub fn cors_enabled(&self) -> Option<bool> {
        self.cors_enabled
    }

    /// Returns the CORS origins setting
    pub fn cors_origins(&self) -> &Option<Vec<String>> {
        &self.cors_origins
    }

    /// Returns the custom headers setting
    pub fn custom_headers(&self) -> &Option<HashMap<String, String>> {
        &self.custom_headers
    }

    /// Returns the request timeout setting
    pub fn request_timeout(&self) -> Option<Duration> {
        self.request_timeout
    }

    /// Returns the connection timeout setting
    pub fn connection_timeout(&self) -> Option<Duration> {
        self.connection_timeout
    }

    /// Returns the server address
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Returns the document root path
    pub fn document_root(&self) -> &PathBuf {
        &self.document_root
    }
}

/// Sends a 503 Service Unavailable response when connection pool is exhausted.
///
/// # Arguments
///
/// * `mut stream` - The TCP stream to send the response to
///
/// # Returns
///
/// A `Result` indicating success or an I/O error.
fn send_service_unavailable(mut stream: TcpStream) -> io::Result<()> {
    let mut response = Response::new(
        503,
        "SERVICE UNAVAILABLE",
        b"Service temporarily unavailable. Please try again later."
            .to_vec(),
    );

    response.add_header("Content-Type", "text/plain");
    response.add_header("Retry-After", "1"); // Suggest client retry after 1 second
    response.add_header("Connection", "close");

    response.send(&mut stream).map_err(|e| {
        use std::io::Error;
        Error::other(format!("Failed to send response: {}", e))
    })?;
    Ok(())
}

/// Handles a single client connection.
///
/// # Arguments
///
/// * `stream` - A `TcpStream` representing the client connection.
/// * `document_root` - A `PathBuf` representing the server's document root.
///
/// # Returns
///
/// A `Result` indicating success or a `ServerError`.
pub(crate) fn handle_connection(
    mut stream: TcpStream,
    server: &Server,
) -> Result<(), ServerError> {
    // Disable Nagle so small responses ship immediately instead of
    // stalling behind delayed-ACK on the client side.
    let _ = stream.set_nodelay(true);
    let timeout =
        server.request_timeout.unwrap_or(Duration::from_secs(30));
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;

    let peer_ip = stream.peer_addr().ok().map(|addr| addr.ip());
    let response = build_response_for_stream(server, &stream, peer_ip);
    response.send(&mut stream)?;
    Ok(())
}

/// Handles a single client connection with shutdown signal awareness.
///
/// This function is similar to `handle_connection` but can be interrupted
/// during shutdown sequences.
///
/// # Arguments
///
/// * `stream` - A `TcpStream` representing the client connection.
/// * `document_root` - A `Path` representing the server's document root.
/// * `shutdown` - The shutdown signal for coordination
///
/// # Returns
///
/// A `Result` indicating success or a `ServerError`.
fn handle_connection_tracked(
    mut stream: TcpStream,
    server: &Server,
    _shutdown: &ShutdownSignal,
) -> Result<(), ServerError> {
    // Ensure per-connection reads are blocking even if the listener is non-blocking.
    stream.set_nonblocking(false)?;
    // Disable Nagle — small responses should not wait for delayed ACKs.
    let _ = stream.set_nodelay(true);

    // Set a reasonable timeout for connection handling
    let timeout =
        server.connection_timeout.unwrap_or(Duration::from_secs(30));
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;

    let peer_ip = stream.peer_addr().ok().map(|addr| addr.ip());
    let response = build_response_for_stream(server, &stream, peer_ip);
    response.send(&mut stream)?;
    Ok(())
}

fn build_response_for_stream(
    server: &Server,
    stream: &TcpStream,
    peer_ip: Option<IpAddr>,
) -> Response {
    match Request::from_stream(stream) {
        Ok(request) => {
            if request.path() == "/metrics" && request.method() == "GET"
            {
                return generate_metrics_response();
            }
            if let Some(ip) = peer_ip
                && is_rate_limited(server, ip)
            {
                let _ =
                    METRIC_RATE_LIMITED.fetch_add(1, Ordering::Relaxed);
                return generate_too_many_requests_response();
            }
            build_response_for_request_with_metrics(server, &request)
        }
        Err(error) => {
            response_from_error(&error, &server.document_root)
        }
    }
}

/// Builds a response for an already parsed request and records response metrics.
///
/// This is shared by protocol-specific frontends (for example HTTP/1 and HTTP/2)
/// to keep behavior consistent across server entrypoints.
pub(crate) fn build_response_for_request_with_metrics(
    server: &Server,
    request: &Request,
) -> Response {
    let response = build_response_for_request(server, request);
    record_metrics(&response);
    response
}

/// Builds a response for an already parsed request and applies server policies.
pub(crate) fn build_response_for_request(
    server: &Server,
    request: &Request,
) -> Response {
    let generated = match request.method() {
        "GET" => generate_response_with_cache(
            request,
            &server.document_root,
            &server.canonical_document_root,
            server.static_cache_ttl_secs,
        ),
        "HEAD" => {
            generate_head_response(request, &server.document_root)
        }
        "OPTIONS" => generate_options_response(request),
        _ => Ok(generate_method_not_allowed_response()),
    };
    match generated {
        Ok(response) => {
            apply_response_policies(response, server, request)
        }
        Err(error) => {
            response_from_error(&error, &server.document_root)
        }
    }
}

fn record_metrics(response: &Response) {
    let _ = METRIC_REQUESTS_TOTAL.fetch_add(1, Ordering::Relaxed);
    if (400..500).contains(&response.status_code) {
        let _ = METRIC_RESPONSES_4XX.fetch_add(1, Ordering::Relaxed);
    } else if response.status_code >= 500 {
        let _ = METRIC_RESPONSES_5XX.fetch_add(1, Ordering::Relaxed);
    }
}

fn generate_metrics_response() -> Response {
    let body = format!(
        "http_handle_requests_total {}\nhttp_handle_responses_4xx_total {}\nhttp_handle_responses_5xx_total {}\nhttp_handle_rate_limited_total {}\n",
        METRIC_REQUESTS_TOTAL.load(Ordering::Relaxed),
        METRIC_RESPONSES_4XX.load(Ordering::Relaxed),
        METRIC_RESPONSES_5XX.load(Ordering::Relaxed),
        METRIC_RATE_LIMITED.load(Ordering::Relaxed),
    );
    let mut response = Response::new(200, "OK", body.into_bytes());
    response.add_header("Content-Type", "text/plain; version=0.0.3");
    response
}

fn generate_too_many_requests_response() -> Response {
    let mut response = Response::new(
        429,
        "TOO MANY REQUESTS",
        b"Rate limit exceeded".to_vec(),
    );
    response.add_header("Content-Type", "text/plain");
    response.add_header("Retry-After", "60");
    response
}

fn is_rate_limited(server: &Server, ip: IpAddr) -> bool {
    let Some(limit) = server.rate_limit_per_minute else {
        return false;
    };
    let now = Instant::now();
    let state =
        RATE_LIMIT_STATE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let hits = guard.entry(ip).or_default();
    hits.retain(|timestamp| {
        now.duration_since(*timestamp) <= Duration::from_secs(60)
    });
    if hits.len() >= limit {
        return true;
    }
    hits.push(now);
    false
}

/// Generates an HTTP response based on the requested file.
///
/// # Arguments
///
/// * `request` - A `Request` instance representing the client's request.
/// * `document_root` - A `Path` representing the server's document root.
///
/// # Returns
///
/// A `Result` containing the `Response` or a `ServerError`.
fn generate_response(
    request: &Request,
    document_root: &Path,
) -> Result<Response, ServerError> {
    // Fallback entry point used only by tests: canonicalize lazily.
    let canonical = fs::canonicalize(document_root)
        .unwrap_or_else(|_| document_root.to_path_buf());
    generate_response_with_cache(
        request,
        document_root,
        &canonical,
        None,
    )
}

fn generate_response_with_cache(
    request: &Request,
    document_root: &Path,
    canonical_root: &Path,
    cache_ttl_secs: Option<u64>,
) -> Result<Response, ServerError> {
    let mut path = PathBuf::from(document_root);
    let request_path = request.path().trim_start_matches('/');

    if request_path.is_empty() {
        // If the request is for the root, append "index.html"
        path.push("index.html");
    } else {
        for component in request_path.split('/') {
            if component == ".." {
                let _ = path.pop();
            } else {
                path.push(component);
            }
        }
    }

    let within_root = fs::canonicalize(&path)
        .map(|candidate| candidate.starts_with(canonical_root))
        .unwrap_or_else(|_| path.starts_with(document_root));
    if !within_root {
        return Err(ServerError::forbidden("Access denied"));
    }

    if path.is_file() {
        serve_file_response(request, &path, cache_ttl_secs)
    } else if path.is_dir() {
        // If it's a directory, try to serve index.html from that directory
        path.push("index.html");
        if path.is_file() {
            serve_file_response(request, &path, cache_ttl_secs)
        } else {
            generate_404_response(document_root)
        }
    } else {
        generate_404_response(document_root)
    }
}

fn serve_file_response(
    request: &Request,
    path: &Path,
    cache_ttl_secs: Option<u64>,
) -> Result<Response, ServerError> {
    let mut serving_path = path.to_path_buf();
    let mut content_encoding: Option<&'static str> = None;
    if let Some(encoding) = request.header("accept-encoding") {
        if encoding.contains("br") {
            let candidate =
                PathBuf::from(format!("{}.br", path.display()));
            if candidate.is_file() {
                serving_path = candidate;
                content_encoding = Some("br");
            }
        }
        if content_encoding.is_none()
            && (encoding.contains("zstd") || encoding.contains("zst"))
        {
            let candidate =
                PathBuf::from(format!("{}.zst", path.display()));
            if candidate.is_file() {
                serving_path = candidate;
                content_encoding = Some("zstd");
            }
        }
        if content_encoding.is_none() && encoding.contains("gzip") {
            let candidate =
                PathBuf::from(format!("{}.gz", path.display()));
            if candidate.is_file() {
                serving_path = candidate;
                content_encoding = Some("gzip");
            }
        }
    }

    let contents = fs::read(&serving_path)?;
    let metadata = fs::metadata(path)?;
    let etag = compute_etag(&metadata);
    if request
        .header("if-none-match")
        .is_some_and(|candidate| candidate == etag)
    {
        let mut response =
            Response::new(304, "NOT MODIFIED", Vec::new());
        response.add_header("ETag", &etag);
        return Ok(response);
    }

    let content_type = get_content_type(path);
    let mut response = if let Some((start, end)) =
        parse_range_header(request.header("range"), contents.len())
    {
        let body = contents[start..=end].to_vec();
        let mut partial = Response::new(206, "PARTIAL CONTENT", body);
        partial.add_header(
            "Content-Range",
            &format!("bytes {}-{}/{}", start, end, contents.len()),
        );
        partial
    } else {
        Response::new(200, "OK", contents)
    };

    response.add_header("Content-Type", content_type);
    response.add_header("ETag", &etag);
    response.add_header("Accept-Ranges", "bytes");
    if let Some(encoding) = content_encoding {
        response.add_header("Content-Encoding", encoding);
        response.add_header("Vary", "Accept-Encoding");
    }
    if let Some(ttl) = cache_ttl_secs {
        response.add_header(
            "Cache-Control",
            &format!("public, max-age={ttl}"),
        );
    }
    Ok(response)
}

fn compute_etag(metadata: &fs::Metadata) -> String {
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0_u64, |duration| duration.as_secs());
    format!("W/\"{:x}-{:x}\"", metadata.len(), modified)
}

fn parse_range_header(
    header: Option<&str>,
    total_len: usize,
) -> Option<(usize, usize)> {
    let header = header?;
    let value = header.strip_prefix("bytes=")?;
    let (start_str, end_str) = value.split_once('-')?;
    if start_str.is_empty() && end_str.is_empty() {
        return None;
    }
    if start_str.is_empty() {
        let suffix_len = end_str.parse::<usize>().ok()?;
        if suffix_len == 0 || suffix_len > total_len {
            return None;
        }
        return Some((total_len - suffix_len, total_len - 1));
    }
    let start = start_str.parse::<usize>().ok()?;
    let end = if end_str.is_empty() {
        total_len.checked_sub(1)?
    } else {
        end_str.parse::<usize>().ok()?
    };
    if start > end || end >= total_len {
        return None;
    }
    Some((start, end))
}

/// Generates a 404 Not Found response.
///
/// # Arguments
///
/// * `document_root` - A `Path` representing the server's document root.
///
/// # Returns
///
/// A `Result` containing the `Response` or a `ServerError`.
fn generate_404_response(
    document_root: &Path,
) -> Result<Response, ServerError> {
    let not_found_path = document_root.join("404/index.html");
    let contents = if not_found_path.is_file() {
        fs::read(not_found_path)?
    } else {
        b"404 Not Found".to_vec()
    };
    let mut response = Response::new(404, "NOT FOUND", contents);
    response.add_header("Content-Type", "text/html");
    Ok(response)
}

/// Generates an HTTP HEAD response based on the requested file.
///
/// HEAD responses are identical to GET responses but without the message body.
/// The response headers, including Content-Length, must be identical to what
/// would be sent for a GET request to the same resource.
///
/// # Arguments
///
/// * `request` - A `Request` instance representing the client's request.
/// * `document_root` - A `Path` representing the server's document root.
///
/// # Returns
///
/// A `Result` containing the `Response` or a `ServerError`.
fn generate_head_response(
    request: &Request,
    document_root: &Path,
) -> Result<Response, ServerError> {
    // Generate the full response as if it were a GET request
    let full_response = generate_response(request, document_root)?;

    // Create a new response with the same status and headers but empty body
    let mut head_response = Response::new(
        full_response.status_code,
        &full_response.status_text,
        Vec::new(), // Empty body for HEAD response
    );

    // Copy all headers from the full response
    for (name, value) in &full_response.headers {
        head_response.add_header(name, value);
    }

    // Add Content-Length header to match what would be sent in GET response
    let content_length = full_response.body.len().to_string();
    head_response.add_header("Content-Length", &content_length);

    Ok(head_response)
}

/// Generates an HTTP OPTIONS response indicating supported methods.
///
/// The OPTIONS method is used to describe the communication options for the target resource.
/// This implementation returns the allowed HTTP methods for any requested resource.
///
/// # Arguments
///
/// * `request` - A `Request` instance representing the client's request.
///
/// # Returns
///
/// A `Response` instance with allowed methods.
fn generate_options_response(
    _request: &Request,
) -> Result<Response, ServerError> {
    let mut response = Response::new(200, "OK", Vec::new());
    response.add_header("Allow", "GET, HEAD, OPTIONS");
    response.add_header("Content-Length", "0");
    Ok(response)
}

/// Generates a 405 Method Not Allowed response.
///
/// This response is sent when the client uses an HTTP method that is not
/// supported by the server for the requested resource.
///
/// # Returns
///
/// A `Response` instance indicating the method is not allowed.
fn generate_method_not_allowed_response() -> Response {
    let mut response = Response::new(
        405,
        "METHOD NOT ALLOWED",
        b"Method Not Allowed".to_vec(),
    );
    response.add_header("Allow", "GET, HEAD, OPTIONS");
    response.add_header("Content-Type", "text/plain");
    response.add_header("Content-Length", "18");
    response
}

fn response_from_error(
    error: &ServerError,
    document_root: &Path,
) -> Response {
    match error {
        ServerError::InvalidRequest(message) => {
            let mut response = Response::new(
                400,
                "BAD REQUEST",
                message.as_bytes().to_vec(),
            );
            response.add_header("Content-Type", "text/plain");
            response
        }
        ServerError::Forbidden(message) => {
            let mut response = Response::new(
                403,
                "FORBIDDEN",
                message.as_bytes().to_vec(),
            );
            response.add_header("Content-Type", "text/plain");
            response
        }
        ServerError::NotFound(_) => {
            generate_404_response(document_root).unwrap_or_else(|_| {
                let mut response = Response::new(
                    404,
                    "NOT FOUND",
                    b"404 Not Found".to_vec(),
                );
                response.add_header("Content-Type", "text/plain");
                response
            })
        }
        ServerError::Io(_)
        | ServerError::Custom(_)
        | ServerError::TaskFailed(_) => {
            let mut response = Response::new(
                500,
                "INTERNAL SERVER ERROR",
                b"Internal Server Error".to_vec(),
            );
            response.add_header("Content-Type", "text/plain");
            response
        }
    }
}

fn apply_response_policies(
    mut response: Response,
    server: &Server,
    request: &Request,
) -> Response {
    if let Some(headers) = server.custom_headers.as_ref() {
        for (name, value) in headers {
            response.add_header(name, value);
        }
    }

    if server.cors_enabled.unwrap_or(false) {
        let allow_origin = server
            .cors_origins
            .as_ref()
            .and_then(|origins| origins.first())
            .map(String::as_str)
            .unwrap_or("*");
        response
            .add_header("Access-Control-Allow-Origin", allow_origin);
        response.add_header(
            "Access-Control-Allow-Methods",
            "GET, HEAD, OPTIONS",
        );
        response.add_header("Access-Control-Allow-Headers", "*");

        if request.method().eq_ignore_ascii_case("OPTIONS") {
            response.add_header("Access-Control-Max-Age", "600");
        }
    }

    if let Some(ttl) = server.static_cache_ttl_secs {
        let has_cache_control =
            response.headers.iter().any(|(name, _)| {
                name.eq_ignore_ascii_case("cache-control")
            });
        if !has_cache_control {
            if is_probably_immutable_asset_path(request.path()) {
                response.add_header(
                    "Cache-Control",
                    "public, max-age=31536000, immutable",
                );
            } else {
                response.add_header(
                    "Cache-Control",
                    &format!("public, max-age={ttl}"),
                );
            }
        }
    }

    response
}

fn is_probably_immutable_asset_path(path: &str) -> bool {
    let file = path.rsplit('/').next().unwrap_or(path);
    let Some((stem, _ext)) = file.rsplit_once('.') else {
        return false;
    };
    let Some(hash) = stem.rsplit('-').next() else {
        return false;
    };
    hash.len() >= 8 && hash.chars().all(|c| c.is_ascii_hexdigit())
}

/// Determines the content type based on the file extension.
///
/// # Arguments
///
/// * `path` - A `Path` representing the file path.
///
/// # Returns
///
/// A string slice representing the content type.
fn get_content_type(path: &Path) -> &'static str {
    match path.extension().and_then(std::ffi::OsStr::to_str) {
        // Text formats
        Some("html") | Some("htm") => "text/html",
        Some("css") => "text/css",
        Some("js") | Some("mjs") => "application/javascript",
        Some("ts") => "application/typescript",
        Some("json") => "application/json",
        Some("xml") => "application/xml",
        Some("txt") => "text/plain",
        Some("md") | Some("markdown") => "text/markdown",
        Some("yaml") | Some("yml") => "application/x-yaml",
        Some("toml") => "application/toml",

        // Traditional image formats
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",

        // Modern image formats
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("heic") | Some("heif") => "image/heic",
        Some("jxl") => "image/jxl",
        Some("bmp") => "image/bmp",
        Some("tiff") | Some("tif") => "image/tiff",

        // Web Assembly
        Some("wasm") => "application/wasm",

        // Font formats
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("eot") => "application/vnd.ms-fontobject",

        // Audio formats
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("ogg") => "audio/ogg",
        Some("opus") => "audio/opus",
        Some("flac") => "audio/flac",
        Some("m4a") => "audio/mp4",
        Some("aac") => "audio/aac",

        // Video formats
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("av1") => "video/av1",
        Some("avi") => "video/x-msvideo",
        Some("mov") => "video/quicktime",

        // Document formats
        Some("pdf") => "application/pdf",
        Some("zip") => "application/zip",
        Some("tar") => "application/x-tar",
        Some("gz") => "application/gzip",

        // Development formats
        Some("map") => "application/json", // Source maps
        Some("webmanifest") => "application/manifest+json",

        // Default fallback
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io;
    use std::io::Read;
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use tempfile::TempDir;

    fn setup_test_directory() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create index.html
        let mut index_file =
            File::create(root_path.join("index.html")).unwrap();
        index_file
            .write_all(b"<html><body>Hello, World!</body></html>")
            .unwrap();

        // Create 404/index.html
        fs::create_dir(root_path.join("404")).unwrap();
        let mut not_found_file =
            File::create(root_path.join("404/index.html")).unwrap();
        not_found_file
            .write_all(b"<html><body>404 Not Found</body></html>")
            .unwrap();

        // Create a subdirectory with its own index.html
        fs::create_dir(root_path.join("subdir")).unwrap();
        let mut subdir_index_file =
            File::create(root_path.join("subdir/index.html")).unwrap();
        subdir_index_file
            .write_all(b"<html><body>Subdirectory Index</body></html>")
            .unwrap();

        temp_dir
    }

    fn roundtrip_handle_connection(
        server: &Server,
        request: &[u8],
    ) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let server_clone = server.clone();
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            handle_connection(stream, &server_clone).expect("handle");
        });

        let mut client = TcpStream::connect(addr).expect("connect");
        client.write_all(request).expect("write");
        let mut response = String::new();
        let _ = client.read_to_string(&mut response).expect("read");
        handle.join().expect("join");
        response
    }

    fn connected_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let client = TcpStream::connect(addr).expect("connect");
        let (server, _) = listener.accept().expect("accept");
        (server, client)
    }

    #[test]
    fn test_server_creation() {
        let server = Server::new("127.0.0.1:8080", "/var/www");
        assert_eq!(server.address, "127.0.0.1:8080");
        assert_eq!(server.document_root, PathBuf::from("/var/www"));
    }

    #[test]
    fn test_get_content_type() {
        assert_eq!(
            get_content_type(Path::new("test.html")),
            "text/html"
        );
        assert_eq!(
            get_content_type(Path::new("page.htm")),
            "text/html"
        );
        assert_eq!(
            get_content_type(Path::new("style.css")),
            "text/css"
        );
        assert_eq!(
            get_content_type(Path::new("script.js")),
            "application/javascript"
        );
        assert_eq!(
            get_content_type(Path::new("data.json")),
            "application/json"
        );
        assert_eq!(
            get_content_type(Path::new("image.png")),
            "image/png"
        );
        assert_eq!(
            get_content_type(Path::new("photo.jpg")),
            "image/jpeg"
        );
        assert_eq!(
            get_content_type(Path::new("animation.gif")),
            "image/gif"
        );
        assert_eq!(
            get_content_type(Path::new("icon.svg")),
            "image/svg+xml"
        );
        assert_eq!(
            get_content_type(Path::new("unknown.xyz")),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_modern_content_types() {
        // Test modern image formats
        assert_eq!(
            get_content_type(Path::new("image.webp")),
            "image/webp"
        );
        assert_eq!(
            get_content_type(Path::new("image.avif")),
            "image/avif"
        );
        assert_eq!(
            get_content_type(Path::new("image.heic")),
            "image/heic"
        );
        assert_eq!(
            get_content_type(Path::new("image.heif")),
            "image/heic"
        );
        assert_eq!(
            get_content_type(Path::new("image.jxl")),
            "image/jxl"
        );

        // Test Web Assembly
        assert_eq!(
            get_content_type(Path::new("module.wasm")),
            "application/wasm"
        );

        // Test modern text formats
        assert_eq!(
            get_content_type(Path::new("script.ts")),
            "application/typescript"
        );
        assert_eq!(
            get_content_type(Path::new("module.mjs")),
            "application/javascript"
        );
        assert_eq!(
            get_content_type(Path::new("README.md")),
            "text/markdown"
        );
        assert_eq!(
            get_content_type(Path::new("config.yaml")),
            "application/x-yaml"
        );
        assert_eq!(
            get_content_type(Path::new("config.yml")),
            "application/x-yaml"
        );
        assert_eq!(
            get_content_type(Path::new("Cargo.toml")),
            "application/toml"
        );

        // Test modern audio formats
        assert_eq!(
            get_content_type(Path::new("audio.opus")),
            "audio/opus"
        );
        assert_eq!(
            get_content_type(Path::new("audio.flac")),
            "audio/flac"
        );

        // Test modern video formats
        assert_eq!(
            get_content_type(Path::new("video.av1")),
            "video/av1"
        );

        // Test development formats
        assert_eq!(
            get_content_type(Path::new("script.js.map")),
            "application/json"
        );
        assert_eq!(
            get_content_type(Path::new("manifest.webmanifest")),
            "application/manifest+json"
        );
    }

    #[test]
    fn test_generate_response() {
        let temp_dir = setup_test_directory();
        let document_root = temp_dir.path();

        // Test root request (should serve index.html)
        let root_request = Request {
            method: "GET".to_string(),
            path: "/".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
        };

        let root_response =
            generate_response(&root_request, document_root).unwrap();
        assert_eq!(root_response.status_code, 200);
        assert_eq!(root_response.status_text, "OK");
        assert!(
            root_response.body.starts_with(
                b"<html><body>Hello, World!</body></html>"
            )
        );

        // Test specific file request
        let file_request = Request {
            method: "GET".to_string(),
            path: "/index.html".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
        };

        let file_response =
            generate_response(&file_request, document_root).unwrap();
        assert_eq!(file_response.status_code, 200);
        assert_eq!(file_response.status_text, "OK");
        assert!(
            file_response.body.starts_with(
                b"<html><body>Hello, World!</body></html>"
            )
        );

        // Test subdirectory index request
        let subdir_request = Request {
            method: "GET".to_string(),
            path: "/subdir/".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
        };

        let subdir_response =
            generate_response(&subdir_request, document_root).unwrap();
        assert_eq!(subdir_response.status_code, 200);
        assert_eq!(subdir_response.status_text, "OK");
        assert!(subdir_response.body.starts_with(
            b"<html><body>Subdirectory Index</body></html>"
        ));

        // Test non-existent file request
        let not_found_request = Request {
            method: "GET".to_string(),
            path: "/nonexistent.html".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
        };

        let not_found_response =
            generate_response(&not_found_request, document_root)
                .unwrap();
        assert_eq!(not_found_response.status_code, 404);
        assert_eq!(not_found_response.status_text, "NOT FOUND");
        assert!(
            not_found_response.body.starts_with(
                b"<html><body>404 Not Found</body></html>"
            )
        );

        // Test directory traversal attempt
        let traversal_request = Request {
            method: "GET".to_string(),
            path: "/../outside.html".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
        };

        let traversal_response =
            generate_response(&traversal_request, document_root);
        assert!(matches!(
            traversal_response,
            Err(ServerError::Forbidden(_))
        ));
    }

    #[test]
    fn test_server_builder() {
        // Test basic ServerBuilder usage
        let server = Server::builder()
            .address("127.0.0.1:8080")
            .document_root("/var/www")
            .enable_cors()
            .custom_header("X-Custom", "test-value")
            .request_timeout(Duration::from_secs(30))
            .build()
            .unwrap();

        assert_eq!(server.address, "127.0.0.1:8080");
        assert_eq!(server.document_root, PathBuf::from("/var/www"));
        assert_eq!(server.cors_enabled, Some(true));
        assert_eq!(
            server.request_timeout,
            Some(Duration::from_secs(30))
        );

        // Check custom headers
        let headers = server.custom_headers.unwrap();
        assert_eq!(
            headers.get("X-Custom"),
            Some(&"test-value".to_string())
        );

        // Test builder error handling
        let result = ServerBuilder::new().build();
        assert!(result.is_err());

        // Test CORS origins setting
        let server_with_origins = Server::builder()
            .address("127.0.0.1:9000")
            .document_root("/tmp")
            .cors_origins(vec!["https://example.com".to_string()])
            .build()
            .unwrap();

        assert_eq!(server_with_origins.cors_enabled, Some(true));
        assert_eq!(
            server_with_origins.cors_origins,
            Some(vec!["https://example.com".to_string()])
        );
    }

    #[test]
    fn test_graceful_shutdown() {
        // Test ShutdownSignal creation and default behavior
        let shutdown = ShutdownSignal::new(Duration::from_secs(5));

        // Initially no shutdown should be requested
        assert!(!shutdown.is_shutdown_requested());
        assert_eq!(shutdown.active_connection_count(), 0);

        // Test connection tracking
        shutdown.connection_started();
        assert_eq!(shutdown.active_connection_count(), 1);

        shutdown.connection_started();
        assert_eq!(shutdown.active_connection_count(), 2);

        shutdown.connection_finished();
        assert_eq!(shutdown.active_connection_count(), 1);

        shutdown.connection_finished();
        assert_eq!(shutdown.active_connection_count(), 0);

        // Test shutdown signal
        shutdown.shutdown();
        assert!(shutdown.is_shutdown_requested());

        // Test immediate shutdown when no active connections
        let graceful = shutdown.wait_for_shutdown();
        assert!(graceful);
    }

    #[test]
    fn test_shutdown_signal_timeout() {
        let shutdown = ShutdownSignal::new(Duration::from_millis(100));

        // Start a connection and request shutdown
        shutdown.connection_started();
        shutdown.shutdown();

        // Should timeout since connection never finishes
        let graceful = shutdown.wait_for_shutdown();
        assert!(!graceful); // Should be false due to timeout
    }

    #[test]
    fn test_thread_pool() {
        use std::sync::Arc;
        use std::sync::atomic::AtomicUsize;
        use std::sync::mpsc;

        let pool = ThreadPool::new(4);
        let counter = Arc::new(AtomicUsize::new(0));
        let (tx, rx) = mpsc::channel();

        // Execute 10 jobs
        for _ in 0..10 {
            let counter_clone = Arc::clone(&counter);
            let tx_clone = tx.clone();

            pool.execute(move || {
                let _ = counter_clone.fetch_add(1, Ordering::SeqCst);
                tx_clone.send(()).unwrap();
            });
        }

        // Wait for all jobs to complete
        for _ in 0..10 {
            rx.recv().unwrap();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_connection_pool() {
        let pool = ConnectionPool::new(2);
        assert_eq!(pool.active_count(), 0);

        // Acquire first connection
        let guard1 = pool.acquire().unwrap();
        assert_eq!(pool.active_count(), 1);

        // Acquire second connection
        let guard2 = pool.acquire().unwrap();
        assert_eq!(pool.active_count(), 2);

        // Try to acquire third connection (should fail)
        let result = pool.acquire();
        assert!(result.is_err());
        assert_eq!(pool.active_count(), 2);

        // Drop first connection
        drop(guard1);
        assert_eq!(pool.active_count(), 1);

        // Should be able to acquire again
        let _guard3 = pool.acquire().unwrap();
        assert_eq!(pool.active_count(), 2);

        // Drop all connections
        drop(guard2);
        drop(_guard3);
        assert_eq!(pool.active_count(), 0);
    }

    #[test]
    fn test_thread_pool_concurrent_execution() {
        use std::sync::Arc;
        use std::sync::atomic::AtomicUsize;
        use std::sync::mpsc;
        use std::time::Instant;

        let pool = ThreadPool::new(4);
        let counter = Arc::new(AtomicUsize::new(0));
        let (tx, rx) = mpsc::channel();

        let start_time = Instant::now();

        // Execute 100 jobs that should be processed concurrently
        for i in 0..100 {
            let counter_clone = Arc::clone(&counter);
            let tx_clone = tx.clone();

            pool.execute(move || {
                // Simulate some work
                thread::sleep(Duration::from_millis(10));
                let _ = counter_clone.fetch_add(1, Ordering::SeqCst);
                tx_clone.send(i).unwrap();
            });
        }

        // Wait for all jobs to complete
        for _ in 0..100 {
            let _ = rx.recv().unwrap();
        }

        let elapsed = start_time.elapsed();
        assert_eq!(counter.load(Ordering::SeqCst), 100);

        // With 4 threads, 100 jobs of 10ms each should complete much faster than 1000ms
        assert!(
            elapsed.as_millis() < 800,
            "Thread pool should provide concurrency benefits"
        );
    }

    #[test]
    fn test_connection_pool_backpressure() {
        let pool = ConnectionPool::new(2);

        // Acquire maximum connections
        let _guard1 = pool.acquire().unwrap();
        let _guard2 = pool.acquire().unwrap();
        assert_eq!(pool.active_count(), 2);

        // Additional connection should be rejected
        let result = pool.acquire();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
    }

    #[test]
    fn test_service_unavailable_response() {
        // Create a test TCP connection
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let _ = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            send_service_unavailable(stream).unwrap();
        });

        let mut client_stream = TcpStream::connect(addr).unwrap();
        let mut response = String::new();
        let _ = client_stream.read_to_string(&mut response).unwrap();

        assert!(response.contains("503"));
        assert!(response.contains("SERVICE UNAVAILABLE"));
        assert!(response.contains("Service temporarily unavailable"));
        assert!(response.contains("Retry-After: 1"));
    }

    #[test]
    fn test_service_unavailable_send_failure_is_mapped() {
        use std::net::Shutdown;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");

        let t = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            stream.shutdown(Shutdown::Write).expect("shutdown");
            let err =
                send_service_unavailable(stream).expect_err("err");
            assert!(
                err.to_string().contains("Failed to send response")
            );
        });

        let _client = TcpStream::connect(addr).expect("connect");
        t.join().expect("join");
    }

    #[test]
    fn test_response_from_error_variants() {
        let temp_dir = setup_test_directory();
        let root = temp_dir.path();

        let bad = response_from_error(
            &ServerError::InvalidRequest("bad".to_string()),
            root,
        );
        assert_eq!(bad.status_code, 400);

        let forbidden = response_from_error(
            &ServerError::Forbidden("no".to_string()),
            root,
        );
        assert_eq!(forbidden.status_code, 403);

        let not_found = response_from_error(
            &ServerError::NotFound("missing".to_string()),
            root,
        );
        assert_eq!(not_found.status_code, 404);

        let internal = response_from_error(
            &ServerError::TaskFailed("boom".to_string()),
            root,
        );
        assert_eq!(internal.status_code, 500);
    }

    #[test]
    fn test_apply_response_policies_with_cors_and_headers() {
        let mut headers = HashMap::new();
        let _ = headers
            .insert("X-App".to_string(), "http-handle".to_string());
        let server = Server::builder()
            .address("127.0.0.1:0")
            .document_root(".")
            .enable_cors()
            .cors_origins(vec!["https://example.com".to_string()])
            .custom_headers(headers)
            .build()
            .expect("server builder");

        let request = Request {
            method: "OPTIONS".to_string(),
            path: "/".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
        };
        let response = apply_response_policies(
            Response::new(200, "OK", Vec::new()),
            &server,
            &request,
        );

        let has_origin = response.headers.iter().any(|(k, v)| {
            k.eq_ignore_ascii_case("Access-Control-Allow-Origin")
                && v == "https://example.com"
        });
        let has_custom = response
            .headers
            .iter()
            .any(|(k, v)| k == "X-App" && v == "http-handle");
        let has_max_age = response.headers.iter().any(|(k, _)| {
            k.eq_ignore_ascii_case("Access-Control-Max-Age")
        });

        assert!(has_origin);
        assert!(has_custom);
        assert!(has_max_age);
    }

    #[test]
    fn test_thread_pool_debug_representation() {
        let pool = ThreadPool::new(1);
        let rendered = format!("{pool:?}");
        assert!(rendered.contains("ThreadPool"));
        assert!(rendered.contains("<Sender<Job>>"));
    }

    #[test]
    fn test_server_getters_expose_builder_config() {
        let mut headers = HashMap::new();
        let _ =
            headers.insert("X-Test".to_string(), "value".to_string());

        let server = Server::builder()
            .address("127.0.0.1:9001")
            .document_root("/tmp")
            .enable_cors()
            .cors_origins(vec!["https://example.com".to_string()])
            .custom_headers(headers)
            .request_timeout(Duration::from_secs(5))
            .connection_timeout(Duration::from_secs(7))
            .build()
            .expect("server");

        assert_eq!(server.cors_enabled(), Some(true));
        assert_eq!(
            server.cors_origins(),
            &Some(vec!["https://example.com".to_string()])
        );
        assert_eq!(
            server.request_timeout(),
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            server.connection_timeout(),
            Some(Duration::from_secs(7))
        );
        assert_eq!(server.address(), "127.0.0.1:9001");
        assert_eq!(server.document_root(), &PathBuf::from("/tmp"));
        assert_eq!(
            server
                .custom_headers()
                .as_ref()
                .and_then(|h| h.get("X-Test")),
            Some(&"value".to_string())
        );
    }

    #[test]
    fn test_start_variants_return_bind_errors_for_in_use_address() {
        let occupied = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = occupied.local_addr().expect("addr").to_string();
        let server = Server::new(&addr, ".");

        assert!(server.start().is_err());
        assert!(
            server
                .start_with_graceful_shutdown(Duration::from_millis(1))
                .is_err()
        );
        assert!(server.start_with_thread_pool(1).is_err());
        assert!(server.start_with_pooling(1, 1).is_err());
    }

    #[test]
    fn test_start_with_shutdown_signal_and_ready_reports_bound_address()
    {
        let root = setup_test_directory();
        let server = Server::builder()
            .address("127.0.0.1:0")
            .document_root(root.path().to_str().expect("path"))
            .build()
            .expect("server");

        let (ready_tx, ready_rx) = mpsc::channel::<String>();
        let shutdown =
            Arc::new(ShutdownSignal::new(Duration::from_secs(2)));
        let shutdown_for_server = shutdown.clone();
        let server_for_thread = server.clone();

        let handle = thread::spawn(move || {
            server_for_thread
                .start_with_shutdown_signal_and_ready(
                    shutdown_for_server,
                    move |addr| {
                        let _ = ready_tx.send(addr);
                    },
                )
                .expect("server run");
        });

        let bound_addr = ready_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("bound address");
        assert!(bound_addr.starts_with("127.0.0.1:"));
        assert_ne!(bound_addr, "127.0.0.1:0");

        let mut stream =
            TcpStream::connect(&bound_addr).expect("connect");
        stream
            .write_all(
                b"GET /index.html HTTP/1.1\r\nHost: localhost\r\n\r\n",
            )
            .expect("write");
        let mut response = String::new();
        let _ = stream.read_to_string(&mut response);
        assert!(response.starts_with("HTTP/1.1 200 OK"));

        shutdown.shutdown();
        handle.join().expect("join");
    }

    #[test]
    fn test_generate_response_falls_back_to_builtin_404_without_page() {
        let temp_dir = TempDir::new().expect("tmp");
        fs::write(temp_dir.path().join("index.html"), b"index")
            .expect("write");
        fs::create_dir(temp_dir.path().join("empty-dir")).expect("dir");

        let request = Request {
            method: "GET".to_string(),
            path: "/empty-dir/".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
        };

        let response = generate_response(&request, temp_dir.path())
            .expect("response");
        assert_eq!(response.status_code, 404);
        assert_eq!(response.body, b"404 Not Found".to_vec());
    }

    #[cfg(unix)]
    #[test]
    fn test_response_from_error_not_found_fallback_when_custom_404_unreadable()
     {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().expect("tmp");
        let custom_404_dir = temp_dir.path().join("404");
        fs::create_dir(&custom_404_dir).expect("create 404 dir");
        let custom_404 = custom_404_dir.join("index.html");
        fs::write(&custom_404, b"custom").expect("write 404");
        fs::set_permissions(
            &custom_404,
            fs::Permissions::from_mode(0o000),
        )
        .expect("chmod");

        let response = response_from_error(
            &ServerError::NotFound("missing".to_string()),
            temp_dir.path(),
        );

        assert_eq!(response.status_code, 404);
        assert_eq!(response.status_text, "NOT FOUND");
        assert_eq!(response.body, b"404 Not Found".to_vec());
    }

    #[test]
    fn test_handle_connection_options_and_parse_error_paths() {
        let root = setup_test_directory();
        let root_str = root.path().to_str().expect("root path");
        let server = Server::builder()
            .address("127.0.0.1:0")
            .document_root(root_str)
            .build()
            .expect("server");

        let options_response = roundtrip_handle_connection(
            &server,
            b"OPTIONS / HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );
        assert!(options_response.starts_with("HTTP/1.1 200 OK"));
        assert!(options_response.contains("Allow: GET, HEAD, OPTIONS"));

        let head_response = roundtrip_handle_connection(
            &server,
            b"HEAD / HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );
        assert!(head_response.starts_with("HTTP/1.1 200 OK"));
        assert!(head_response.contains("Content-Length:"));

        let method_not_allowed = roundtrip_handle_connection(
            &server,
            b"POST / HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );
        assert!(
            method_not_allowed
                .starts_with("HTTP/1.1 405 METHOD NOT ALLOWED")
        );

        let traversal_response = roundtrip_handle_connection(
            &server,
            b"GET /../outside HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );
        assert!(
            traversal_response.starts_with("HTTP/1.1 403 FORBIDDEN")
        );

        let bad_response =
            roundtrip_handle_connection(&server, b"BAD\r\n\r\n");
        assert!(bad_response.starts_with("HTTP/1.1 400 BAD REQUEST"));
    }

    #[test]
    fn test_generate_response_supports_etag_and_range() {
        let temp_dir = setup_test_directory();
        let root = temp_dir.path();

        let mut headers = HashMap::new();
        let _ = headers
            .insert("range".to_string(), "bytes=0-4".to_string());
        let range_request = Request {
            method: "GET".to_string(),
            path: "/index.html".to_string(),
            version: "HTTP/1.1".to_string(),
            headers,
        };
        let range_response =
            generate_response(&range_request, root).expect("range");
        assert_eq!(range_response.status_code, 206);
        assert!(range_response.body.starts_with(b"<html"));
        let etag = range_response
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("etag"))
            .map(|(_, value)| value.clone())
            .expect("etag");

        let mut headers = HashMap::new();
        let _ = headers.insert("if-none-match".to_string(), etag);
        let conditional_request = Request {
            method: "GET".to_string(),
            path: "/index.html".to_string(),
            version: "HTTP/1.1".to_string(),
            headers,
        };
        let conditional_response =
            generate_response(&conditional_request, root)
                .expect("conditional");
        assert_eq!(conditional_response.status_code, 304);
        assert!(conditional_response.body.is_empty());
    }

    #[test]
    fn test_metrics_and_rate_limiting() {
        let root = setup_test_directory();
        let server = Server::builder()
            .address("127.0.0.1:0")
            .document_root(root.path().to_str().expect("path"))
            .rate_limit_per_minute(1)
            .build()
            .expect("server");

        let _ = roundtrip_handle_connection(
            &server,
            b"GET /index.html HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );
        let limited = roundtrip_handle_connection(
            &server,
            b"GET /index.html HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );
        assert!(limited.starts_with("HTTP/1.1 429 TOO MANY REQUESTS"));

        let metrics = roundtrip_handle_connection(
            &server,
            b"GET /metrics HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );
        assert!(metrics.starts_with("HTTP/1.1 200 OK"));
        assert!(metrics.contains("http_handle_requests_total"));
    }

    #[test]
    fn test_trigger_shutdown_from_slot_helper() {
        let shutdown =
            Arc::new(ShutdownSignal::new(Duration::from_secs(1)));
        let slot = Mutex::new(Some(shutdown.clone()));
        assert!(!shutdown.is_shutdown_requested());
        Server::trigger_shutdown_from_slot(&slot);
        assert!(shutdown.is_shutdown_requested());
    }

    #[test]
    fn test_handle_shutdown_signal_helper() {
        let shutdown =
            Arc::new(ShutdownSignal::new(Duration::from_secs(1)));
        let slot =
            SHUTDOWN_SIGNAL_SLOT.get_or_init(|| Mutex::new(None));
        if let Ok(mut guard) = slot.lock() {
            *guard = Some(shutdown.clone());
        }
        Server::handle_shutdown_signal();
        assert!(shutdown.is_shutdown_requested());
    }

    #[test]
    fn test_accept_loop_helpers_cover_ok_and_err_paths() {
        let root = setup_test_directory();
        let server = Server::builder()
            .address("127.0.0.1:0")
            .document_root(root.path().to_str().expect("path"))
            .build()
            .expect("server");

        Server::run_basic_accept_loop(
            vec![Err(io::Error::other("incoming failed"))],
            server.clone(),
        );
        let pool = ThreadPool::new(1);
        Server::run_thread_pool_accept_loop(
            vec![Err(io::Error::other("incoming failed"))],
            server.clone(),
            &pool,
        );
        Server::run_pooling_accept_loop(
            vec![Err(io::Error::other("incoming failed"))],
            server.clone(),
            &pool,
            Arc::new(ConnectionPool::new(1)),
        );

        let (server_stream, mut client) = connected_pair();
        client.write_all(b"BAD\r\n\r\n").expect("write");
        Server::run_basic_accept_loop(
            vec![Ok(server_stream)],
            server.clone(),
        );
        let mut response = String::new();
        let _ = client.read_to_string(&mut response).expect("read");
        assert!(response.starts_with("HTTP/1.1 400 BAD REQUEST"));

        let (server_stream, mut client) = connected_pair();
        client.write_all(b"BAD\r\n\r\n").expect("write");
        Server::run_thread_pool_accept_loop(
            vec![Ok(server_stream)],
            server.clone(),
            &pool,
        );
        let mut response = String::new();
        let _ = client.read_to_string(&mut response).expect("read");
        assert!(response.starts_with("HTTP/1.1 400 BAD REQUEST"));

        let (server_stream, mut client) = connected_pair();
        client.write_all(b"BAD\r\n\r\n").expect("write");
        Server::run_pooling_accept_loop(
            vec![Ok(server_stream)],
            server.clone(),
            &pool,
            Arc::new(ConnectionPool::new(1)),
        );
        let mut response = String::new();
        let _ = client.read_to_string(&mut response).expect("read");
        assert!(response.starts_with("HTTP/1.1 400 BAD REQUEST"));

        let (server_stream, mut client) = connected_pair();
        Server::run_pooling_accept_loop(
            vec![Ok(server_stream)],
            server,
            &pool,
            Arc::new(ConnectionPool::new(0)),
        );
        let mut response = String::new();
        let _ = client.read_to_string(&mut response).expect("read");
        assert!(
            response.starts_with("HTTP/1.1 503 SERVICE UNAVAILABLE")
        );
    }

    #[test]
    fn test_immutable_cache_control_policy() {
        let root = setup_test_directory();
        let server = Server::builder()
            .address("127.0.0.1:0")
            .document_root(root.path().to_str().expect("path"))
            .static_cache_ttl_secs(60)
            .build()
            .expect("server");

        let request = Request {
            method: "GET".to_string(),
            path: "/assets/app-abcdef12.js".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
        };
        let response = apply_response_policies(
            Response::new(200, "OK", b"ok".to_vec()),
            &server,
            &request,
        );
        assert!(response.headers.iter().any(|(name, value)| {
            name.eq_ignore_ascii_case("cache-control")
                && value.contains("immutable")
        }));
    }

    #[test]
    fn test_zstd_precompressed_asset_is_served() {
        let root = setup_test_directory();
        let file = root.path().join("index.html.zst");
        fs::write(&file, b"zstd-data").expect("write");

        let mut headers = HashMap::new();
        let _ = headers.insert(
            "accept-encoding".to_string(),
            "zstd,gzip".to_string(),
        );
        let request = Request {
            method: "GET".to_string(),
            path: "/index.html".to_string(),
            version: "HTTP/1.1".to_string(),
            headers,
        };

        let response = generate_response_with_cache(
            &request,
            root.path(),
            &fs::canonicalize(root.path()).expect("canonicalize"),
            None,
        )
        .expect("response");
        assert!(response.headers.iter().any(|(name, value)| {
            name.eq_ignore_ascii_case("content-encoding")
                && value.eq_ignore_ascii_case("zstd")
        }));
        assert_eq!(response.body, b"zstd-data");
    }

    #[test]
    fn test_brotli_precompressed_asset_is_served() {
        let root = setup_test_directory();
        fs::write(root.path().join("index.html.br"), b"brotli-encoded")
            .expect("write br");

        let mut headers = HashMap::new();
        let _ = headers.insert(
            "accept-encoding".to_string(),
            "br, gzip".to_string(),
        );
        let request = Request {
            method: "GET".to_string(),
            path: "/index.html".to_string(),
            version: "HTTP/1.1".to_string(),
            headers,
        };

        let response = generate_response_with_cache(
            &request,
            root.path(),
            &fs::canonicalize(root.path()).expect("canonicalize"),
            None,
        )
        .expect("response");
        assert!(response.headers.iter().any(|(name, value)| {
            name.eq_ignore_ascii_case("content-encoding")
                && value.eq_ignore_ascii_case("br")
        }));
        assert_eq!(response.body, b"brotli-encoded");
    }

    #[test]
    fn test_gzip_precompressed_asset_is_served() {
        let root = setup_test_directory();
        fs::write(root.path().join("index.html.gz"), b"gzdata")
            .expect("write gz");

        let mut headers = HashMap::new();
        let _ = headers
            .insert("accept-encoding".to_string(), "gzip".to_string());
        let request = Request {
            method: "GET".to_string(),
            path: "/index.html".to_string(),
            version: "HTTP/1.1".to_string(),
            headers,
        };

        let response = generate_response_with_cache(
            &request,
            root.path(),
            &fs::canonicalize(root.path()).expect("canonicalize"),
            None,
        )
        .expect("response");
        assert!(response.headers.iter().any(|(name, value)| {
            name.eq_ignore_ascii_case("content-encoding")
                && value.eq_ignore_ascii_case("gzip")
        }));
        assert_eq!(response.body, b"gzdata");
    }

    #[test]
    fn test_serve_file_response_applies_cache_ttl() {
        let root = setup_test_directory();
        let request = Request {
            method: "GET".to_string(),
            path: "/index.html".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
        };

        let response = generate_response_with_cache(
            &request,
            root.path(),
            &fs::canonicalize(root.path()).expect("canonicalize"),
            Some(600),
        )
        .expect("response");
        assert!(response.headers.iter().any(|(name, value)| {
            name.eq_ignore_ascii_case("cache-control")
                && value.contains("max-age=600")
        }));
    }

    #[test]
    fn test_parse_range_header_covers_all_branches() {
        // Missing header / wrong prefix / malformed.
        assert!(parse_range_header(None, 100).is_none());
        assert!(parse_range_header(Some("items=0-1"), 100).is_none());
        assert!(
            parse_range_header(Some("bytes=no-dash"), 100).is_none()
        );
        // Both ends empty.
        assert!(parse_range_header(Some("bytes=-"), 100).is_none());
        // Suffix form: last N bytes.
        assert_eq!(
            parse_range_header(Some("bytes=-10"), 100),
            Some((90, 99))
        );
        // Suffix longer than file or zero: rejected.
        assert!(parse_range_header(Some("bytes=-0"), 100).is_none());
        assert!(parse_range_header(Some("bytes=-500"), 100).is_none());
        // Open-ended "start-" form uses total-1 as end.
        assert_eq!(
            parse_range_header(Some("bytes=10-"), 100),
            Some((10, 99))
        );
        // Open-ended on empty body falls off checked_sub.
        assert!(parse_range_header(Some("bytes=0-"), 0).is_none());
        // Explicit start > end is rejected.
        assert!(parse_range_header(Some("bytes=50-10"), 100).is_none());
        // End beyond total is rejected.
        assert!(
            parse_range_header(Some("bytes=0-9999"), 100).is_none()
        );
        // Well-formed closed range.
        assert_eq!(
            parse_range_header(Some("bytes=0-9"), 100),
            Some((0, 9))
        );
        // Non-numeric parts.
        assert!(parse_range_header(Some("bytes=abc-9"), 100).is_none());
        assert!(parse_range_header(Some("bytes=0-abc"), 100).is_none());
        assert!(parse_range_header(Some("bytes=-abc"), 100).is_none());
    }

    #[test]
    fn test_non_immutable_cache_control_policy_uses_ttl() {
        let root = setup_test_directory();
        let server = Server::builder()
            .address("127.0.0.1:0")
            .document_root(root.path().to_str().expect("path"))
            .static_cache_ttl_secs(90)
            .build()
            .expect("server");

        let request = Request {
            method: "GET".to_string(),
            path: "/index.html".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
        };
        let response = apply_response_policies(
            Response::new(200, "OK", b"ok".to_vec()),
            &server,
            &request,
        );
        assert!(response.headers.iter().any(|(name, value)| {
            name.eq_ignore_ascii_case("cache-control")
                && value == "public, max-age=90"
        }));
    }

    #[test]
    fn test_cache_control_policy_respects_existing_header() {
        let root = setup_test_directory();
        let server = Server::builder()
            .address("127.0.0.1:0")
            .document_root(root.path().to_str().expect("path"))
            .static_cache_ttl_secs(90)
            .build()
            .expect("server");

        let mut existing = Response::new(200, "OK", b"ok".to_vec());
        existing.add_header("Cache-Control", "no-store");

        let request = Request {
            method: "GET".to_string(),
            path: "/anything.txt".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
        };
        let response =
            apply_response_policies(existing, &server, &request);
        let header = response
            .headers
            .iter()
            .find(|(name, _)| {
                name.eq_ignore_ascii_case("cache-control")
            })
            .map(|(_, value)| value.clone())
            .expect("cache-control");
        assert_eq!(header, "no-store");
    }

    #[test]
    fn test_is_probably_immutable_asset_path_edge_cases() {
        assert!(is_probably_immutable_asset_path(
            "/assets/app-abcdef12.js"
        ));
        // No extension → rsplit_once('.') returns None.
        assert!(!is_probably_immutable_asset_path("/noext"));
        // Non-hex hash suffix is rejected.
        assert!(!is_probably_immutable_asset_path(
            "/assets/app-zzzzzzzz.js"
        ));
        // Too short to be a hash.
        assert!(!is_probably_immutable_asset_path("/assets/app-ab.js"));
    }

    #[test]
    fn test_record_metrics_tracks_5xx_responses() {
        let before = METRIC_RESPONSES_5XX.load(Ordering::Relaxed);
        let response =
            Response::new(503, "SERVICE UNAVAILABLE", b"down".to_vec());
        record_metrics(&response);
        let after = METRIC_RESPONSES_5XX.load(Ordering::Relaxed);
        assert!(after > before);
    }

    #[test]
    fn test_rate_limit_recovers_from_poisoned_mutex() {
        let state =
            RATE_LIMIT_STATE.get_or_init(|| Mutex::new(HashMap::new()));
        let _ = std::panic::catch_unwind(|| {
            let _guard = state.lock().expect("lock");
            panic!("intentional to poison");
        });
        assert!(state.is_poisoned());

        let root = setup_test_directory();
        let server = Server::builder()
            .address("127.0.0.1:0")
            .document_root(root.path().to_str().expect("path"))
            .rate_limit_per_minute(10)
            .build()
            .expect("server");
        let ip: IpAddr = "127.0.0.1".parse().expect("ip");
        // Must not panic even though the mutex is poisoned — poisoned lock
        // recovery branch.
        let _ = is_rate_limited(&server, ip);

        // Clear poison so subsequent tests see a healthy lock.
        state.clear_poison();
    }

    #[test]
    fn test_log_connection_result_handles_error() {
        // The Ok path is exercised by existing tests via run_*_accept_loop.
        // Exercise the Err branch directly (eprintln) to cover the error arm.
        Server::log_connection_result(Err(
            ServerError::invalid_request("boom"),
        ));
    }

    #[test]
    fn test_start_with_shutdown_signal_reports_active_connections_on_timeout()
     {
        let root = setup_test_directory();
        let server = Server::builder()
            .address("127.0.0.1:0")
            .document_root(root.path().to_str().expect("path"))
            .build()
            .expect("server");

        // 50ms grace period so wait_for_shutdown returns `false` if a
        // connection is still in flight when we request shutdown.
        let shutdown =
            Arc::new(ShutdownSignal::new(Duration::from_millis(50)));
        let (ready_tx, ready_rx) = mpsc::channel::<String>();
        let shutdown_for_server = shutdown.clone();
        let server_clone = server.clone();
        let handle = thread::spawn(move || {
            server_clone
                .start_with_shutdown_signal_and_ready(
                    shutdown_for_server,
                    move |addr| {
                        let _ = ready_tx.send(addr);
                    },
                )
                .expect("server start");
        });

        let addr = ready_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("ready");

        // Hold a long-running connection so the grace period expires before
        // the tracked handler finishes, forcing the "active connections
        // remaining" branch.
        let _holder = TcpStream::connect(&addr).expect("connect");
        thread::sleep(Duration::from_millis(20));
        shutdown.shutdown();

        handle.join().expect("join server thread");
    }

    #[test]
    fn test_start_with_thread_pool_serves_one_connection() {
        let root = setup_test_directory();
        let probe = TcpListener::bind("127.0.0.1:0").expect("probe");
        let addr = probe.local_addr().expect("addr");
        drop(probe);

        let server = Server::builder()
            .address(&addr.to_string())
            .document_root(root.path().to_str().expect("path"))
            .build()
            .expect("server");

        let _handle = thread::spawn(move || {
            let _ = server.start_with_thread_pool(2);
        });

        // Retry briefly until the server has bound.
        let mut stream = None;
        for _ in 0..50 {
            if let Ok(s) = TcpStream::connect(&addr.to_string()) {
                stream = Some(s);
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        let mut stream = stream.expect("server did not bind");
        stream
            .write_all(
                b"GET /index.html HTTP/1.1\r\nHost: localhost\r\n\r\n",
            )
            .expect("write");
        let mut response = String::new();
        let _ = stream.read_to_string(&mut response).expect("read");
        assert!(response.starts_with("HTTP/1.1 200 OK"));
        // Thread continues serving but detaches with the test.
    }

    #[test]
    fn test_start_with_pooling_serves_one_connection() {
        let root = setup_test_directory();
        let probe = TcpListener::bind("127.0.0.1:0").expect("probe");
        let addr = probe.local_addr().expect("addr");
        drop(probe);

        let server = Server::builder()
            .address(&addr.to_string())
            .document_root(root.path().to_str().expect("path"))
            .build()
            .expect("server");

        let _handle = thread::spawn(move || {
            let _ = server.start_with_pooling(2, 4);
        });

        let mut stream = None;
        for _ in 0..50 {
            if let Ok(s) = TcpStream::connect(&addr.to_string()) {
                stream = Some(s);
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        let mut stream = stream.expect("server did not bind");
        stream
            .write_all(
                b"GET /index.html HTTP/1.1\r\nHost: localhost\r\n\r\n",
            )
            .expect("write");
        let mut response = String::new();
        let _ = stream.read_to_string(&mut response).expect("read");
        assert!(response.starts_with("HTTP/1.1 200 OK"));
    }
}
