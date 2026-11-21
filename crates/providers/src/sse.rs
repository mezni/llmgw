#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    pub data: String,
}

#[derive(Debug, Default)]
pub struct SseDecoder {
    buffer: Vec<u8>,
    data_lines: Vec<String>,
}

impl SseDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        self.buffer.extend_from_slice(chunk);
        self.drain_lines(false)
    }

    pub fn finish(&mut self) -> Vec<SseEvent> {
        self.drain_lines(true)
    }

    fn drain_lines(&mut self, final_flush: bool) -> Vec<SseEvent> {
        let mut events = Vec::new();
        while let Some(position) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.buffer.drain(..=position).collect();
            self.process_line(&line, &mut events);
        }
        if final_flush {
            if !self.buffer.is_empty() {
                let line = std::mem::take(&mut self.buffer);
                self.process_line(&line, &mut events);
            }
            self.flush_event(&mut events);
        }
        events
    }

    fn process_line(&mut self, line: &[u8], events: &mut Vec<SseEvent>) {
        let line = String::from_utf8_lossy(line);
        let line = line.strip_suffix('\n').unwrap_or(&line);
        let line = line.strip_suffix('\r').unwrap_or(line);

        if line.is_empty() {
            self.flush_event(events);
        } else if let Some(data) = line.strip_prefix("data:") {
            self.data_lines
                .push(data.strip_prefix(' ').unwrap_or(data).to_owned());
        }
    }

    fn flush_event(&mut self, events: &mut Vec<SseEvent>) {
        if self.data_lines.is_empty() {
            return;
        }
        events.push(SseEvent {
            data: std::mem::take(&mut self.data_lines).join("\n"),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STREAM_FIXTURE: &str = include_str!("../../../tests/fixtures/sse/stream-ok.sse");

    #[test]
    fn parses_events_split_across_chunks() {
        let mut decoder = SseDecoder::new();
        let fixture = STREAM_FIXTURE.as_bytes();
        let mut events = Vec::new();
        for byte in fixture {
            events.extend(decoder.push(&[*byte]));
        }
        events.extend(decoder.finish());

        assert_eq!(events.len(), 4);
        assert!(events[0].data.contains("\"content\":\"He\""));
        assert_eq!(events[3].data, "[DONE]");
    }

    #[test]
    fn parses_multiple_events_in_one_chunk() {
        let mut decoder = SseDecoder::new();
        let events = decoder.push(b"data: first\n\ndata: second\n\n");
        assert_eq!(
            events,
            vec![
                SseEvent {
                    data: "first".to_owned()
                },
                SseEvent {
                    data: "second".to_owned()
                }
            ]
        );
    }

    #[test]
    fn handles_crlf_comments_and_multiline_data() {
        let mut decoder = SseDecoder::new();
        let events =
            decoder.push(b": keepalive\r\nevent: message\r\ndata: one\r\ndata: two\r\n\r\n");
        assert_eq!(
            events,
            vec![SseEvent {
                data: "one\ntwo".to_owned()
            }]
        );
    }

    #[test]
    fn flushes_trailing_event_without_blank_line() {
        let mut decoder = SseDecoder::new();
        assert!(decoder.push(b"data: tail\n").is_empty());
        assert_eq!(
            decoder.finish(),
            vec![SseEvent {
                data: "tail".to_owned()
            }]
        );
    }
}
