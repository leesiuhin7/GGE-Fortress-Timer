use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Bytes;

pub(crate) struct Response {
    pub(crate) command: String,
    pub(crate) payload: String,
}

fn deserialize_responses(responses: &str) -> Vec<Response> {
    responses
        .split("%xt")
        .filter_map(|response| {
            if response.is_empty() {
                None
            } else {
                let mut components = response.strip_prefix('%')?.strip_suffix('%')?.split('%');
                let command = components.next()?;
                let payload = components.nth(2)?;
                Some(Response {
                    command: command.to_string(),
                    payload: payload.to_string(),
                })
            }
        })
        .collect::<Vec<_>>()
}

pub(crate) enum Error {
    Recv,
    Utf8,
}

pub(crate) struct Receiver {
    receiver: mpsc::Receiver<Bytes>,
}

impl Receiver {
    pub(super) fn new(receiver: mpsc::Receiver<Bytes>) -> Self {
        Receiver { receiver }
    }

    pub(crate) async fn recv(&mut self) -> Result<Vec<Response>, Error> {
        let bytes = self.receiver.recv().await.ok_or(Error::Recv)?;
        let responses = std::str::from_utf8(&bytes).map_err(|_| Error::Utf8)?;
        Ok(deserialize_responses(responses))
    }
}
