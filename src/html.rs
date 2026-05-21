use fastly::http::StatusCode;
use fastly::{Request, Response};

const EXAMPLE_KEY: &str = "uploads/example.bin";

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Build `http://127.0.0.1:7676` locally or `https://your.domain` when deployed.
pub fn request_base_url(req: &Request) -> String {
    let host = req.get_header_str("host").unwrap_or("127.0.0.1:7676");
    let scheme = if host.starts_with("127.0.0.1") || host.starts_with("localhost") {
        "http"
    } else {
        req.get_header_str("x-forwarded-proto").unwrap_or("https")
    };
    format!("{scheme}://{host}")
}

fn render_index(base_url: &str) -> String {
    let base = escape_html(base_url);
    let example_key = escape_html(EXAMPLE_KEY);
    let upload_url = format!("{base_url}/api/upload?key={EXAMPLE_KEY}");
    let upload_url_escaped = escape_html(&upload_url);

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Fastly Object Storage uploader</title>
  <style>
    :root {{ font-family: system-ui, sans-serif; color: #1a1a1a; background: #f6f7f9; }}
    body {{ max-width: 48rem; margin: 2rem auto; padding: 0 1rem; }}
    h1 {{ font-size: 1.35rem; margin-bottom: 0.25rem; }}
    h2 {{ font-size: 1.05rem; margin-top: 2rem; }}
    p.lead {{ color: #555; margin-top: 0; }}
    form, section.examples {{ background: #fff; border: 1px solid #d8dde6; border-radius: 8px; padding: 1.25rem; }}
    label {{ display: block; font-weight: 600; margin: 0.75rem 0 0.35rem; }}
    input[type="text"], input[type="file"] {{ width: 100%; box-sizing: border-box; }}
    button {{ margin-top: 1rem; background: #4b2bb5; color: #fff; border: 0; border-radius: 6px;
      padding: 0.65rem 1rem; font-size: 1rem; cursor: pointer; }}
    button:disabled {{ opacity: 0.6; cursor: wait; }}
    pre {{ background: #0f172a; color: #e2e8f0; padding: 1rem; border-radius: 8px;
      overflow-x: auto; font-size: 0.85rem; white-space: pre-wrap; word-break: break-all; }}
    .hint {{ color: #555; font-size: 0.9rem; margin: 0.5rem 0 0; }}
    .err {{ color: #b42318; }}
    code {{ font-size: 0.9em; }}
  </style>
</head>
<body>
  <h1>Fastly Object Storage uploader</h1>
  <p class="lead">Demo UI for the Compute starter. Files are uploaded with AWS SigV4 to your FOS bucket.</p>
  <p class="hint">Base URL for this page: <code>{base}</code></p>

  <h2>Upload a file</h2>
  <form id="upload-form">
    <label for="prefix">Object key prefix (optional)</label>
    <input id="prefix" type="text" value="uploads/" placeholder="uploads/">

    <label for="file">File</label>
    <input id="file" type="file" required>

    <button type="submit" id="submit">Upload</button>
  </form>

  <h2>Upload result</h2>
  <pre id="result">Pick a file and click Upload.</pre>

  <h2>API examples (curl)</h2>
  <section class="examples">
    <p class="hint">Examples use the hostname from your current request (<code>Host</code> header).</p>

    <p><strong>POST</strong> — upload a file:</p>
    <pre>curl -i -X POST "{upload_url_escaped}" \
  -H "Content-Type: application/octet-stream" \
  --data-binary @./example.bin</pre>

    <p><strong>PUT</strong> — same upload, different method:</p>
    <pre>curl -i -X PUT "{upload_url_escaped}" \
  -H "Content-Type: application/octet-stream" \
  --data-binary @./example.bin</pre>

    <p><strong>Custom object key</strong> — change <code>key=</code> in the URL:</p>
    <pre>curl -i -X POST "{base}/api/upload?key=my-folder/my-file.pdf" \
  --data-binary @./my-file.pdf</pre>
  </section>

  <script>
    const form = document.getElementById('upload-form');
    const result = document.getElementById('result');
    const submit = document.getElementById('submit');

    form.addEventListener('submit', async (e) => {{
      e.preventDefault();
      const fileInput = document.getElementById('file');
      const prefixInput = document.getElementById('prefix');
      const file = fileInput.files[0];
      if (!file) return;

      let prefix = prefixInput.value.trim();
      if (prefix && !prefix.endsWith('/')) prefix += '/';
      const key = prefix + file.name;

      submit.disabled = true;
      result.textContent = 'Uploading…';

      try {{
        const url = '/api/upload?key=' + encodeURIComponent(key);
        const res = await fetch(url, {{
          method: 'POST',
          body: file,
          headers: {{ 'Content-Type': file.type || 'application/octet-stream' }}
        }});
        const text = await res.text();
        let body;
        try {{ body = JSON.stringify(JSON.parse(text), null, 2); }}
        catch {{ body = text; }}
        result.textContent = 'HTTP ' + res.status + '\\n\\n' + body;
        if (!res.ok) result.classList.add('err');
        else result.classList.remove('err');
      }} catch (err) {{
        result.classList.add('err');
        result.textContent = String(err);
      }} finally {{
        submit.disabled = false;
      }}
    }});
  </script>
</body>
</html>"#
    )
}

pub fn index_page(req: &Request) -> Response {
    let html = render_index(&request_base_url(req));
    Response::from_status(StatusCode::OK)
        .with_content_type(fastly::mime::TEXT_HTML_UTF_8)
        .with_body(html)
}
