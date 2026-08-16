use std::collections::{HashMap, VecDeque};

use ratatui_image::{picker::Picker, protocol::StatefulProtocol};
use tokio::sync::mpsc;

const MAX_IMAGE_BYTES: usize = 12 * 1024 * 1024;
const MAX_CACHED_IMAGES: usize = 512;

enum ImageEntry {
    Loading,
    Ready(StatefulProtocol),
    Failed,
}

struct DownloadedImage {
    url: String,
    bytes: Option<Vec<u8>>,
}

pub struct ImageCache {
    picker: Option<Picker>,
    entries: HashMap<String, ImageEntry>,
    insertion_order: VecDeque<String>,
    completed_tx: mpsc::UnboundedSender<DownloadedImage>,
    completed_rx: mpsc::UnboundedReceiver<DownloadedImage>,
    client: reqwest::Client,
}

impl ImageCache {
    pub fn new() -> Self {
        let (completed_tx, completed_rx) = mpsc::unbounded_channel();
        Self {
            picker: None,
            entries: HashMap::new(),
            insertion_order: VecDeque::new(),
            completed_tx,
            completed_rx,
            client: reqwest::Client::new(),
        }
    }

    pub fn configure(&mut self, mut picker: Picker) {
        picker.set_background_color([0, 0, 0, 0]);
        self.picker = Some(picker);
    }

    pub fn queue<I>(&mut self, urls: I)
    where
        I: IntoIterator<Item = String>,
    {
        for url in urls {
            if url.is_empty() || self.entries.contains_key(&url) {
                continue;
            }
            if !self.make_room() {
                break;
            }

            self.entries.insert(url.clone(), ImageEntry::Loading);
            self.insertion_order.push_back(url.clone());
            let client = self.client.clone();
            let completed_tx = self.completed_tx.clone();
            tokio::spawn(async move {
                let bytes = download(&client, &url).await;
                let _ = completed_tx.send(DownloadedImage { url, bytes });
            });
        }
    }

    pub fn poll(&mut self) {
        for entry in self.entries.values_mut() {
            if let ImageEntry::Ready(protocol) = entry {
                let _ = protocol.last_encoding_result();
            }
        }

        while let Ok(downloaded) = self.completed_rx.try_recv() {
            let entry = downloaded
                .bytes
                .and_then(|bytes| image::load_from_memory(&bytes).ok())
                .and_then(|image| {
                    self.picker
                        .as_ref()
                        .map(|picker| picker.new_resize_protocol(image))
                })
                .map(ImageEntry::Ready)
                .unwrap_or(ImageEntry::Failed);
            self.entries.insert(downloaded.url, entry);
        }
    }

    pub fn get_mut(&mut self, url: &str) -> Option<&mut StatefulProtocol> {
        match self.entries.get_mut(url) {
            Some(ImageEntry::Ready(protocol)) => Some(protocol),
            _ => None,
        }
    }

    fn make_room(&mut self) -> bool {
        if self.entries.len() < MAX_CACHED_IMAGES {
            return true;
        }

        for _ in 0..self.insertion_order.len() {
            let Some(oldest) = self.insertion_order.pop_front() else {
                break;
            };
            if matches!(self.entries.get(&oldest), Some(ImageEntry::Loading)) {
                self.insertion_order.push_back(oldest);
            } else {
                self.entries.remove(&oldest);
                return true;
            }
        }
        false
    }
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}

async fn download(client: &reqwest::Client, url: &str) -> Option<Vec<u8>> {
    let response = client.get(url).send().await.ok()?.error_for_status().ok()?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_IMAGE_BYTES as u64)
    {
        return None;
    }

    let bytes = response.bytes().await.ok()?;
    (bytes.len() <= MAX_IMAGE_BYTES).then(|| bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_urls_are_only_queued_once() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let mut cache = ImageCache::new();
            cache.queue(["https://example.invalid/avatar.png".to_string()]);
            cache.queue(["https://example.invalid/avatar.png".to_string()]);
            assert_eq!(cache.entries.len(), 1);
        });
    }

    #[test]
    fn completed_entries_are_evicted_at_the_cache_limit() {
        let mut cache = ImageCache::new();
        for index in 0..MAX_CACHED_IMAGES {
            let url = format!("image-{index}");
            cache.entries.insert(url.clone(), ImageEntry::Failed);
            cache.insertion_order.push_back(url);
        }

        assert!(cache.make_room());
        assert_eq!(cache.entries.len(), MAX_CACHED_IMAGES - 1);
        assert!(!cache.entries.contains_key("image-0"));
    }
}
