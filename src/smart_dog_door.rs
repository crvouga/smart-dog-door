use crate::config::Config;
use crate::device_camera::interface::{DeviceCamera, DeviceCameraEvent};
use crate::device_display::interface::DeviceDisplay;
use crate::device_door::interface::{DeviceDoor, DeviceDoorEvent};
use crate::image_classifier::interface::{Classification, ImageClassifier};
use crate::library::logger::interface::Logger;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

#[derive(Default, Clone)]
pub struct DeviceStates {
    camera: CameraState,
    door: DoorState,
}

#[derive(Default, Clone)]
pub enum CameraState {
    #[default]
    Disconnected,
    Connected,
    Started,
}

#[derive(Default, Clone)]
pub enum DoorState {
    #[default]
    Disconnected,
    Connected,
    Initialized,
    Locked,
    Unlocked,
}

#[derive(Clone)]
pub enum State {
    DevicesInitializing {
        device_states: DeviceStates,
    },
    CapturingFrame {
        door_state: DoorState,
    },
    ClassifyingFrame {
        door_state: DoorState,
    },
    ControllingDoor {
        action: DoorAction,
        door_state: DoorState,
    },
    Idle {
        action: DoorAction,
        door_state: DoorState,
    },
    UnlockGracePeriod {
        door_state: DoorState,
    },
}

#[derive(Clone)]
pub enum DoorAction {
    Locking,
    Unlocking,
}

#[derive(Debug)]
pub enum Event {
    CameraEvent(DeviceCameraEvent),
    CameraStarting,
    CameraStartDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    DoorEvent(DeviceDoorEvent),
    DoorLockStart,
    DoorLockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    DoorUnlockStart,
    DoorUnlockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    FrameClassifyStart,
    FrameClassifyDone(Result<Vec<Classification>, Box<dyn std::error::Error + Send + Sync>>),
    FrameCaptureStart,
    FrameCaptureDone(Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>),
    DelayStart,
    DelayDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    GracePeriodDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
}

// impl std::fmt::Debug for Event {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Event::FrameCaptureDone(result) => f
//                 .debug_tuple("FrameCaptureDone")
//                 .field(&result.as_ref().map(|_| "Vec<u8>"))
//                 .finish(),
//             _ => f.debug_tuple(&format!("{:?}", self)).finish(),
//         }
//     }
// }

#[derive(Clone, Debug)]
pub enum Effect {
    StartCamera,
    // StopCamera,
    LockDogDoor,
    UnlockDogDoor,
    CaptureFrame,
    ClassifyFrame { frame: Vec<u8> },
    Delay,
    GracePeriodDelay,
    SubscribeToCameraEvents,
    SubscribeToDoorEvents,
    None,
}

//
//
//
//
//
//

#[derive(Clone)]
pub struct SmartDogDoor {
    config: Config,
    logger: Arc<dyn Logger + Send + Sync>,
    device_camera: Arc<dyn DeviceCamera + Send + Sync>,
    device_door: Arc<dyn DeviceDoor + Send + Sync>,
    device_display: Arc<Mutex<dyn DeviceDisplay + Send + Sync>>,
    image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
    event_sender: Sender<Event>,
    event_receiver: Arc<Mutex<Receiver<Event>>>,
}

impl SmartDogDoor {
    pub fn new(
        config: Config,
        logger: Arc<dyn Logger + Send + Sync>,
        device_camera: Arc<dyn DeviceCamera + Send + Sync>,
        device_door: Arc<dyn DeviceDoor + Send + Sync>,
        device_display: Arc<Mutex<dyn DeviceDisplay + Send + Sync>>,
        image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
    ) -> Self {
        let (sender, receiver) = channel();
        Self {
            config,
            logger,
            device_camera,
            device_door,
            device_display,
            image_classifier,
            event_sender: sender,
            event_receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    fn init(&self) -> (State, Vec<Effect>) {
        (
            State::DevicesInitializing {
                device_states: DeviceStates::default(),
            },
            vec![
                Effect::SubscribeToCameraEvents,
                Effect::SubscribeToDoorEvents,
            ],
        )
    }

    fn transition(&self, state: State, event: Event) -> (State, Vec<Effect>) {
        match (state.clone(), event) {
            // Device connection handling
            (
                State::DevicesInitializing { device_states },
                Event::CameraEvent(DeviceCameraEvent::Connected),
            ) => {
                let new_states = DeviceStates {
                    camera: CameraState::Connected,
                    door: device_states.door,
                };
                (
                    State::DevicesInitializing {
                        device_states: new_states,
                    },
                    vec![Effect::StartCamera],
                )
            }
            (State::DevicesInitializing { device_states }, Event::CameraStartDone(Ok(()))) => {
                let new_states = DeviceStates {
                    camera: CameraState::Started,
                    door: device_states.door,
                };
                let state = match (new_states.camera.clone(), new_states.door.clone()) {
                    (CameraState::Started, DoorState::Initialized) => State::CapturingFrame {
                        door_state: DoorState::Locked,
                    },
                    _ => State::DevicesInitializing {
                        device_states: new_states,
                    },
                };
                (state, vec![])
            }
            (
                State::DevicesInitializing { device_states },
                Event::DoorEvent(DeviceDoorEvent::Connected),
            ) => {
                let new_states = DeviceStates {
                    camera: device_states.camera,
                    door: DoorState::Connected,
                };
                (
                    State::DevicesInitializing {
                        device_states: new_states,
                    },
                    vec![Effect::LockDogDoor],
                )
            }
            (State::DevicesInitializing { device_states }, Event::DoorLockDone(Ok(()))) => {
                let new_states = DeviceStates {
                    camera: device_states.camera,
                    door: DoorState::Initialized,
                };
                let (state, effect) = match (new_states.camera.clone(), new_states.door.clone()) {
                    (CameraState::Started, DoorState::Initialized) => (
                        State::CapturingFrame {
                            door_state: DoorState::Locked,
                        },
                        Effect::CaptureFrame,
                    ),
                    _ => (
                        State::DevicesInitializing {
                            device_states: new_states,
                        },
                        Effect::None,
                    ),
                };
                (state, vec![effect])
            }

            // Main loop
            (State::CapturingFrame { door_state }, Event::FrameCaptureDone(Ok(frame))) => (
                State::ClassifyingFrame { door_state },
                vec![Effect::ClassifyFrame { frame }],
            ),
            (
                State::ClassifyingFrame { door_state },
                Event::FrameClassifyDone(Ok(classifications)),
            ) => {
                let is_dog_in_frame = self.does_probably_have_dog_in_frame(&classifications);
                let is_cat_in_frame = self.does_probably_have_cat_in_frame(&classifications);

                if is_dog_in_frame && !is_cat_in_frame {
                    (
                        State::ControllingDoor {
                            action: DoorAction::Unlocking,
                            door_state,
                        },
                        vec![Effect::UnlockDogDoor],
                    )
                } else {
                    match door_state {
                        DoorState::Unlocked => (
                            State::ControllingDoor {
                                action: DoorAction::Locking,
                                door_state,
                            },
                            vec![Effect::LockDogDoor],
                        ),
                        _ => (
                            State::Idle {
                                action: DoorAction::Locking,
                                door_state,
                            },
                            vec![Effect::Delay],
                        ),
                    }
                }
            }
            (State::ControllingDoor { action, .. }, Event::DoorLockDone(_)) => (
                State::Idle {
                    action,
                    door_state: DoorState::Locked,
                },
                vec![Effect::Delay],
            ),
            (State::ControllingDoor { action, .. }, Event::DoorUnlockDone(result)) => {
                match result {
                    Ok(_) => (
                        State::UnlockGracePeriod {
                            door_state: DoorState::Unlocked,
                        },
                        vec![Effect::GracePeriodDelay],
                    ),
                    Err(_) => (
                        State::Idle {
                            action,
                            door_state: DoorState::Locked, // Keep as locked if unlock failed
                        },
                        vec![Effect::Delay],
                    ),
                }
            }
            (State::UnlockGracePeriod { door_state }, Event::GracePeriodDone(result)) => {
                match result {
                    Ok(_) | Err(_) => (
                        State::CapturingFrame { door_state },
                        vec![Effect::CaptureFrame],
                    ),
                }
            }
            (State::Idle { door_state, .. }, Event::DelayDone(result)) => match result {
                Ok(_) | Err(_) => (
                    State::CapturingFrame { door_state },
                    vec![Effect::CaptureFrame],
                ),
            },

            (_, Event::CameraEvent(DeviceCameraEvent::Disconnected)) => (
                State::DevicesInitializing {
                    device_states: DeviceStates::default(),
                },
                vec![],
            ),
            (_, Event::DoorEvent(DeviceDoorEvent::Disconnected)) => (
                State::DevicesInitializing {
                    device_states: DeviceStates::default(),
                },
                vec![Effect::LockDogDoor],
            ),

            // Default case
            _ => (state, vec![Effect::None]),
        }
    }

    fn does_probably_have_dog_in_frame(&self, classifications: &[Classification]) -> bool {
        classifications.iter().any(|c| {
            c.label.to_lowercase().contains("dog")
                && c.confidence >= self.config.classification_min_confidence_dog
        })
    }

    fn does_probably_have_cat_in_frame(&self, classifications: &[Classification]) -> bool {
        classifications.iter().any(|c| {
            c.label.to_lowercase().contains("cat")
                && c.confidence >= self.config.classification_min_confidence_cat
        })
    }

    fn run_effect(&self, effect: Effect, event_queue: Sender<Event>) {
        let _ = self.logger.info(&format!("Running effect: {:?}", effect));

        match effect {
            Effect::SubscribeToDoorEvents => {
                let events = self.device_door.events();
                loop {
                    match events.recv() {
                        Ok(event) => {
                            if event_queue.send(Event::DoorEvent(event)).is_err() {
                                continue;
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
            Effect::SubscribeToCameraEvents => {
                let events = self.device_camera.events();
                loop {
                    match events.recv() {
                        Ok(event) => {
                            if event_queue.send(Event::CameraEvent(event)).is_err() {
                                continue;
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
            Effect::StartCamera => {
                let _ = event_queue.send(Event::CameraStarting);
                let started = self.device_camera.start();
                let _ = event_queue.send(Event::CameraStartDone(started));
            }
            // Effect::StopCamera => {
            //     let _ = event_queue.send(Event::CameraStopping);
            //     let _ = self.device_camera.stop();
            // }
            Effect::LockDogDoor => {
                let _ = event_queue.send(Event::DoorLockStart);
                let locked = self.device_door.lock();
                let _ = event_queue.send(Event::DoorLockDone(locked));
            }
            Effect::UnlockDogDoor => {
                let _ = event_queue.send(Event::DoorUnlockStart);
                let unlocked = self.device_door.unlock();
                let _ = event_queue.send(Event::DoorUnlockDone(unlocked));
            }
            Effect::CaptureFrame => {
                let _ = event_queue.send(Event::FrameCaptureStart);
                let frame = self.device_camera.capture_frame();
                let _ = event_queue.send(Event::FrameCaptureDone(frame));
            }
            Effect::ClassifyFrame { frame } => {
                let _ = event_queue.send(Event::FrameClassifyStart);
                let classifications = self.image_classifier.classify(&frame);
                let _ = event_queue.send(Event::FrameClassifyDone(classifications));
            }
            Effect::Delay => {
                let _ = event_queue.send(Event::DelayStart);
                std::thread::sleep(self.config.classification_rate);
                let _ = event_queue.send(Event::DelayDone(Ok(())));
            }
            Effect::GracePeriodDelay => {
                std::thread::sleep(self.config.unlock_grace_period);
                let _ = event_queue.send(Event::GracePeriodDone(Ok(())));
            }
            Effect::None => {}
        }
    }

    fn render(&self, state: &State) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        let mut device_display = self.device_display.lock().unwrap();

        match state {
            State::DevicesInitializing { device_states } => {
                match device_states.camera {
                    CameraState::Disconnected => {
                        device_display.write_line(0, "Camera disconnected")?;
                    }
                    CameraState::Connected => {
                        device_display.write_line(0, "Camera connected")?;
                    }
                    CameraState::Started => {
                        device_display.write_line(0, "Camera started")?;
                    }
                }

                match device_states.door {
                    DoorState::Disconnected => {
                        device_display.write_line(1, "Door disconnected")?;
                    }
                    DoorState::Connected => {
                        device_display.write_line(1, "Door connected")?;
                    }
                    DoorState::Initialized => {
                        device_display.write_line(1, "Door initialized")?;
                    }
                    DoorState::Locked => {
                        device_display.write_line(1, "Door locked")?;
                    }
                    DoorState::Unlocked => {
                        device_display.write_line(1, "Door unlocked")?;
                    }
                }
            }
            State::CapturingFrame { door_state } => {
                device_display.write_line(0, "Capturing frame")?;
                match door_state {
                    DoorState::Locked => device_display.write_line(1, "Door locked")?,
                    DoorState::Unlocked => device_display.write_line(1, "Door unlocked")?,
                    _ => device_display.write_line(1, "Door state unknown")?,
                }
            }
            State::ClassifyingFrame { door_state } => {
                device_display.write_line(0, "Classifying frame")?;
                match door_state {
                    DoorState::Locked => device_display.write_line(1, "Door locked")?,
                    DoorState::Unlocked => device_display.write_line(1, "Door unlocked")?,
                    _ => device_display.write_line(1, "Door state unknown")?,
                }
            }
            State::ControllingDoor { action, door_state } => {
                match action {
                    DoorAction::Locking => device_display.write_line(0, "Locking door")?,
                    DoorAction::Unlocking => device_display.write_line(0, "Unlocking door")?,
                }
                match door_state {
                    DoorState::Locked => device_display.write_line(1, "Door locked")?,
                    DoorState::Unlocked => device_display.write_line(1, "Door unlocked")?,
                    _ => device_display.write_line(1, "Door state unknown")?,
                }
            }
            State::UnlockGracePeriod { door_state } => {
                device_display.write_line(0, "Grace period - keeping unlocked")?;
                match door_state {
                    DoorState::Locked => device_display.write_line(1, "Door locked")?,
                    DoorState::Unlocked => device_display.write_line(1, "Door unlocked")?,
                    _ => device_display.write_line(1, "Door state unknown")?,
                }
            }
            State::Idle { action, door_state } => {
                match action {
                    DoorAction::Locking => device_display.write_line(0, "Idle - Will lock")?,
                    DoorAction::Unlocking => device_display.write_line(0, "Idle - Will unlock")?,
                }
                match door_state {
                    DoorState::Locked => device_display.write_line(1, "Door locked")?,
                    DoorState::Unlocked => device_display.write_line(1, "Door unlocked")?,
                    _ => device_display.write_line(1, "Door state unknown")?,
                }
            }
        }

        Ok(())
    }

    fn spawn_effects(&self, effects: Vec<Effect>) {
        for effect in effects {
            let effect_sender = self.event_sender.clone();
            let effect_clone = effect.clone();
            let self_clone = self.clone();
            std::thread::spawn(move || self_clone.run_effect(effect_clone, effect_sender));
        }
    }

    pub fn run(&self) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        let (mut current_state, effects) = self.init();

        self.spawn_effects(effects);

        loop {
            match self.event_receiver.lock().unwrap().recv() {
                Ok(event) => {
                    let _ = self.logger.info(&format!("Processing event: {:?}", event));

                    let (new_state, new_effects) = self.transition(current_state.clone(), event);
                    current_state = new_state;
                    self.render(&current_state)?;

                    self.spawn_effects(new_effects);
                }
                Err(e) => {
                    return Err(Arc::new(e));
                }
            }
        }
    }
}
