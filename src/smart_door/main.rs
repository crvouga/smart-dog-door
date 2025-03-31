use crate::config::Config;
use crate::device_camera::interface::DeviceCamera;
use crate::device_display::interface::DeviceDisplay;
use crate::device_door::interface::DeviceDoor;
use crate::image_classifier::interface::ImageClassifier;
use crate::library::logger::interface::Logger;
use crate::smart_door::core::{init, transition, Effect, Event, State};
use crate::smart_door::render::Render;
use crate::smart_door::run_effect::RunEffect;
use std::io;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SmartDoor {
    pub state: Arc<Mutex<State>>,
    event_receiver: Arc<Mutex<Receiver<Event>>>,
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
        let (initial_state, _) = init();

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
            state: Arc::new(Mutex::new(initial_state)),
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
        let (initial_state, initial_effects) = init();
        *self.state.lock().unwrap() = initial_state.clone();

        self.spawn_effects(initial_effects);

        let mut current_state = initial_state.clone();

        loop {
            match self.event_receiver.lock().unwrap().recv() {
                Ok(event) => {
                    let _ = self.logger.info(&format!(
                        "Processing event: {:?}",
                        event.to_display_string()
                    ));

                    let (new_state, effects) = transition(&self.config, current_state, event);
                    current_state = new_state.clone();
                    *self.state.lock().unwrap() = new_state;

                    if let Err(e) = self.renderer.render(&self.state.lock().unwrap()) {
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
