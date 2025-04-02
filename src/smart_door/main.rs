use crate::config::Config;
use crate::device_camera::interface::DeviceCamera;
use crate::device_display::interface::DeviceDisplay;
use crate::device_door::interface::DeviceDoor;
use crate::image_classifier::interface::ImageClassifier;
use crate::library::logger::interface::Logger;
use crate::smart_door::core::{init, transition, Effect, Model, Msg};
use crate::smart_door::render::Render;
use crate::smart_door::run_effect::RunEffect;
use std::io;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SmartDoor {
    model: Arc<Mutex<Model>>,
    event_receiver: Arc<Mutex<Receiver<Msg>>>,
    config: Config,
    logger: Arc<dyn Logger + Send + Sync>,
    effect_runner: RunEffect,
    renderer: Render,
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
        let initial = init();

        let effect_runner = RunEffect::new(
            config.clone(),
            logger.clone(),
            device_camera,
            device_door,
            image_classifier,
            event_sender.clone(),
        );

        let renderer = Render::new(device_display, config.clone());

        Self {
            config,
            logger,
            effect_runner,
            renderer,
            event_receiver: Arc::new(Mutex::new(event_receiver)),
            model: Arc::new(Mutex::new(initial.0)),
        }
    }

    fn spawn_effects(&self, effects: Vec<Effect>) {
        for effect in effects {
            let effect_clone = effect.clone();
            let self_clone = self.clone();
            std::thread::spawn(move || self_clone.effect_runner.run_effect(effect_clone));
        }
    }

    fn run_loop(&self) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        let initial = init();
        *self.model.lock().unwrap() = initial.0.clone();

        self.spawn_effects(initial.1);

        let mut current_model = initial.0.clone();

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
                    *self.model.lock().unwrap() = new_model;

                    if let Err(e) = self.renderer.render(&self.model.lock().unwrap()) {
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
