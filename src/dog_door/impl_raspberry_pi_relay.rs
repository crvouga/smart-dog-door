use crate::dog_door::interface::DogDoor;
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
    relay_pin: rppal::gpio::OutputPin,
    logger: Box<dyn Logger>,
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
            relay_pin,
            logger: logger.with_namespace("raspberry_pi_dog_door"),
            is_locked: false,
        })
    }
}

impl DogDoor for DogDoorRaspberryPiRelay {
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
}

impl Drop for DogDoorRaspberryPiRelay {
    fn drop(&mut self) {
        // Safety: Always ensure door is unlocked when program exits
        // This prevents dog from getting locked out if program crashes
        if let Err(e) = self.unlock() {
            eprintln!("Failed to unlock door during shutdown: {}", e);
        }
    }
}
