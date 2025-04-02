use crate::config::Config;
use crate::device_camera::interface::DeviceCamera;
use crate::device_display::interface::DeviceDisplay;
use crate::device_door::interface::DeviceDoor;
use crate::image_classifier::interface::ImageClassifier;
use crate::library::logger::interface::Logger;
use crate::smart_door::core::{transition, Effect, Model, Msg};
use std::io;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SmartDoor {
    pub event_sender: Sender<Msg>,
    pub event_receiver: Arc<Mutex<Receiver<Msg>>>,
    pub config: Config,
    pub logger: Arc<dyn Logger + Send + Sync>,
    pub device_camera: Arc<dyn DeviceCamera + Send + Sync>,
    pub device_door: Arc<dyn DeviceDoor + Send + Sync>,
    pub device_display: Arc<Mutex<dyn DeviceDisplay + Send + Sync>>,
    pub image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
}

impl SmartDoor {
    pub fn new(
        config: Config,
        logger: Arc<dyn Logger + Send + Sync>,
        device_camera: Arc<dyn DeviceCamera + Send + Sync>,
        device_door: Arc<dyn DeviceDoor + Send + Sync>,
        device_display: Arc<Mutex<dyn DeviceDisplay + Send + Sync>>,
        image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
    ) -> Self {
        let (event_sender, event_receiver) = channel();

        Self {
            config,
            logger,
            device_camera,
            device_door,
            device_display,
            image_classifier,
            event_sender,
            event_receiver: Arc::new(Mutex::new(event_receiver)),
        }
    }

    fn spawn_effects(&self, effects: Vec<Effect>) {
        for effect in effects {
            let effect_clone = effect.clone();
            let self_clone = self.clone();
            std::thread::spawn(move || self_clone.interpret_effect(effect_clone));
        }
    }

    fn run_loop(&self) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        let initial = Model::default();

        let mut current_model = initial.clone();

        loop {
            match self.event_receiver.lock().unwrap().recv() {
                Ok(event) => {
                    let _ = self.logger.info(&format!(
                        "\nold model:\n\t{:?}\n\nevent:\n\t{:?}",
                        current_model, event,
                    ));
                    let (new_model, effects) = transition(&self.config, current_model, event);
                    let _ = self.logger.info(&format!(
                        "\nnew model:\n\t{:?}\n\neffects:\n\t{:?}",
                        new_model, effects
                    ));

                    current_model = new_model.clone();

                    if let Err(e) = self.render(&current_model) {
                        return Err::<(), Arc<dyn std::error::Error + Send + Sync>>(Arc::new(
                            io::Error::new(io::ErrorKind::Other, e.to_string()),
                        ));
                    }

                    self.spawn_effects(effects);
                }
                Err(e) => {
                    return Err::<(), Arc<dyn std::error::Error + Send + Sync>>(Arc::new(
                        io::Error::new(io::ErrorKind::Other, e.to_string()),
                    ));
                }
            }
        }
    }

    pub fn run(&self) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        self.run_loop()?;

        Ok(())
    }
}
