//! Server command
//!
//! Serve gems over HTTP with documentation browsing

use anyhow::{Context, Result};
use axum::{
    Router,
    extract::Path as AxumPath,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use flate2::{
    Compression,
    write::{GzEncoder, ZlibEncoder},
};
use lode::gem_store::GemStore;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tower_http::services::ServeDir;

/// Options for gem server command
#[derive(Debug)]
pub(crate) struct ServerOptions {
    pub launch: bool,
    pub daemon: bool,
    pub verbose: bool,
    pub quiet: bool,
    pub silent: bool,
}

/// Run the gem server with options
pub(crate) async fn run_with_options(
    port: u16,
    gem_dir: Option<PathBuf>,
    bind: String,
    options: &ServerOptions,
) -> Result<()> {
    let gem_dir = if let Some(dir) = gem_dir {
        dir
    } else {
        get_default_gem_dir()?
    };

    // Handle output flags
    if !options.silent && !options.quiet {
        if options.verbose {
            println!("Starting gem server with configuration:");
            println!("  Port: {port}");
            println!("  Bind address: {bind}");
            println!("  Gem directory: {}", gem_dir.display());
            println!("  Daemon mode: {}", options.daemon);
        } else {
            println!("Serving gems from: {}", gem_dir.display());
        }
    }

    // Handle daemon mode
    if options.daemon {
        #[cfg(unix)]
        {
            use std::process::{Command, Stdio};

            if options.verbose && !options.silent && !options.quiet {
                println!("Forking to background...");
            }

            // Fork to background using nohup
            let child = Command::new(std::env::current_exe()?)
                .args([
                    "gem-server",
                    "-p",
                    &port.to_string(),
                    "-d",
                    &gem_dir.display().to_string(),
                    "-b",
                    &bind,
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .context("Failed to spawn daemon process")?;

            if !options.silent && !options.quiet {
                println!("Server started in background (PID: {})", child.id());
            }

            // Don't wait for the child process
            drop(child);
            return Ok(());
        }

        #[cfg(not(unix))]
        {
            anyhow::bail!(
                "Daemon mode is not supported on this platform. Run without --daemon flag."
            );
        }
    }

    let app = build_router(&gem_dir);
    let addr = format!("{bind}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind to {addr}"))?;

    if !options.silent && !options.quiet {
        if bind == "0.0.0.0" {
            println!("Server started at http://localhost:{port}");
        } else {
            println!("Server started at http://{bind}:{port}");
        }
    }

    if options.launch {
        launch_browser(port);
    }

    axum::serve(listener, app).await.context("Server error")?;
    Ok(())
}

fn build_router(gem_dir: &Path) -> Router {
    let gem_dir_clone = gem_dir.to_path_buf();
    let gem_dir_clone2 = gem_dir.to_path_buf();
    let gem_dir_clone3 = gem_dir.to_path_buf();
    let gem_dir_clone4 = gem_dir.to_path_buf();

    Router::new()
        .route("/", get(root_handler))
        // Marshal API endpoints for gem install support
        .route(
            "/specs.4.8.gz",
            get(move || specs_handler(gem_dir_clone.clone(), false, false)),
        )
        .route(
            "/latest_specs.4.8.gz",
            get(move || specs_handler(gem_dir_clone2.clone(), true, false)),
        )
        .route(
            "/prerelease_specs.4.8.gz",
            get(move || specs_handler(gem_dir_clone3.clone(), false, true)),
        )
        // Quick gemspec endpoint for individual gems
        .route(
            "/quick/Marshal.4.8/{gemspec}",
            get(move |path| quick_marshal_handler(gem_dir_clone4.clone(), path)),
        )
        // Static file serving
        .nest_service("/gems", ServeDir::new(gem_dir.join("cache")))
        .nest_service("/doc_root", ServeDir::new(gem_dir.join("doc")))
}

async fn root_handler() -> Result<Html<String>, ServerError> {
    let store = GemStore::new().map_err(|e| ServerError(e.to_string()))?;
    let gems = store.list_gems().map_err(|e| ServerError(e.to_string()))?;

    let mut html = String::from(
        r"<!DOCTYPE html>
<html>
<head><title>RubyGems Index</title>
<style>
body{font-family:Arial,sans-serif;margin:40px;background:#f5f5f5}
h1{color:#333;border-bottom:3px solid #006;padding-bottom:10px}
.gem-list{list-style:none;padding:0}
.gem-item{background:white;margin:10px 0;padding:15px;border-radius:5px;box-shadow:0 2px 4px rgba(0,0,0,0.1)}
.gem-name{font-weight:bold;font-size:1.2em;color:#048}
.gem-version{color:#666;margin-left:10px}
a{color:#039;text-decoration:none}
a:hover{text-decoration:underline}
</style>
</head>
<body>
<h1>RubyGems Index</h1>
<p>There are ",
    );

    html.push_str(&gems.len().to_string());
    html.push_str(" gems installed</p>\n<ul class=\"gem-list\">\n");

    for gem in gems {
        html.push_str("  <li class=\"gem-item\">\n    <span class=\"gem-name\">");
        html.push_str(&html_escape(&gem.name));
        html.push_str("</span>\n    <span class=\"gem-version\">");
        html.push_str(&html_escape(&gem.version));
        html.push_str("</span>\n    <a href=\"/doc_root/");
        html.push_str(&html_escape(&format!("{}-{}", gem.name, gem.version)));
        html.push_str("/rdoc/index.html\">[rdoc]</a>\n  </li>\n");
    }

    html.push_str("</ul>\n</body>\n</html>");
    Ok(Html(html))
}

fn get_default_gem_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let candidates = vec![
        home.join(".gem/ruby/3.5.0"),
        home.join(".gem/ruby/3.4.0"),
        home.join(".gem/ruby/3.3.0"),
    ];

    for dir in candidates {
        if dir.join("specifications").exists() {
            return Ok(dir);
        }
    }

    anyhow::bail!("Could not auto-detect gem directory. Use --dir flag.")
}

fn launch_browser(port: u16) {
    let url = format!("http://localhost:{port}");
    println!("Launching browser to {url}");

    #[cfg(target_os = "macos")]
    {
        drop(std::process::Command::new("open").arg(&url).spawn());
    }

    #[cfg(target_os = "linux")]
    {
        drop(std::process::Command::new("xdg-open").arg(&url).spawn());
    }

    #[cfg(target_os = "windows")]
    {
        drop(
            std::process::Command::new("cmd")
                .args(["/c", "start", &url])
                .spawn(),
        );
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Handler for Marshal-formatted specs endpoints
#[allow(clippy::unused_async)]
async fn specs_handler(
    gem_dir: PathBuf,
    latest_only: bool,
    prerelease_only: bool,
) -> Result<Response, ServerError> {
    // Generate Marshal data using Ruby (guaranteed compatibility)
    let specs_dir = gem_dir.join("specifications");

    let ruby_script = format!(
        r"
require 'rubygems'
require 'rubygems/specification'

specs_dir = '{}'
latest_only = {}
prerelease_only = {}

specs = []
Dir.glob(File.join(specs_dir, '*.gemspec')).each do |path|
  begin
    spec = Gem::Specification.load(path)
    next if prerelease_only && !spec.version.prerelease?
    next if !prerelease_only && spec.version.prerelease?

    platform = spec.platform == 'ruby' ? 'ruby' : spec.platform.to_s
    specs << [spec.name, spec.version, platform]
  rescue => e
    # Skip invalid gemspecs
  end
end

if latest_only
  latest = {{}}
  specs.each do |name, version, platform|
    key = [name, platform]
    if !latest[key] || version > latest[key][1]
      latest[key] = [name, version, platform]
    end
  end
  specs = latest.values
end

specs.sort_by! {{ |name, version, platform| [name, version.to_s, platform] }}
print Marshal.dump(specs)
",
        specs_dir.display(),
        latest_only,
        prerelease_only
    );

    let output = Command::new("ruby")
        .arg("-e")
        .arg(&ruby_script)
        .output()
        .map_err(|e| ServerError(format!("Failed to run ruby: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ServerError(format!("Ruby script failed: {stderr}")));
    }

    let marshal_data = output.stdout;

    // Gzip compress
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&marshal_data)
        .map_err(|e| ServerError(format!("Gzip failed: {e}")))?;
    let compressed = encoder
        .finish()
        .map_err(|e| ServerError(format!("Gzip finish failed: {e}")))?;

    // Build response
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/x-gzip")],
        compressed,
    )
        .into_response())
}

/// Handler for individual gemspec files
#[allow(clippy::unused_async)]
async fn quick_marshal_handler(
    gem_dir: PathBuf,
    AxumPath(gemspec_name): AxumPath<String>,
) -> Result<Response, ServerError> {
    // Check if .rz (compressed) format was requested
    let is_compressed = std::path::Path::new(&gemspec_name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("rz"));

    // Parse gemspec filename: "gemname-version.gemspec.rz" or "gemname-version.gemspec"
    let gemspec_path = gemspec_name
        .trim_end_matches(".rz")
        .trim_end_matches(".gemspec");

    let specs_dir = gem_dir.join("specifications");
    let full_path = specs_dir.join(format!("{gemspec_path}.gemspec"));

    if !full_path.exists() {
        return Err(ServerError(format!(
            "Gemspec not found: {gemspec_path}.gemspec"
        )));
    }

    // Load and marshal the gemspec using Ruby
    let ruby_script = format!(
        r"
require 'rubygems'
require 'rubygems/specification'

spec = Gem::Specification.load('{}')
print Marshal.dump(spec)
",
        full_path.display()
    );

    let output = Command::new("ruby")
        .arg("-e")
        .arg(&ruby_script)
        .output()
        .map_err(|e| ServerError(format!("Failed to run ruby: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ServerError(format!("Ruby script failed: {stderr}")));
    }

    let marshal_data = output.stdout;

    // Compress if .rz was requested
    if is_compressed {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&marshal_data)
            .map_err(|e| ServerError(format!("Zlib compression failed: {e}")))?;
        let compressed = encoder
            .finish()
            .map_err(|e| ServerError(format!("Zlib finish failed: {e}")))?;

        Ok((
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/x-deflate")],
            compressed,
        )
            .into_response())
    } else {
        // Return uncompressed Marshal data
        Ok((
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/octet-stream")],
            marshal_data,
        )
            .into_response())
    }
}

struct ServerError(String);

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0).into_response()
    }
}

#[cfg(test)]
mod tests {
    // Note: This command primarily deals with filesystem/network operations
    // which are better suited for integration tests.
    // This is a placeholder for potential future unit tests.

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_placeholder() {
        // Placeholder test to maintain test structure
        assert!(true);
    }
}
