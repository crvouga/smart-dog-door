use crate::config::Config;
use crate::device_camera::interface::DeviceCamera;
use crate::device_display::interface::DeviceDisplay;
use crate::device_dog_door::interface::DeviceDogDoor;
use crate::image_classifier::interface::{Classification, ImageClassifier};
use crate::library::logger::interface::Logger;
use crate::library::state_machine::StateMachine;
use std::sync::mpsc::Sender;
use std::sync::Arc;

#[derive(Default, Clone)]
pub struct DeviceStates {
    camera: CameraState,
    dog_door: DogDoorState,
}

#[derive(Default, Clone)]
pub enum CameraState {
    #[default]
    Disconnected,
    Connected,
    Started,
}

#[derive(Default, Clone)]
pub enum DogDoorState {
    #[default]
    Disconnected,
    Connected,
    Initialized,
}

#[derive(Clone, Copy)]
pub enum DoorState {
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

pub enum Event {
    CameraDisconnected,
    CameraConnected,
    CameraStarting,
    CameraStartDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    CameraStopping,
    CameraStopDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    DogDoorConnected,
    DogDoorDisconnected,
    DogDoorLocking,
    DogDoorLockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    DogDoorUnlocking,
    DogDoorUnlockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    FrameClassifying,
    FrameClassifyDone {
        classifications: Vec<Classification>,
    },
    FrameCapturing,
    FrameCaptured {
        frame: Vec<u8>,
    },
    Sleeping,
    SleepCompleted(Result<(), Box<dyn std::error::Error + Send + Sync>>),
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
    device_dog_door: Arc<dyn DeviceDogDoor + Send + Sync>,
    // device_display: Arc<dyn DeviceDisplay + Send + Sync>,
    image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
}

impl SmartDogDoor {
    pub fn new(
        config: Config,
        _logger: Arc<dyn Logger + Send + Sync>,
        device_camera: Arc<dyn DeviceCamera + Send + Sync>,
        device_dog_door: Arc<dyn DeviceDogDoor + Send + Sync>,
        _device_display: Arc<dyn DeviceDisplay + Send + Sync>,
        image_classifier: Arc<dyn ImageClassifier + Send + Sync>,
    ) -> Self {
        Self {
            config,
            // logger,
            device_camera,
            device_dog_door,
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
                    dog_door: device_states.dog_door,
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
                    dog_door: device_states.dog_door,
                };
                let state = match (new_states.camera.clone(), new_states.dog_door.clone()) {
                    (CameraState::Started, DogDoorState::Initialized) => State::CapturingFrame {
                        door_state: DoorState::Locked,
                    },
                    _ => State::DevicesInitializing {
                        device_states: new_states,
                    },
                };
                (state, vec![])
            }
            (State::DevicesInitializing { device_states }, Event::DogDoorConnected) => {
                let new_states = DeviceStates {
                    camera: device_states.camera,
                    dog_door: DogDoorState::Connected,
                };
                (
                    State::DevicesInitializing {
                        device_states: new_states,
                    },
                    vec![Effect::LockDogDoor],
                )
            }
            (State::DevicesInitializing { device_states }, Event::DogDoorLockDone(Ok(()))) => {
                let new_states = DeviceStates {
                    camera: device_states.camera,
                    dog_door: DogDoorState::Initialized,
                };
                let (state, effect) = match (new_states.camera.clone(), new_states.dog_door.clone())
                {
                    (CameraState::Started, DogDoorState::Initialized) => (
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
            (State::CapturingFrame { door_state }, Event::FrameCaptured { frame }) => (
                State::ClassifyingFrame { door_state },
                vec![Effect::ClassifyFrame { frame }],
            ),
            (
                State::ClassifyingFrame { door_state },
                Event::FrameClassifyDone { classifications },
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
            (State::ControllingDoor { action, .. }, Event::DogDoorLockDone(_)) => (
                State::Sleeping {
                    action,
                    door_state: DoorState::Locked,
                },
                vec![Effect::Sleep],
            ),
            (State::ControllingDoor { action, .. }, Event::DogDoorUnlockDone(_)) => (
                State::Sleeping {
                    action,
                    door_state: DoorState::Unlocked,
                },
                vec![Effect::Sleep],
            ),
            (State::Sleeping { door_state, .. }, Event::SleepCompleted(_)) => (
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
            (_, Event::DogDoorDisconnected) => (
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
                let _ = event_queue.send(Event::DogDoorConnected);
                let _ = event_queue.send(Event::DogDoorDisconnected);
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
                let _ = event_queue.send(Event::DogDoorLocking);
                let locked = self.device_dog_door.lock();
                let _ = event_queue.send(Event::DogDoorLockDone(locked));
            }
            Effect::UnlockDogDoor => {
                let _ = event_queue.send(Event::DogDoorUnlocking);
                let unlocked = self.device_dog_door.unlock();
                let _ = event_queue.send(Event::DogDoorUnlockDone(unlocked));
            }
            Effect::CaptureFrame => {
                let _ = event_queue.send(Event::FrameCapturing);
                if let Ok(frame) = self.device_camera.capture_frame() {
                    let _ = event_queue.send(Event::FrameCaptured { frame });
                }
            }
            Effect::ClassifyFrame { frame } => {
                let _ = event_queue.send(Event::FrameClassifying);
                if let Ok(classifications) = self.image_classifier.classify(&frame) {
                    let _ = event_queue.send(Event::FrameClassifyDone { classifications });
                }
            }
            Effect::Sleep => {
                let _ = event_queue.send(Event::Sleeping);
                std::thread::sleep(std::time::Duration::from_secs(1));
                let _ = event_queue.send(Event::SleepCompleted(Ok(())));
            }
            Effect::None => {}
        }
    }

    fn render(&self, state: &State) {
        match state {
            State::DevicesInitializing { device_states } => match device_states.camera {
                CameraState::Disconnected => println!("Display: Waiting for camera to connect..."),
                CameraState::Connected => println!("Display: Camera connected"),
                CameraState::Started => match device_states.dog_door {
                    DogDoorState::Disconnected => {
                        println!("Display: Waiting for dog door to connect...")
                    }
                    DogDoorState::Connected => println!("Display: Initializing dog door..."),
                    DogDoorState::Initialized => {}
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
        let init = self.init();
        let clone1 = self.clone();
        let clone2 = self.clone();
        let clone3 = self.clone();

        let state_machine = StateMachine::new(
            init,
            move |state, event| clone1.transition(state, event),
            move |state| clone2.render(state),
            move |effect, sender| clone3.run_effect(effect, sender),
        );

        state_machine.run()?;

        Ok(())
    }
}
