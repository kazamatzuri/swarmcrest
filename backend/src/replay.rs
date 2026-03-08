// Replay recording: collects game messages and compresses them for storage.

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::{Read, Write};

/// Collects raw JSON game messages during a match and compresses them on finish.
pub struct ReplayRecorder {
    messages: Vec<String>,
}

impl Default for ReplayRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplayRecorder {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Record a raw JSON message string.
    pub fn record_message(&mut self, msg: &str) {
        self.messages.push(msg.to_string());
    }

    /// Returns the number of recorded messages (tick count proxy).
    pub fn tick_count(&self) -> i32 {
        self.messages.len() as i32
    }

    /// Compress all recorded messages into a gzipped JSON array.
    pub fn finish(self) -> Vec<u8> {
        // Build a JSON array from the raw message strings.
        // Each message is already valid JSON, so we join them manually.
        let mut json = String::from("[");
        for (i, msg) in self.messages.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            json.push_str(msg);
        }
        json.push(']');

        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(json.as_bytes()).expect("gzip write");
        encoder.finish().expect("gzip finish")
    }
}

/// Decompress gzipped replay data back to the JSON string.
pub fn decompress_replay(data: &[u8]) -> Result<String, std::io::Error> {
    let mut decoder = GzDecoder::new(data);
    let mut result = String::new();
    decoder.read_to_string(&mut result)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_decompress() {
        let mut recorder = ReplayRecorder::new();
        recorder.record_message(r#"{"type":"world","width":10}"#);
        recorder.record_message(r#"{"type":"snapshot","game_time":100}"#);
        recorder.record_message(r#"{"type":"game_end","winner":1}"#);

        assert_eq!(recorder.tick_count(), 3);

        let compressed = recorder.finish();
        assert!(!compressed.is_empty());

        // Decompress and verify
        let json_str = decompress_replay(&compressed).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0]["type"], "world");
        assert_eq!(arr[1]["type"], "snapshot");
        assert_eq!(arr[2]["type"], "game_end");
    }

    #[test]
    fn test_empty_recorder() {
        let recorder = ReplayRecorder::new();
        assert_eq!(recorder.tick_count(), 0);

        let compressed = recorder.finish();
        let json_str = decompress_replay(&compressed).unwrap();
        assert_eq!(json_str, "[]");
    }
}
