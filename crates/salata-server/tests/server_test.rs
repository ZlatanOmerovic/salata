use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Duration;

/// Find a free port by binding to port 0.
fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind to free port");
    listener.local_addr().expect("local addr").port()
}

/// Create a temporary directory with a config.toml and test files.
struct TestSite {
    dir: PathBuf,
}

impl TestSite {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "salata_server_test_{name}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Write a minimal config.toml.
        let config = r#"
[salata]
display_errors = true

[server]
hot_reload = false
"#;
        fs::write(dir.join("config.toml"), config).unwrap();

        Self { dir }
    }

    fn path(&self) -> &PathBuf {
        &self.dir
    }

    fn config_path(&self) -> PathBuf {
        self.dir.join("config.toml")
    }

    fn write_file(&self, name: &str, content: &str) {
        let path = self.dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }
}

impl Drop for TestSite {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Start the server binary and return the child process.
fn start_server(site: &TestSite, port: u16) -> Child {
    let binary = env!("CARGO_BIN_EXE_salata-server");
    let child = Command::new(binary)
        .arg("--config")
        .arg(site.config_path())
        .arg("--port")
        .arg(port.to_string())
        .arg(site.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to start salata-server");

    // Wait for server to be ready.
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > Duration::from_secs(5) {
            panic!("server did not start within 5 seconds");
        }
        if TcpListener::bind(("127.0.0.1", port)).is_err() {
            // Port is in use = server is listening.
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    child
}

fn stop_server(mut child: Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn serve_static_html_file() {
    let site = TestSite::new("static_html");
    site.write_file("page.html", "<h1>Hello</h1>");

    let port = free_port();
    let child = start_server(&site, port);

    let resp = reqwest::blocking::get(format!("http://127.0.0.1:{port}/page.html")).unwrap();
    assert_eq!(resp.status(), 200);
    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.contains("text/html"), "expected text/html, got: {ct}");
    let body = resp.text().unwrap();
    assert_eq!(body, "<h1>Hello</h1>");

    stop_server(child);
}

#[test]
fn serve_static_css_file() {
    let site = TestSite::new("static_css");
    site.write_file("style.css", "body { color: red; }");

    let port = free_port();
    let child = start_server(&site, port);

    let resp = reqwest::blocking::get(format!("http://127.0.0.1:{port}/style.css")).unwrap();
    assert_eq!(resp.status(), 200);
    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.contains("css"), "expected css content type, got: {ct}");

    stop_server(child);
}

#[test]
fn serve_static_js_file() {
    let site = TestSite::new("static_js");
    site.write_file("app.js", "console.log('hello');");

    let port = free_port();
    let child = start_server(&site, port);

    let resp = reqwest::blocking::get(format!("http://127.0.0.1:{port}/app.js")).unwrap();
    assert_eq!(resp.status(), 200);
    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        ct.contains("javascript"),
        "expected javascript content type, got: {ct}"
    );

    stop_server(child);
}

#[test]
fn serve_404_for_missing_file() {
    let site = TestSite::new("404");

    let port = free_port();
    let child = start_server(&site, port);

    let resp = reqwest::blocking::get(format!("http://127.0.0.1:{port}/nonexistent.html")).unwrap();
    assert_eq!(resp.status(), 404);

    stop_server(child);
}

#[test]
fn directory_index_html() {
    let site = TestSite::new("dir_index_html");
    site.write_file("index.html", "<h1>Index</h1>");

    let port = free_port();
    let child = start_server(&site, port);

    let resp = reqwest::blocking::get(format!("http://127.0.0.1:{port}/")).unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().unwrap();
    assert_eq!(body, "<h1>Index</h1>");

    stop_server(child);
}

#[test]
fn html_only_slt_file() {
    let site = TestSite::new("slt_html_only");
    site.write_file("page.slt", "<h1>Pure HTML in SLT</h1>");

    let port = free_port();
    let child = start_server(&site, port);

    let resp = reqwest::blocking::get(format!("http://127.0.0.1:{port}/page.slt")).unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().unwrap();
    assert!(
        body.contains("<h1>Pure HTML in SLT</h1>"),
        "unexpected body: {body}"
    );

    stop_server(child);
}

#[test]
fn path_traversal_blocked() {
    let site = TestSite::new("traversal");
    site.write_file("secret.txt", "secret data");

    let port = free_port();
    let child = start_server(&site, port);

    // Use a raw TCP request to avoid reqwest normalizing the path.
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    write!(
        stream,
        "GET /../../../etc/passwd HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);
    assert!(
        response.contains("403"),
        "expected 403 in response, got: {response}"
    );

    stop_server(child);
}

#[test]
fn blocked_extension_toml() {
    let site = TestSite::new("blocked_ext");

    let port = free_port();
    let child = start_server(&site, port);

    let resp = reqwest::blocking::get(format!("http://127.0.0.1:{port}/config.toml")).unwrap();
    assert_eq!(resp.status(), 403);

    stop_server(child);
}

#[test]
fn dotfile_blocked() {
    let site = TestSite::new("dotfile");
    site.write_file(".env", "SECRET=123");

    let port = free_port();
    let child = start_server(&site, port);

    let resp = reqwest::blocking::get(format!("http://127.0.0.1:{port}/.env")).unwrap();
    assert_eq!(resp.status(), 403);

    stop_server(child);
}

#[test]
fn subdirectory_static_file() {
    let site = TestSite::new("subdir");
    site.write_file("assets/logo.svg", "<svg></svg>");

    let port = free_port();
    let child = start_server(&site, port);

    let resp = reqwest::blocking::get(format!("http://127.0.0.1:{port}/assets/logo.svg")).unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().unwrap();
    assert_eq!(body, "<svg></svg>");

    stop_server(child);
}
