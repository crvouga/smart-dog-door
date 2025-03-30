use crate::dog_door::interface::DogDoor;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct FakeDogDoor {
    locked: AtomicBool,
}

impl FakeDogDoor {
    pub fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
        }
    }
}

impl DogDoor for FakeDogDoor {
    fn lock(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Locking fake dog door...");
        self.locked.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn unlock(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Unlocking fake dog door...");
        self.locked.store(false, Ordering::SeqCst);
        Ok(())
    }
}
