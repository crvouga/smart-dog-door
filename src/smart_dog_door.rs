use crate::config::Config;
use crate::device_camera::interface::DeviceCamera;
use crate::device_display::interface::DeviceDisplay;
use crate::device_door::interface::DeviceDoor;
use crate::image_classifier::interface::{Classification, ImageClassifier};
use crate::library::logger::interface::Logger;
use std::sync::mpsc::Sender;
use std::sync::Arc;

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
    Sleeping {
        action: DoorAction,
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
    CameraDisconnected,
    CameraConnected,
    CameraStarting,
    CameraStartDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    CameraStopping,
    CameraStopDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    DoorDeviceConnected,
    DoorDeviceDisconnected,
    DoorLockStart,
    DoorLockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    DoorUnlockStart,
    DoorUnlockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    FrameClassifyStart,
    FrameClassifyDone(Result<Vec<Classification>, Box<dyn std::error::Error + Send + Sync>>),
    FrameCaptureStart,
    FrameCaptureDone(Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>),
    SleepStart,
    SleepDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
}

#[derive(Clone)]
pub enum Effect {
    StartCamera,
    StopCamera,
    LockDogDoor,
    UnlockDogDoor,
    CaptureFrame,
    ClassifyFrame { frame: Vec<u8> },
    Sleep,
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
    // logger: Arc<dyn Logger + Send + Sync>,
    device_camera: Arc<dyn DeviceCamera + Send + Sync>,
    device_door: Arc<dyn DeviceDoor + Send + Sync>,
    // device_display: Arc<dyn DeviceDisplay + Send + Sync>,
    image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
}

impl SmartDogDoor {
    pub fn new(
        config: Config,
        _logger: Arc<dyn Logger + Send + Sync>,
        device_camera: Arc<dyn DeviceCamera + Send + Sync>,
        device_door: Arc<dyn DeviceDoor + Send + Sync>,
        _device_display: Arc<dyn DeviceDisplay + Send + Sync>,
        image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
    ) -> Self {
        Self {
            config,
            // logger,
            device_camera,
            device_door,
            // device_display,
            image_classifier,
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
            (State::DevicesInitializing { device_states }, Event::CameraConnected) => {
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
            (State::DevicesInitializing { device_states }, Event::DoorDeviceConnected) => {
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
                    (
                        State::ControllingDoor {
                            action: DoorAction::Locking,
                            door_state,
                        },
                        vec![Effect::LockDogDoor],
                    )
                }
            }
            (State::ControllingDoor { action, .. }, Event::DoorLockDone(_)) => (
                State::Sleeping {
                    action,
                    door_state: DoorState::Locked,
                },
                vec![Effect::Sleep],
            ),
            (State::ControllingDoor { action, .. }, Event::DoorUnlockDone(_)) => (
                State::Sleeping {
                    action,
                    door_state: DoorState::Unlocked,
                },
                vec![Effect::Sleep],
            ),
            (State::Sleeping { door_state, .. }, Event::SleepDone(_)) => (
                State::CapturingFrame { door_state },
                vec![Effect::CaptureFrame], // Back to start of main loop
            ),

            // Device disconnection handling
            (_, Event::CameraDisconnected) => (
                State::DevicesInitializing {
                    device_states: DeviceStates::default(),
                },
                vec![], // Wait for camera to reconnect
            ),
            (_, Event::DoorDeviceDisconnected) => (
                State::DevicesInitializing {
                    device_states: DeviceStates::default(),
                },
                vec![Effect::LockDogDoor], // Try to lock door when reconnected
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
        match effect {
            Effect::SubscribeToDoorEvents => {
                let _ = event_queue.send(Event::DoorDeviceConnected);
                let _ = event_queue.send(Event::DoorDeviceDisconnected);
            }
            Effect::SubscribeToCameraEvents => {
                let _ = event_queue.send(Event::CameraConnected);
                let _ = event_queue.send(Event::CameraDisconnected);
            }
            Effect::StartCamera => {
                let _ = event_queue.send(Event::CameraStarting);
                let started = self.device_camera.start();
                let _ = event_queue.send(Event::CameraStartDone(started));
            }
            Effect::StopCamera => {
                let _ = event_queue.send(Event::CameraStopping);
                let _ = self.device_camera.stop();
            }
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
            Effect::Sleep => {
                let _ = event_queue.send(Event::SleepStart);
                std::thread::sleep(self.config.classification_rate);
                let _ = event_queue.send(Event::SleepDone(Ok(())));
            }
            Effect::None => {}
        }
    }

    fn render(&self, state: &State) {
        match state {
            State::DevicesInitializing { device_states } => match device_states.camera {
                CameraState::Disconnected => println!("Display: Waiting for camera to connect..."),
                CameraState::Connected => println!("Display: Camera connected"),
                CameraState::Started => match device_states.door {
                    DoorState::Disconnected => {
                        println!("Display: Waiting for dog door to connect...")
                    }
                    DoorState::Connected => println!("Display: Initializing dog door..."),
                    DoorState::Initialized => {}
                    _ => {}
                },
            },
            State::CapturingFrame { .. } => println!("Display: Capturing frame..."),
            State::ClassifyingFrame { .. } => println!("Display: Classifying frame..."),
            State::ControllingDoor { action, .. } => match action {
                DoorAction::Locking => println!("Display: Locking door..."),
                DoorAction::Unlocking => println!("Display: Unlocking door..."),
            },
            State::Sleeping { action, door_state } => match (action, door_state) {
                (DoorAction::Locking, DoorState::Locked) => {
                    println!("Display: Sleeping... Door is locked")
                }
                (DoorAction::Unlocking, DoorState::Unlocked) => {
                    println!("Display: Sleeping... Door is unlocked")
                }
                _ => println!("Display: Sleeping... Door state mismatch"),
            },
        }
    }
    pub fn run(&self) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        let (event_sender, event_receiver) = std::sync::mpsc::channel();
        let (mut current_state, effects) = self.init();

        // Process initial effects
        for effect in effects {
            let effect_sender = event_sender.clone();
            let effect_clone = effect.clone();
            let self_clone = self.clone();
            std::thread::spawn(move || {
                self_clone.run_effect(effect_clone, effect_sender);
            });
        }

        // Main loop
        loop {
            match event_receiver.recv() {
                Ok(event) => {
                    println!("Processing event: {:?}", event);
                    let (new_state, new_effects) = self.transition(current_state.clone(), event);
                    current_state = new_state;
                    self.render(&current_state);

                    // Process new effects
                    for effect in new_effects {
                        let effect_sender = event_sender.clone();
                        let effect_clone = effect.clone();
                        let self_clone = self.clone();

                        std::thread::spawn(move || {
                            self_clone.run_effect(effect_clone, effect_sender);
                        });
                    }
                }
                Err(e) => {
                    return Err(Arc::new(e));
                }
            }
        }
    }
}
