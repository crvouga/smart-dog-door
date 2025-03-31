mod client_console;
mod config;
mod device_camera;
mod device_display;
mod device_door;
mod image_classifier;
mod library;
mod smart_door;

fn main() {
    client_console::main().unwrap();
    // client_desktop_gui::main().unwrap();
}
