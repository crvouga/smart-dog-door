use crate::device_door::interface::{DeviceDoor, DeviceDoorEvent};
use crate::library::logger::interface::Logger;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
pub struct DeviceDoorFake {
    locked: AtomicBool,
    logger: Arc<dyn Logger + Send + Sync>,
}

impl DeviceDoorFake {
    pub fn new(logger: Arc<dyn Logger + Send + Sync>) -> Self {
        Self {
            locked: AtomicBool::new(false),
            logger: logger.with_namespace("dog_door").with_namespace("fake"),
        }
    }
}

impl DeviceDoor for DeviceDoorFake {
    fn lock(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.info("Locking dog door...")?;
        self.locked.store(true, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_secs(1));
        self.logger.info("Dog door locked")?;
        Ok(())
    }

    fn unlock(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.info("Unlocking dog door...")?;
        self.locked.store(false, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_secs(1));
        self.logger.info("Dog door unlocked")?;
        Ok(())
    }

    fn is_unlocked(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        Ok(!self.locked.load(Ordering::SeqCst))
    }

    fn events(&self) -> std::sync::mpsc::Receiver<DeviceDoorEvent> {
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            tx.send(DeviceDoorEvent::Connected).unwrap();

            loop {
                std::thread::sleep(std::time::Duration::from_secs(300)); // Sleep for 5 minutes

                // 1% chance of disconnecting
                if rand::random::<f32>() < 0.01 {
                    tx.send(DeviceDoorEvent::Disconnected).unwrap();
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    tx.send(DeviceDoorEvent::Connected).unwrap();
                }
            }
        });

        rx
    }
}
