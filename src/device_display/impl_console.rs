use crate::device_display::interface::DeviceDisplay;
use std::error::Error;

pub struct DeviceDisplayConsole {
    display_buffer: [[char; 16]; 2],
    backlight_on: bool,
}

impl DeviceDisplayConsole {
    pub fn new() -> Self {
        Self {
            display_buffer: [[' '; 16]; 2],
            backlight_on: true,
        }
    }

    fn render_display(&self) {
        if !self.backlight_on {
            return;
        }
        println!("┌────────────────┐");
        for row in &self.display_buffer {
            print!("│");
            for &c in row {
                print!("{}", c);
            }
            println!("│");
        }
        println!("└────────────────┘");
    }
}

impl DeviceDisplay for DeviceDisplayConsole {
    fn init(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.render_display();
        Ok(())
    }

    fn clear(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.display_buffer = [[' '; 16]; 2];
        self.render_display();
        Ok(())
    }

    fn write_line(&mut self, line: u8, text: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        if line >= 2 {
            return Err("Invalid line number".into());
        }

        // Clear the line first
        self.display_buffer[line as usize] = [' '; 16];

        // Copy characters from text, truncating if needed
        for (i, c) in text.chars().take(16).enumerate() {
            self.display_buffer[line as usize][i] = c;
        }

        self.render_display();
        Ok(())
    }

    fn set_backlight(&mut self, on: bool) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.backlight_on = on;
        self.render_display();
        Ok(())
    }

    fn set_cursor(&mut self, column: u8, row: u8) -> Result<(), Box<dyn Error + Send + Sync>> {
        if row >= 2 || column >= 16 {
            return Err("Invalid cursor position".into());
        }
        // Cursor position is just visual in console, no need to store
        Ok(())
    }

    fn create_char(
        &mut self,
        _location: u8,
        _char_map: [u8; 8],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Custom characters not supported in console
        Ok(())
    }
}
