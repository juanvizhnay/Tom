//! The one place this application talks to the network.
//!
//! Everything about *what* to send and how to read the answer lives in `tom-core`; this
//! module only carries bytes. It is also the only code that ever holds a decrypted API
//! key, and it holds it just long enough to build the request.

use std::time::Duration;

use tom_core::{ChatRequest, InferenceError, parse_chat_response, parse_error_message};

/// How long to wait for a whole reply before giving up.
///
/// Generous: a large local model on a cold start can take a while, and the request runs on
/// a worker thread where waiting costs the user nothing.
const REQUEST_TIMEOUT: Duration = Duration::from_mins(2);

/// Sends one chat request and returns the reply text.
///
/// Blocking, so it must be called from a worker thread and never from the UI thread.
///
/// # Errors
///
/// Distinguishes a request that never arrived ([`InferenceError::Transport`]) from one the
/// provider refused ([`InferenceError::Status`]) so the interface can say which happened.
pub fn send_chat(request: &ChatRequest) -> Result<String, InferenceError> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        // Read the body of a failed response too: that is where providers explain refusals.
        .http_status_as_error(false)
        .build()
        .into();

    let mut call = agent.post(&request.url);
    for (name, value) in &request.headers {
        call = call.header(name.as_str(), value.as_str());
    }

    let mut response = call
        .send(request.body.as_bytes())
        .map_err(|error| InferenceError::Transport(error.to_string()))?;

    let status = response.status().as_u16();
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| InferenceError::Transport(error.to_string()))?;

    if !(200..300).contains(&status) {
        return Err(InferenceError::Status {
            code: status,
            message: parse_error_message(&body).unwrap_or_default(),
        });
    }

    parse_chat_response(request.provider, &body)
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::{TcpListener, TcpStream},
        thread,
    };

    use super::send_chat;
    use tom_core::{AiProvider, ChatRequest, InferenceError};

    /// What one canned exchange saw on the wire.
    struct Exchange {
        request_line: String,
        headers: Vec<(String, String)>,
        body: String,
    }

    /// Serves exactly one HTTP request from a throwaway port and reports what it received.
    ///
    /// Real enough to exercise ureq end to end - status handling, headers, and body - which
    /// is the part `tom-core`'s pure tests cannot reach.
    fn serve_once(
        status: u16,
        response_body: &'static str,
    ) -> (String, thread::JoinHandle<Exchange>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a free port");
        let url = format!(
            "http://{}/chat/completions",
            listener.local_addr().expect("addr")
        );

        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("one connection");
            let exchange = read_request(&stream);
            let mut stream = stream;
            let reason = if (200..300).contains(&status) {
                "OK"
            } else {
                "ERR"
            };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
                response_body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
            stream.flush().expect("flush");
            exchange
        });

        (url, handle)
    }

    fn read_request(stream: &TcpStream) -> Exchange {
        let mut reader = BufReader::new(stream);
        let mut request_line = String::new();
        reader.read_line(&mut request_line).expect("request line");

        let mut headers = Vec::new();
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("header line");
            let line = line.trim_end().to_owned();
            if line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                let name = name.trim().to_ascii_lowercase();
                let value = value.trim().to_owned();
                if name == "content-length" {
                    content_length = value.parse().expect("a numeric content length");
                }
                headers.push((name, value));
            }
        }

        let mut body = vec![0u8; content_length];
        reader.read_exact(&mut body).expect("body");

        Exchange {
            request_line: request_line.trim_end().to_owned(),
            headers,
            body: String::from_utf8(body).expect("utf-8 body"),
        }
    }

    fn request(url: String) -> ChatRequest {
        ChatRequest {
            url,
            headers: vec![
                ("content-type".to_owned(), "application/json".to_owned()),
                ("authorization".to_owned(), "Bearer sk-test".to_owned()),
            ],
            body: r#"{"model":"m","messages":[]}"#.to_owned(),
            provider: AiProvider::OpenAi,
        }
    }

    #[test]
    fn a_successful_exchange_sends_the_headers_and_returns_the_reply() {
        let (url, server) = serve_once(
            200,
            r#"{"choices":[{"message":{"role":"assistant","content":"hola desde el servidor"}}]}"#,
        );

        let reply = send_chat(&request(url)).expect("the server answered");
        let exchange = server.join().expect("server thread");

        assert_eq!(reply, "hola desde el servidor");
        assert!(exchange.request_line.starts_with("POST /chat/completions"));
        assert!(
            exchange
                .headers
                .iter()
                .any(|(name, value)| name == "authorization" && value == "Bearer sk-test"),
            "the credential reaches the provider verbatim"
        );
        assert!(
            exchange
                .headers
                .iter()
                .any(|(name, value)| name == "content-type" && value == "application/json")
        );
        assert_eq!(exchange.body, r#"{"model":"m","messages":[]}"#);
    }

    #[test]
    fn a_refused_exchange_reports_the_status_and_the_providers_own_words() {
        let (url, server) = serve_once(
            401,
            r#"{"error":{"type":"authentication_error","message":"invalid x-api-key"}}"#,
        );

        let outcome = send_chat(&request(url));
        let _ = server.join();

        assert_eq!(
            outcome,
            Err(InferenceError::Status {
                code: 401,
                message: "invalid x-api-key".to_owned(),
            })
        );
    }

    #[test]
    fn an_error_body_that_is_not_json_still_reports_the_status() {
        let (url, server) = serve_once(502, "<html>bad gateway</html>");

        let outcome = send_chat(&request(url));
        let _ = server.join();

        match outcome {
            Err(InferenceError::Status { code, message }) => {
                assert_eq!(code, 502);
                assert!(message.is_empty());
            }
            other => panic!("expected a status error, got {other:?}"),
        }
    }

    #[test]
    fn an_unreachable_provider_is_a_transport_failure_not_a_status() {
        // Port 1 on loopback refuses immediately.
        let mut unreachable = request("http://127.0.0.1:1/chat/completions".to_owned());
        unreachable.body = "{}".to_owned();

        assert!(matches!(
            send_chat(&unreachable),
            Err(InferenceError::Transport(_))
        ));
    }
}
