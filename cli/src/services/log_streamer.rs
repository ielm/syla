use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use colored::*;
use regex::Regex;

/// Log entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub service: String,
    pub level: LogLevel,
    pub message: String,
    pub fields: HashMap<String, serde_json::Value>,
    pub raw: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "TRACE" => LogLevel::Trace,
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARN" | "WARNING" => LogLevel::Warn,
            "ERROR" => LogLevel::Error,
            _ => LogLevel::Info,
        }
    }

    fn color(&self) -> Color {
        match self {
            LogLevel::Trace => Color::BrightBlack,
            LogLevel::Debug => Color::Cyan,
            LogLevel::Info => Color::Green,
            LogLevel::Warn => Color::Yellow,
            LogLevel::Error => Color::Red,
        }
    }
}

/// Configuration for log streaming
#[derive(Debug, Clone)]
pub struct LogStreamConfig {
    pub follow: bool,
    pub lines: Option<usize>,
    pub level_filter: Option<LogLevel>,
    pub service_filter: Option<String>,
    pub pattern_filter: Option<Regex>,
    pub format: LogFormat,
    pub buffer_size: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum LogFormat {
    Pretty,
    Json,
    Raw,
}

impl Default for LogStreamConfig {
    fn default() -> Self {
        Self {
            follow: false,
            lines: Some(100),
            level_filter: None,
            service_filter: None,
            pattern_filter: None,
            format: LogFormat::Pretty,
            buffer_size: 8192,
        }
    }
}

/// Log file watcher that monitors changes and sends new entries
struct LogWatcher {
    path: PathBuf,
    service: String,
    sender: Sender<LogEntry>,
    position: u64,
    parser: LogParser,
}

impl LogWatcher {
    fn new(path: PathBuf, service: String, sender: Sender<LogEntry>) -> Self {
        Self {
            path,
            service,
            sender,
            position: 0,
            parser: LogParser::new(),
        }
    }

    fn watch(&mut self, follow: bool) -> Result<()> {
        let file = File::open(&self.path)
            .with_context(|| format!("Failed to open log file: {}", self.path.display()))?;
        
        let mut reader = BufReader::new(file);
        
        // Seek to end if following
        if follow {
            self.position = reader.seek(SeekFrom::End(0))?;
        }
        
        loop {
            // Read new lines
            let mut line = String::new();
            while reader.read_line(&mut line)? > 0 {
                if let Some(entry) = self.parser.parse_line(&line, &self.service) {
                    let _ = self.sender.send(entry);
                }
                line.clear();
                self.position = reader.stream_position()?;
            }
            
            if !follow {
                break;
            }
            
            // Wait for new data
            thread::sleep(Duration::from_millis(100));
            
            // Reopen file to catch rotations
            if let Ok(metadata) = std::fs::metadata(&self.path) {
                let current_size = metadata.len();
                if current_size < self.position {
                    // File was truncated or rotated
                    let file = File::open(&self.path)?;
                    reader = BufReader::new(file);
                    self.position = 0;
                }
            }
        }
        
        Ok(())
    }
}

/// Log parser that extracts structured data from log lines
struct LogParser {
    json_regex: Regex,
    level_regex: Regex,
    timestamp_regex: Regex,
}

impl LogParser {
    fn new() -> Self {
        Self {
            json_regex: Regex::new(r"^\{.*\}$").unwrap(),
            level_regex: Regex::new(r"(?i)\b(TRACE|DEBUG|INFO|WARN|WARNING|ERROR)\b").unwrap(),
            timestamp_regex: Regex::new(r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}").unwrap(),
        }
    }

    fn parse_line(&self, line: &str, service: &str) -> Option<LogEntry> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        // Try to parse as JSON first
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            return self.parse_json_log(json, service, line);
        }

        // Parse as plain text
        self.parse_text_log(line, service)
    }

    fn parse_json_log(&self, mut json: serde_json::Value, service: &str, raw: &str) -> Option<LogEntry> {
        let obj = json.as_object_mut()?;
        
        // Extract standard fields
        let timestamp = obj.remove("timestamp")
            .or_else(|| obj.remove("time"))
            .or_else(|| obj.remove("ts"))
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| Utc::now());
        
        let level = obj.remove("level")
            .or_else(|| obj.remove("severity"))
            .and_then(|v| v.as_str())
            .map(LogLevel::from_str)
            .unwrap_or(LogLevel::Info);
        
        let message = obj.remove("message")
            .or_else(|| obj.remove("msg"))
            .and_then(|v| v.as_str())
            .unwrap_or(raw)
            .to_string();
        
        // Remaining fields become metadata
        let fields: HashMap<String, serde_json::Value> = obj.clone();
        
        Some(LogEntry {
            timestamp,
            service: service.to_string(),
            level,
            message,
            fields,
            raw: raw.to_string(),
        })
    }

    fn parse_text_log(&self, line: &str, service: &str) -> Option<LogEntry> {
        // Extract timestamp
        let timestamp = self.timestamp_regex.find(line)
            .and_then(|m| DateTime::parse_from_str(m.as_str(), "%Y-%m-%d %H:%M:%S").ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| Utc::now());
        
        // Extract log level
        let level = self.level_regex.find(line)
            .map(|m| LogLevel::from_str(m.as_str()))
            .unwrap_or(LogLevel::Info);
        
        Some(LogEntry {
            timestamp,
            service: service.to_string(),
            level,
            message: line.to_string(),
            fields: HashMap::new(),
            raw: line.to_string(),
        })
    }
}

/// Main log streaming service
pub struct LogStreamer {
    watchers: Arc<Mutex<HashMap<String, thread::JoinHandle<()>>>>,
    receiver: Arc<Mutex<Receiver<LogEntry>>>,
    sender: Sender<LogEntry>,
}

impl LogStreamer {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        
        Self {
            watchers: Arc::new(Mutex::new(HashMap::new())),
            receiver: Arc::new(Mutex::new(receiver)),
            sender,
        }
    }

    /// Add a log file to watch
    pub fn add_log_file(&self, service: String, path: PathBuf, follow: bool) -> Result<()> {
        let sender = self.sender.clone();
        
        let handle = thread::spawn(move || {
            let mut watcher = LogWatcher::new(path, service.clone(), sender);
            if let Err(e) = watcher.watch(follow) {
                eprintln!("Error watching log file for {}: {}", service, e);
            }
        });
        
        self.watchers.lock().unwrap().insert(service, handle);
        Ok(())
    }

    /// Stream logs with the given configuration
    pub fn stream(&self, config: LogStreamConfig) -> Result<()> {
        let receiver = self.receiver.lock().unwrap();
        let mut buffer = Vec::new();
        let mut count = 0;
        
        // Collect logs first if not following
        if !config.follow {
            while let Ok(entry) = receiver.recv_timeout(Duration::from_millis(100)) {
                if self.should_display(&entry, &config) {
                    buffer.push(entry);
                }
            }
            
            // Display last N lines
            let start = buffer.len().saturating_sub(config.lines.unwrap_or(buffer.len()));
            for entry in &buffer[start..] {
                self.display_entry(entry, &config);
            }
            
            return Ok(());
        }
        
        // Stream logs in real-time
        println!("{}", "Streaming logs (press Ctrl-C to stop)...".dimmed());
        
        loop {
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(entry) => {
                    if self.should_display(&entry, &config) {
                        self.display_entry(&entry, &config);
                        count += 1;
                        
                        if let Some(limit) = config.lines {
                            if count >= limit && !config.follow {
                                break;
                            }
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if !config.follow {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        
        Ok(())
    }

    fn should_display(&self, entry: &LogEntry, config: &LogStreamConfig) -> bool {
        // Check level filter
        if let Some(min_level) = config.level_filter {
            if (entry.level as u8) < (min_level as u8) {
                return false;
            }
        }
        
        // Check service filter
        if let Some(ref service) = config.service_filter {
            if !entry.service.contains(service) {
                return false;
            }
        }
        
        // Check pattern filter
        if let Some(ref pattern) = config.pattern_filter {
            if !pattern.is_match(&entry.message) {
                return false;
            }
        }
        
        true
    }

    fn display_entry(&self, entry: &LogEntry, config: &LogStreamConfig) {
        match config.format {
            LogFormat::Pretty => self.display_pretty(entry),
            LogFormat::Json => self.display_json(entry),
            LogFormat::Raw => println!("{}", entry.raw),
        }
    }

    fn display_pretty(&self, entry: &LogEntry) {
        let timestamp = entry.timestamp.with_timezone(&Local).format("%H:%M:%S%.3f");
        let level = format!("{:5}", format!("{:?}", entry.level).to_uppercase())
            .color(entry.level.color());
        let service = entry.service.bright_black();
        
        println!("{} {} {} {}", 
            timestamp.to_string().dimmed(),
            level,
            service,
            entry.message
        );
        
        // Display additional fields if present
        if !entry.fields.is_empty() {
            let fields: Vec<String> = entry.fields.iter()
                .map(|(k, v)| format!("{}={}", k.cyan(), v))
                .collect();
            println!("  {}", fields.join(" ").dimmed());
        }
    }

    fn display_json(&self, entry: &LogEntry) {
        if let Ok(json) = serde_json::to_string(entry) {
            println!("{}", json);
        }
    }

    /// Create a log file for a service
    pub fn create_log_file(&self, service: &str, log_dir: &Path) -> Result<PathBuf> {
        std::fs::create_dir_all(log_dir)?;
        
        let log_path = log_dir.join(format!("{}.log", service));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;
        
        // Write initial log entry
        let mut writer = BufWriter::new(file);
        writeln!(writer, "{} {} Service '{}' started", 
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            "[INFO]".green(),
            service
        )?;
        writer.flush()?;
        
        Ok(log_path)
    }

    /// Stop all watchers
    pub fn stop(&self) {
        let mut watchers = self.watchers.lock().unwrap();
        watchers.clear(); // Threads will exit on their own
    }
}

impl Drop for LogStreamer {
    fn drop(&mut self) {
        self.stop();
    }
}