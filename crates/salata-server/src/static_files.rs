//! Static file serving with MIME type detection.
//!
//! Serves non-`.slt` files (HTML, CSS, JavaScript, images, fonts, media, etc.)
//! directly from disk with the appropriate `Content-Type` header. MIME types
//! are inferred from file extensions using the `mime_guess` crate, falling
//! back to `application/octet-stream` for unknown types.

use std::path::Path;

use actix_web::HttpResponse;

/// Read a file from disk and return it as an HTTP response with the correct
/// `Content-Type` header inferred from the file extension.
pub fn serve_static_file(path: &Path) -> std::io::Result<HttpResponse> {
    let bytes = std::fs::read(path)?;
    let mime = mime_guess::from_path(path)
        .first_raw()
        .unwrap_or("application/octet-stream");
    Ok(HttpResponse::Ok().content_type(mime).body(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn serve_html_file() {
        let dir = std::env::temp_dir().join("salata_static_test");
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("test.html");
        fs::write(&file, "<h1>hello</h1>").unwrap();

        let resp = serve_static_file(&file).unwrap();
        assert_eq!(resp.status(), 200);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn serve_css_file() {
        let dir = std::env::temp_dir().join("salata_static_test_css");
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("style.css");
        fs::write(&file, "body { color: red; }").unwrap();

        let resp = serve_static_file(&file).unwrap();
        assert_eq!(resp.status(), 200);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn serve_nonexistent_file() {
        let result = serve_static_file(Path::new("/nonexistent/file.txt"));
        assert!(result.is_err());
    }
}
