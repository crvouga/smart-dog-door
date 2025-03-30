use crate::device_dog_door::interface::DeviceDogDoor;
use crate::logger::interface::Logger;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct DeviceDogDoorFake {
    locked: AtomicBool,
    logger: Box<dyn Logger>,
}

impl DeviceDogDoorFake {
    pub fn new(logger: Box<dyn Logger>) -> Self {
        Self {
            locked: AtomicBool::new(false),
            logger: logger.with_namespace("dog_door").with_namespace("fake"),
        }
    }
}

impl DeviceDogDoor for DeviceDogDoorFake {
    fn lock(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.logger.info("Locking dog door...")?;
        self.locked.store(true, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_secs(1));
        self.logger.info("Dog door locked")?;
        Ok(())
    }

    fn unlock(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.logger.info("Unlocking dog door...")?;
        self.locked.store(false, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_secs(1));
        self.logger.info("Dog door unlocked")?;
        Ok(())
    }

    fn is_unlocked(&self) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(self.locked.load(Ordering::SeqCst))
    }

    fn events(&self) -> std::sync::mpsc::Sender<DogDoorEvent> {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                tx.send(event).unwrap();
            }
        });
        tx
    }
}
