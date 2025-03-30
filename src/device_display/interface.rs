use std::error::Error;

/// Represents a 16x2 LCD display module with I2C interface
pub trait DeviceDisplay: Send + Sync {
    /// Initialize the I2C LCD display hardware
    #[allow(dead_code)]
    fn init(&mut self) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Clear all text from the LCD display
    #[allow(dead_code)]
    fn clear(&mut self) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Write text to a specific line on the display (0-based index)
    /// Returns error if line number is invalid (must be 0 or 1)
    fn write_line(&mut self, line: u8, text: &str) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Get the number of lines supported by this display (always 2)
    #[allow(dead_code)]
    fn num_lines(&self) -> u8 {
        2
    }

    /// Get the number of characters per line supported by this display (always 16)
    #[allow(dead_code)]
    fn chars_per_line(&self) -> u8 {
        16
    }

    /// Set the backlight on or off
    #[allow(dead_code)]
    fn set_backlight(&mut self, on: bool) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Set the cursor position (column 0-15, row 0-1)
    #[allow(dead_code)]
    fn set_cursor(&mut self, column: u8, row: u8) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Create a custom character in CGRAM at given location (0-7)
    #[allow(dead_code)]
    fn create_char(
        &mut self,
        location: u8,
        char_map: [u8; 8],
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}
