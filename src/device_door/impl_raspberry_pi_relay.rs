use crate::device_dog_door::interface::DeviceDoor;
use crate::logger::interface::Logger;
use rppal::gpio::Gpio;
use std::error::Error;

pub struct DogDoorRaspberryPiConfig {
    pub gpio_pin: u8,
}

impl Default for DogDoorRaspberryPiConfig {
    fn default() -> Self {
        Self {
            gpio_pin: 17, // Default to GPIO17
        }
    }
}

pub struct DogDoorRaspberryPiRelay {
    config: DogDoorRaspberryPiConfig,
    logger: Box<dyn Logger>,
    event_thread: Option<std::thread::JoinHandle<()>>,
    shutdown_tx: Option<std::sync::mpsc::Sender<()>>,
    is_locked: bool,
}

impl DogDoorRaspberryPiRelay {
    pub fn new(
        config: DogDoorRaspberryPiConfig,
        logger: Box<dyn Logger>,
    ) -> Result<Self, Box<dyn Error>> {
        let gpio = Gpio::new()?;

        let relay_pin = gpio.get(config.gpio_pin)?.into_output();

        logger.info(&format!(
            "Initializing dog door relay on GPIO {}",
            config.gpio_pin
        ))?;

        Ok(Self {
            config,
            logger: logger.with_namespace("raspberry_pi_dog_door"),
            event_thread: None,
            shutdown_tx: None,
            is_locked: false,
        })
    }
}

impl DeviceDoor for DogDoorRaspberryPiRelay {
    fn lock(&self) -> Result<(), Box<dyn Error>> {
        if !self.is_locked {
            self.logger.info("Engaging electromagnetic lock")?;

            // Turn relay ON which powers the electromagnet (locks door)
            self.relay_pin.set_high();

            self.is_locked = true;
            self.logger.info("Door locked")?;
        }
        Ok(())
    }

    fn unlock(&self) -> Result<(), Box<dyn Error>> {
        if self.is_locked {
            self.logger.info("Disengaging electromagnetic lock")?;

            // Turn relay OFF which cuts power to electromagnet (unlocks door)
            self.relay_pin.set_low();

            self.is_locked = false;
            self.logger.info("Door unlocked")?;
        }
        Ok(())
    }

    fn is_unlocked(&self) -> Result<bool, Box<dyn Error>> {
        Ok(!self.is_locked)
    }

    fn events(&self) -> std::sync::mpsc::Sender<DoorEvent> {
        let (event_tx, event_rx) = std::sync::mpsc::channel();
        let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel();

        let handle = std::thread::spawn(move || {
            let gpio = Gpio::new()?;
            let relay_pin = gpio.get(self.config.gpio_pin)?.into_input();

            loop {
                // Check for shutdown signal
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }

                let value = relay_pin.read();
                if value == 0 {
                    if event_tx.send(DoorEvent::Connected).is_err() {
                        break; // Exit if receiver is dropped
                    }
                } else {
                    if event_tx.send(DoorEvent::Disconnected).is_err() {
                        break; // Exit if receiver is dropped
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });

        self.event_thread = Some(handle);
        self.shutdown_tx = Some(shutdown_tx);

        event_tx
    }
}

impl Drop for DogDoorRaspberryPiRelay {
    fn drop(&mut self) {
        // Signal thread to shut down
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Wait for thread to finish
        if let Some(handle) = self.event_thread.take() {
            let _ = handle.join();
        }

        // Safety: Always ensure door is unlocked when program exits
        // This prevents dog from getting locked out if program crashes
        if let Err(e) = self.unlock() {
            eprintln!("Failed to unlock door during shutdown: {}", e);
        }
    }
}
