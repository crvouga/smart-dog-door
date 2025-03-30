use crate::device_display::interface::DeviceDisplay;
use crate::library::logger::interface::Logger;
use std::error::Error;
use std::sync::Arc;
pub struct DeviceDisplayFake {
    logger: Arc<dyn Logger + Send + Sync>,
}

impl DeviceDisplayFake {
    pub fn new(logger: Arc<dyn Logger + Send + Sync>) -> Self {
        Self { logger }
    }
}

impl DeviceDisplay for DeviceDisplayFake {
    fn init(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.logger.info("DeviceDisplayFake::init()")?;
        Ok(())
    }

    fn clear(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.logger.info("DeviceDisplayFake::clear()")?;
        Ok(())
    }

    fn write_line(&mut self, line: u8, text: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.logger.info(&format!(
            "DeviceDisplayFake::write_line({}, {})",
            line, text
        ))?;
        Ok(())
    }

    fn set_backlight(&mut self, on: bool) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.logger
            .info(&format!("DeviceDisplayFake::set_backlight({})", on))?;
        Ok(())
    }

    fn set_cursor(&mut self, column: u8, row: u8) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.logger.info(&format!(
            "DeviceDisplayFake::set_cursor({}, {})",
            column, row
        ))?;
        Ok(())
    }

    fn create_char(
        &mut self,
        location: u8,
        char_map: [u8; 8],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.logger.info(&format!(
            "DeviceDisplayFake::create_char({}, {:?})",
            location, char_map
        ))?;
        Ok(())
    }
}
