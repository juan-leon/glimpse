use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{self, Duration, Instant};

/// A trait for objects that can be "ticked" to update their state.
pub trait Tickable: Send + Sync {
    fn tick(&mut self);
}

/// Holds information about a tickable object and its frequency
struct TickableEntry {
    tickable: Box<dyn Tickable>,
    frequency: u32, // Frequency in milliseconds
    last_tick: Instant,
    next_tick: Instant,
}

impl TickableEntry {
    fn new(tickable: Box<dyn Tickable>, frequency: u32) -> Self {
        let now = Instant::now();
        Self {
            tickable,
            frequency,
            last_tick: now,
            next_tick: now + Duration::from_millis(frequency as u64),
        }
    }

    fn update_next_tick(&mut self) {
        self.next_tick = self.last_tick + Duration::from_millis(self.frequency as u64);
    }
}

/// Manages multiple Tickable objects and updates them at specified frequencies
struct Ticker {
    entries: Arc<RwLock<Vec<TickableEntry>>>,
    running: Arc<RwLock<bool>>,
}

impl Ticker {
    /// Creates a new Ticker instance
    pub fn new() -> Self {
        Ticker {
            entries: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Starts the ticker loop
    pub async fn start(&self) {
        {
            let mut running = self.running.write().await;
            if *running {
                return;
            }
            *running = true;
        }

        let entries = self.entries.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            while *running.read().await {
                let now = Instant::now();
                let next_tick = {
                    // Read lock scope for finding next tick
                    let entries = entries.read().await;
                    entries
                        .iter()
                        .map(|entry| entry.next_tick)
                        .min()
                        .unwrap_or(now + Duration::from_secs(1))
                };

                if next_tick > now {
                    time::sleep(next_tick - now).await;
                }

                // Write lock scope for updating entries
                let mut entries = entries.write().await;
                for entry in entries.iter_mut() {
                    if now >= entry.next_tick {
                        entry.tickable.tick();
                        entry.last_tick = now;
                        entry.update_next_tick();
                    }
                }
            }
        });
    }

    /// Stops the ticker loop
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }

    /// Adds a new Tickable object with the specified frequency
    pub async fn add_tickable<T: Tickable + 'static>(&mut self, frequency: u32, tickable: T) {
        let entry = TickableEntry::new(Box::new(tickable), frequency);
        let mut entries = self.entries.write().await;
        entries.push(entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    struct TestTickable {
        counter: Arc<AtomicU32>,
    }

    impl Tickable for TestTickable {
        fn tick(&mut self) {
            self.counter.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn test_ticker() {
        let mut ticker = Ticker::new();
        let counter = Arc::new(AtomicU32::new(0));

        let test_tickable = TestTickable {
            counter: counter.clone(),
        };

        ticker.add_tickable(100, test_tickable).await;
        ticker.start().await;

        // Wait for some ticks to occur
        tokio::time::sleep(Duration::from_millis(350)).await;
        ticker.stop().await;

        // Should have 3 ticks
        assert!(counter.load(Ordering::SeqCst) == 3);
    }

    #[tokio::test]
    async fn test_multiple_frequencies() {
        let mut ticker = Ticker::new();
        let counter1 = Arc::new(AtomicU32::new(0));
        let counter2 = Arc::new(AtomicU32::new(0));

        ticker
            .add_tickable(
                100,
                TestTickable {
                    counter: counter1.clone(),
                },
            )
            .await;
        ticker
            .add_tickable(
                200,
                TestTickable {
                    counter: counter2.clone(),
                },
            )
            .await;

        ticker.start().await;
        tokio::time::sleep(Duration::from_millis(550)).await;
        ticker.stop().await;

        // First counter should have 5 ticks, second counter should have 2 ticks
        assert!(counter1.load(Ordering::SeqCst) == 5);
        assert!(counter2.load(Ordering::SeqCst) == 2);
    }
}
