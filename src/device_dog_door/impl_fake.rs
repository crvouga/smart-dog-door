use crate::device_dog_door::interface::{DeviceDogDoor, DogDoorEvent};
use crate::library::logger::interface::Logger;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
pub struct DeviceDogDoorFake {
    locked: AtomicBool,
    logger: Arc<dyn Logger + Send + Sync>,
}

impl DeviceDogDoorFake {
    pub fn new(logger: Arc<dyn Logger + Send + Sync>) -> Self {
        Self {
            locked: AtomicBool::new(false),
            logger: logger.with_namespace("dog_door").with_namespace("fake"),
        }
    }
}

impl DeviceDogDoor for DeviceDogDoorFake {
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

    fn events(&self) -> std::sync::mpsc::Sender<DogDoorEvent> {
        let (tx, rx) = std::sync::mpsc::channel();
        let tx_clone = tx.clone();
        std::thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                tx_clone.send(event).unwrap();
            }
        });
        tx
    }
}
