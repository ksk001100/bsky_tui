use std::collections::{HashMap, VecDeque};

use ratatui::layout::Rect;
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, FontSize};
use tokio::sync::mpsc;

const MAX_IMAGE_BYTES: usize = 12 * 1024 * 1024;
const MAX_CACHED_IMAGES: usize = 64;

enum ImageEntry {
    Loading,
    Ready {
        protocol: Box<StatefulProtocol>,
        pixel_dimensions: (u32, u32),
    },
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
        picker.set_background_color(Some([0, 0, 0, 0]));
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
            if let ImageEntry::Ready { protocol, .. } = entry {
                let _ = protocol.last_encoding_result();
            }
        }

        while let Ok(downloaded) = self.completed_rx.try_recv() {
            let entry = downloaded
                .bytes
                .and_then(|bytes| image::load_from_memory(&bytes).ok())
                .and_then(|image| {
                    let pixel_dimensions = (image.width(), image.height());
                    self.picker.as_ref().map(|picker| ImageEntry::Ready {
                        protocol: Box::new(picker.new_resize_protocol(image)),
                        pixel_dimensions,
                    })
                })
                .unwrap_or(ImageEntry::Failed);
            self.entries.insert(downloaded.url, entry);
        }
    }

    pub fn get_mut(&mut self, url: &str) -> Option<&mut StatefulProtocol> {
        match self.entries.get_mut(url) {
            Some(ImageEntry::Ready { protocol, .. }) => Some(protocol.as_mut()),
            _ => None,
        }
    }

    pub fn centered_area(&self, url: &str, bounds: Rect) -> Option<Rect> {
        let font_size = self.picker.as_ref()?.font_size();
        let ImageEntry::Ready {
            pixel_dimensions, ..
        } = self.entries.get(url)?
        else {
            return None;
        };
        Some(centered_image_area(bounds, *pixel_dimensions, font_size))
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

fn centered_image_area(bounds: Rect, pixel_dimensions: (u32, u32), font_size: FontSize) -> Rect {
    if bounds.width == 0 || bounds.height == 0 || pixel_dimensions.0 == 0 || pixel_dimensions.1 == 0
    {
        return Rect::new(bounds.x, bounds.y, 0, 0);
    }

    let available_width = bounds.width as f64 * font_size.width.max(1) as f64;
    let available_height = bounds.height as f64 * font_size.height.max(1) as f64;
    let scale = (available_width / pixel_dimensions.0 as f64)
        .min(available_height / pixel_dimensions.1 as f64);
    let width = ((pixel_dimensions.0 as f64 * scale) / font_size.width.max(1) as f64)
        .ceil()
        .clamp(1.0, bounds.width as f64) as u16;
    let height = ((pixel_dimensions.1 as f64 * scale) / font_size.height.max(1) as f64)
        .ceil()
        .clamp(1.0, bounds.height as f64) as u16;

    Rect::new(
        bounds.x + bounds.width.saturating_sub(width) / 2,
        bounds.y + bounds.height.saturating_sub(height) / 2,
        width,
        height,
    )
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

    #[test]
    fn image_area_is_centered_using_pixel_and_cell_aspect_ratios() {
        let bounds = Rect::new(5, 3, 80, 30);
        let area = centered_image_area(bounds, (1000, 500), FontSize::new(10, 20));
        assert_eq!(area, Rect::new(5, 8, 80, 20));

        let portrait = centered_image_area(bounds, (500, 1000), FontSize::new(10, 20));
        assert_eq!(portrait, Rect::new(30, 3, 30, 30));
    }
}
