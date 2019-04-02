use std::collections::{HashMap};
use std::net::{SocketAddr};
use std::time::{Duration};

use futures::{future, Future, Async, Poll};
use simple_error::{SimpleError};
use tokio::prelude::*;
use tokio::net::{TcpStream};
use tokio::io::{write_all};

pub const DEFAULT_PORT: u16 = 9993;

pub struct HyperDeck {
    stream: TcpStream,
    read_buffer: String,
}

impl HyperDeck {
    pub fn connect(addr: &SocketAddr) -> impl Future<Item=Self, Error=SimpleError> {
        TcpStream::connect(addr)
            .timeout(Duration::from_secs(2))
            .and_then(|stream| {
                future::ok(HyperDeck{
                    stream: stream,
                    read_buffer: String::new(),
                })
            })
            .map_err(|e| SimpleError::with("hyperdeck connect error", e))
    }

    /// Reads the next response from the HyperDeck, including asynchronous responses.
    pub fn read_response(self) -> impl Future<Item=(Self, HyperDeckResponse), Error=SimpleError> {
        HyperDeckResponseFuture{
            hyperdeck: Some(self),
        }
            .timeout(Duration::from_secs(2))
            .map_err(|e| SimpleError::with("hyperdeck read response error", e))
    }

    pub fn write_command(self, cmd: String) -> impl Future<Item=Self, Error=SimpleError> {
        let read_buffer = self.read_buffer;
        write_all(self.stream, cmd + "\n")
            .map(|(stream, _)| HyperDeck{
                stream: stream,
                read_buffer: read_buffer,
            })
            .map_err(|e| SimpleError::with("hyperdeck write command error", e))
    }

    /// Reads responses until a command response (a response that does not have a 5XX code) is
    /// found.
    pub fn read_command_response(self) -> impl Future<Item=(Self, HyperDeckResponse), Error=SimpleError> {
        let state = (self, None);
        future::loop_fn(state, move |state: (Self, Option<HyperDeckResponse>)| {
            state.0.read_response()
            .and_then(|(hyperdeck, response)| {
                match response.code {
                    500...600 => Ok(future::Loop::Continue((hyperdeck, None))),
                    _ => Ok(future::Loop::Break((hyperdeck, Some(response)))),
                }
            })
        })
            .map(|state| (state.0, state.1.unwrap()))
    }
}

#[derive(Clone, Debug)]
pub struct HyperDeckResponse {
    pub code: i32,
    pub text: String,
    pub payload: Option<String>,
}

impl HyperDeckResponse {
    pub fn parse_payload_parameters<'a>(&'a self) -> Result<HashMap<&'a str, &'a str>, SimpleError> {
        match &self.payload {
            None => Ok(HashMap::new()),
            Some(payload) => {
                let mut params = HashMap::new();
                for line in payload.lines() {
                    let parts: Vec<&str> = line.splitn(2, ':').collect();
                    if parts.len() < 2 {
                        return Err(SimpleError::new("malformed parameters"));
                    }
                    let key = parts[0].trim();
                    let value = if parts.len() > 1 { parts[1].trim() } else { "" };
                    params.insert(key, value);
                }
                Ok(params)
            },
        }
    }
}

struct HyperDeckResponseFuture {
    hyperdeck: Option<HyperDeck>,
}

impl HyperDeckResponseFuture {
    fn try_parse_response(&mut self) -> Result<Option<HyperDeckResponse>, SimpleError> {
        let mut response: Option<HyperDeckResponse> = None;

        let hyperdeck = self.hyperdeck.as_mut().unwrap();

        let read_buffer = hyperdeck.read_buffer.clone();
        let complete_lines = read_buffer.trim_end_matches(|c| c != '\n' && c != '\r');
        let mut consumed_lines = 0;

        for line in complete_lines.lines() {
            consumed_lines += 1;

            if line.is_empty() {
                match response {
                    Some(ref response) => {
                        let (_, remaining) = read_buffer.split_at(hyperdeck.read_buffer.match_indices('\n').nth(consumed_lines - 1).unwrap_or((read_buffer.len(), "")).0);
                        hyperdeck.read_buffer = remaining.to_string();
                        return Ok(Some(response.clone()));
                    },
                    _ => response = None,
                }
                continue;
            }

            match response {
                Some(ref mut response) => {
                    let mut payload = response.payload.get_or_insert(String::new());
                    payload.push_str(line);
                    payload.push_str("\n");
                },
                None => {
                    let parts: Vec<&str> = line.splitn(2, ' ').collect();
                    if parts.len() < 2 {
                        return Err(SimpleError::new("malformed response code line"));
                    }
                    let code = parts[0].parse();
                    if code.is_err() {
                        return Err(SimpleError::new("malformed response code"));
                    }
                    let text = parts[1];
                    if !text.ends_with(':') {
                        let (_, remaining) = read_buffer.split_at(hyperdeck.read_buffer.match_indices('\n').nth(consumed_lines - 1).unwrap_or((read_buffer.len(), "")).0);
                        hyperdeck.read_buffer = remaining.to_string();
                        return Ok(Some(HyperDeckResponse{
                            code: code.unwrap(),
                            text: text.to_string(),
                            payload: None,
                        }));
                    }
                    response = Some(HyperDeckResponse{
                        code: code.unwrap(),
                        text: text.trim_end_matches(':').to_string(),
                        payload: None,
                    });
                },
            }
        }

        Ok(None)
    }
}

impl Future for HyperDeckResponseFuture {
    type Item = (HyperDeck, HyperDeckResponse);
    type Error = SimpleError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.try_parse_response() {
            Ok(Some(response)) => return Ok(Async::Ready((self.hyperdeck.take().unwrap(), response))),
            Ok(None) => {},
            Err(e) => return Err(e),
        }

        {
            let hyperdeck = self.hyperdeck.as_mut().unwrap();
            let mut buf = [0; 1024];
            match hyperdeck.stream.poll_read(&mut buf) {
                Ok(Async::Ready(0)) => return Err(SimpleError::new("unexpected eof")),
                Ok(Async::Ready(n)) => match std::str::from_utf8(&buf[..n]) {
                    Ok(s) => hyperdeck.read_buffer.push_str(s),
                    Err(e) => return Err(SimpleError::with("malformed response", e))
                },
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(e) => return Err(SimpleError::with("read error", e)),
            }
        }

        match self.try_parse_response() {
            Ok(Some(response)) => Ok(Async::Ready((self.hyperdeck.take().unwrap(), response))),
            Ok(None) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HyperDeck;

    use actix::{Arbiter, System};
    use futures::{future, Future, Stream};
    use tokio::io::{write_all};
    use tokio::net::{TcpListener};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn hyperdeck_read_command_response() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let listener = TcpListener::bind(&addr).unwrap();

        System::run(|| {
            let addr = listener.local_addr().unwrap();

            Arbiter::spawn(
                HyperDeck::connect(&addr)
                    .and_then(|hyperdeck| hyperdeck.read_command_response())
                    .then(|result| {
                        let (_, result) = result.unwrap();
                        assert_eq!(200, result.code);
                        assert_eq!("hello", result.text);

                        System::current().stop();
                        future::result(Ok(()))
                    })
            );

            Arbiter::spawn(
                listener.incoming()
                    .take(1)
                    .collect()
                    .and_then(|mut clients| {
                        write_all(clients.remove(0), b"500 init\n200 hello\n")
                    })
                    .then(|_| future::result(Ok(())))
            );
        });
    }

    #[test]
    fn hyperdeck_read_response() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let listener = TcpListener::bind(&addr).unwrap();

        System::run(|| {
            let addr = listener.local_addr().unwrap();

            Arbiter::spawn(
                HyperDeck::connect(&addr)
                    .and_then(|hyperdeck| hyperdeck.read_response())
                    .then(|result| {
                        let (_, result) = result.unwrap();
                        assert_eq!(200, result.code);
                        assert_eq!("hello", result.text);
                        assert_eq!("foo\n", result.payload.unwrap());

                        System::current().stop();
                        future::result(Ok(()))
                    })
            );

            Arbiter::spawn(
                listener.incoming()
                    .take(1)
                    .collect()
                    .and_then(|mut clients| {
                        write_all(clients.remove(0), b"200 hello:\nfoo\n\n")
                    })
                    .then(|_| future::result(Ok(())))
            );
        });
    }

    #[test]
    fn hyperdeck_connect_timeout() {
        System::run(|| {
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 0)), super::DEFAULT_PORT);
            Arbiter::spawn(
                HyperDeck::connect(&addr)
                    .then(|result| {
                        assert!(result.is_err());
                        System::current().stop();
                        future::result(Ok(()))
                    })
            );
        });
    }
}
