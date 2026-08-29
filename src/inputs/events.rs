use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use super::{key::Key, InputEvent};

pub struct Events {
    rx: tokio::sync::mpsc::Receiver<InputEvent>,
    _tx: tokio::sync::mpsc::Sender<InputEvent>,
    stop_capture: Arc<AtomicBool>,
}

impl Events {
    pub fn new(tick_rate: Duration) -> Events {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let stop_capture = Arc::new(AtomicBool::new(false));

        let event_tx = tx.clone();
        let event_stop_capture = stop_capture.clone();
        tokio::spawn(async move {
            loop {
                match crossterm::event::poll(tick_rate) {
                    Ok(true) => match crossterm::event::read() {
                        Ok(crossterm::event::Event::Key(key)) => {
                            let _ = event_tx.send(InputEvent::Input(Key::from(key))).await;
                        }
                        Ok(crossterm::event::Event::Mouse(mouse)) => {
                            let _ = event_tx.send(InputEvent::Mouse(mouse)).await;
                        }
                        Ok(crossterm::event::Event::Resize(width, height)) => {
                            let _ = event_tx.send(InputEvent::Resize(width, height)).await;
                        }
                        Ok(_) => {}
                        Err(_) => break,
                    },
                    Ok(false) => {}
                    Err(_) => break,
                }
                let _ = event_tx.send(InputEvent::Tick).await;
                if event_stop_capture.load(Ordering::Relaxed) {
                    break;
                }
            }
        });

        Events {
            rx,
            _tx: tx,
            stop_capture,
        }
    }

    /// Attempts to read an event.
    pub async fn next(&mut self) -> InputEvent {
        self.rx.recv().await.unwrap_or(InputEvent::Tick)
    }

    /// Close
    pub fn close(&mut self) {
        self.stop_capture.store(true, Ordering::Relaxed)
    }
}
