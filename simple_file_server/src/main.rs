use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use infer;
use url_escape::encode_component;

fn handle_connection(mut stream: TcpStream, root_dir: &PathBuf) {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    let request = String::from_utf8_lossy(&buffer[..]);

    // Get the requested file path
    let request_line = request.lines().next().unwrap_or_default();
    let request_path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .trim_start_matches('/');

    let requested_path = root_dir.join(request_path);

    if requested_path.is_dir() {
        // Serve directory listing
        let response = list_directory(&requested_path);
        let response = format!("HTTP/1.1 200 OK\r\n\r\n{}", response);
        stream.write_all(response.as_bytes()).unwrap();
    } else if requested_path.is_file() {
        // Serve the file
        let content_type = infer::get_from_path(&requested_path)
            .unwrap()
            .map_or("application/octet-stream", |t| t.mime_type());

        let mut file = fs::File::open(&requested_path).unwrap();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {}\r\n\r\n",
            content_type
        );
        stream.write_all(response.as_bytes()).unwrap();
        stream.write_all(&contents).unwrap();
    } else {
        let response = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
        stream.write_all(response.as_bytes()).unwrap();
    }
}

fn list_directory(dir: &Path) -> String {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html><html><head><meta charset='utf-8'></head><body>");
    html.push_str(&format!("<h1>Directory listing for {}</h1>", dir.display()));

    html.push_str("<ul>");
    for entry in WalkDir::new(dir).max_depth(1) {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy();

        let escaped_name = encode_component(&name);
        let display_name = if path.is_dir() {
            format!("{}/", name)
        } else {
            name.to_string()
        };

        html.push_str(&format!(
            "<li><a href='{}'>{}</a></li>",
            escaped_name, display_name
        ));
    }
    html.push_str("</ul>");
    html.push_str("</body></html>");

    html
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let root_dir = std::env::current_dir().unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream, &root_dir);
    }
}
