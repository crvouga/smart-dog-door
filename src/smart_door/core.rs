use crate::config::Config;
use crate::device_camera::interface::DeviceCameraEvent;
use crate::device_door::interface::DeviceDoorEvent;
use crate::image_classifier::interface::Classification;
use std::time::{Duration, Instant};

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct DeviceStates {
    pub camera: CameraState,
    pub door: DoorState,
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub enum CameraState {
    #[default]
    Disconnected,
    Connected(Instant),
    Started,
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub enum DoorState {
    #[default]
    Disconnected,
    Connected(Instant),
    Initialized,
    Locked,
    Unlocked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Initial state while devices are connecting
    DevicesInitializing { device_states: DeviceStates },
    /// Capturing frames from cameras for analysis
    AnalyzingFramesCapture { door_state: DoorState },
    /// Classifying captured frames to detect animals
    AnalyzingFramesClassifying {
        door_state: DoorState,
        frames: Vec<Vec<u8>>,
    },
    /// Actively controlling the door (locking/unlocking)
    ControllingDoor {
        action: DoorAction,
        door_state: DoorState,
        start_time: Instant,
    },
    /// Waiting state between operations
    Idle {
        door_state: DoorState,
        message: String,
        message_time: Instant,
        last_classification: Instant,
    },
    /// Period after unlocking to allow dog to pass through
    UnlockedGracePeriod {
        door_state: DoorState,
        countdown_start: Instant,
    },
    /// Period before locking to ensure no dog is present
    LockingGracePeriod {
        door_state: DoorState,
        countdown_start: Instant,
        last_detection: Instant,
    },
    /// Emergency state if something goes wrong
    Error {
        message: String,
        door_state: DoorState,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DoorAction {
    Locking,
    Unlocking,
}

#[derive(Debug)]
pub enum Event {
    Tick(Instant),
    CameraEvent(DeviceCameraEvent),
    CameraStartDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    DoorEvent(DeviceDoorEvent),
    DoorLockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    DoorUnlockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    FramesClassifyDone(Result<Vec<Vec<Classification>>, Box<dyn std::error::Error + Send + Sync>>),
    FramesCaptureDone(Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>>),
}

impl Event {
    pub fn to_display_string(&self) -> String {
        match self {
            Event::FramesCaptureDone(Ok(_)) => "Frames captured successfully".to_string(),
            Event::FramesClassifyDone(Ok(_)) => "Frames classified successfully".to_string(),
            Event::CameraEvent(e) => format!("Camera event: {:?}", e),
            Event::DoorEvent(e) => format!("Door event: {:?}", e),
            e => format!("{:?}", e),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    StartCamera,
    LockDoor,
    UnlockDoor,
    CaptureFrames,
    ClassifyFrames { frames: Vec<Vec<u8>> },
    SubscribeToCameraEvents,
    SubscribeToDoorEvents,
    SubscribeTick,
    Notify { message: String },
}

impl Effect {
    pub fn to_display_string(&self) -> String {
        match self {
            Effect::ClassifyFrames { .. } => "Classify frames".to_string(),
            Effect::Notify { message } => format!("Notify: {}", message),
            e => format!("{:?}", e),
        }
    }
}

pub fn init() -> (State, Vec<Effect>) {
    (
        State::DevicesInitializing {
            device_states: DeviceStates::default(),
        },
        vec![
            Effect::SubscribeToDoorEvents,
            Effect::SubscribeToCameraEvents,
            Effect::SubscribeTick,
        ],
    )
}

pub fn transition(config: &Config, state: State, event: Event) -> (State, Vec<Effect>) {
    match (state.clone(), event) {
        // Device initialization
        (
            State::DevicesInitializing { device_states },
            Event::CameraEvent(DeviceCameraEvent::Connected),
        ) => {
            let new_states = DeviceStates {
                camera: CameraState::Connected(Instant::now()),
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
                door: device_states.door.clone(),
            };

            if device_states.door == DoorState::Initialized {
                (
                    State::AnalyzingFramesCapture {
                        door_state: DoorState::Locked,
                    },
                    vec![Effect::CaptureFrames],
                )
            } else {
                (
                    State::DevicesInitializing {
                        device_states: new_states,
                    },
                    vec![],
                )
            }
        }

        (
            State::DevicesInitializing { device_states },
            Event::DoorEvent(DeviceDoorEvent::Connected),
        ) => {
            let new_states = DeviceStates {
                camera: device_states.camera,
                door: DoorState::Connected(Instant::now()),
            };
            (
                State::DevicesInitializing {
                    device_states: new_states,
                },
                vec![Effect::LockDoor],
            )
        }

        (State::DevicesInitializing { device_states }, Event::DoorLockDone(Ok(()))) => {
            let new_states = DeviceStates {
                camera: device_states.camera.clone(),
                door: DoorState::Initialized,
            };

            if device_states.camera == CameraState::Started {
                (
                    State::AnalyzingFramesCapture {
                        door_state: DoorState::Locked,
                    },
                    vec![Effect::CaptureFrames],
                )
            } else {
                (
                    State::DevicesInitializing {
                        device_states: new_states,
                    },
                    vec![],
                )
            }
        }

        // Main operation flow
        (State::AnalyzingFramesCapture { door_state }, Event::FramesCaptureDone(Ok(frames))) => {
            if frames.is_empty() {
                (
                    State::AnalyzingFramesCapture { door_state },
                    vec![Effect::CaptureFrames],
                )
            } else {
                (
                    State::AnalyzingFramesClassifying {
                        door_state: door_state.clone(),
                        frames: frames.clone(),
                    },
                    vec![Effect::ClassifyFrames { frames }],
                )
            }
        }

        (
            State::AnalyzingFramesClassifying { door_state, .. },
            Event::FramesClassifyDone(Ok(classifications)),
        ) => {
            let dog_detected = classifications.iter().any(|frame_class| {
                frame_class.iter().any(|c| {
                    config.unlock_list.iter().any(|unlock_config| {
                        c.label
                            .to_lowercase()
                            .contains(&unlock_config.label.to_lowercase())
                            && c.confidence >= unlock_config.min_confidence
                    })
                })
            });

            let cat_detected = classifications.iter().any(|frame_class| {
                frame_class.iter().any(|c| {
                    config.lock_list.iter().any(|lock_config| {
                        c.label
                            .to_lowercase()
                            .contains(&lock_config.label.to_lowercase())
                            && c.confidence >= lock_config.min_confidence
                    })
                })
            });

            match (dog_detected, cat_detected, door_state.clone()) {
                (true, false, _) => (
                    State::ControllingDoor {
                        action: DoorAction::Unlocking,
                        door_state: door_state.clone(),
                        start_time: Instant::now(),
                    },
                    vec![
                        Effect::UnlockDoor,
                        Effect::Notify {
                            message: "Dog detected - unlocking door".to_string(),
                        },
                    ],
                ),
                (false, true, DoorState::Unlocked) => (
                    State::LockingGracePeriod {
                        door_state,
                        countdown_start: Instant::now(),
                        last_detection: Instant::now(),
                    },
                    vec![Effect::Notify {
                        message: "Cat detected - preparing to lock".to_string(),
                    }],
                ),
                (false, true, _) => (
                    State::Idle {
                        door_state,
                        message: "Cat detected - door remains locked".to_string(),
                        message_time: Instant::now(),
                        last_classification: Instant::now(),
                    },
                    vec![],
                ),
                _ => (
                    State::Idle {
                        door_state,
                        message: "No relevant animals detected".to_string(),
                        message_time: Instant::now(),
                        last_classification: Instant::now(),
                    },
                    vec![],
                ),
            }
        }

        // Grace period handling
        (
            State::LockingGracePeriod {
                door_state,
                countdown_start,
                last_detection,
            },
            Event::Tick(now),
        ) => {
            let grace_elapsed = now.duration_since(countdown_start);
            let since_last_detection = now.duration_since(last_detection);

            if grace_elapsed >= config.locking_grace_period {
                (
                    State::ControllingDoor {
                        action: DoorAction::Locking,
                        door_state,
                        start_time: Instant::now(),
                    },
                    vec![Effect::LockDoor],
                )
            } else if since_last_detection >= Duration::from_secs(5) {
                // If no new detections for 5 seconds, check again
                (
                    State::AnalyzingFramesCapture { door_state },
                    vec![Effect::CaptureFrames],
                )
            } else {
                (
                    State::LockingGracePeriod {
                        door_state,
                        countdown_start,
                        last_detection,
                    },
                    vec![],
                )
            }
        }

        (
            State::UnlockedGracePeriod {
                door_state,
                countdown_start,
            },
            Event::Tick(now),
        ) => {
            if now.duration_since(countdown_start) >= config.unlock_grace_period {
                (
                    State::AnalyzingFramesCapture { door_state },
                    vec![Effect::CaptureFrames],
                )
            } else {
                (
                    State::UnlockedGracePeriod {
                        door_state,
                        countdown_start,
                    },
                    vec![],
                )
            }
        }

        // Door control results
        (State::ControllingDoor { .. }, Event::DoorLockDone(Ok(_))) => (
            State::Idle {
                door_state: DoorState::Locked,
                message: "Door locked successfully".to_string(),
                message_time: Instant::now(),
                last_classification: Instant::now(),
            },
            vec![],
        ),

        (State::ControllingDoor { .. }, Event::DoorUnlockDone(Ok(_))) => (
            State::UnlockedGracePeriod {
                door_state: DoorState::Unlocked,
                countdown_start: Instant::now(),
            },
            vec![],
        ),

        (
            State::ControllingDoor {
                action: _,
                door_state,
                start_time: _,
            },
            Event::DoorLockDone(Err(e)),
        ) => (
            State::Error {
                message: format!("Failed to lock door: {}", e),
                door_state: door_state.clone(),
            },
            vec![Effect::Notify {
                message: "Door lock failed!".to_string(),
            }],
        ),

        (
            State::ControllingDoor {
                action: _,
                door_state,
                start_time: _,
            },
            Event::DoorUnlockDone(Err(e)),
        ) => (
            State::Error {
                message: format!("Failed to unlock door: {}", e),
                door_state: door_state.clone(),
            },
            vec![Effect::Notify {
                message: "Door unlock failed!".to_string(),
            }],
        ),

        // Error recovery
        (State::Error { .. }, Event::DoorEvent(DeviceDoorEvent::Connected)) => (
            State::DevicesInitializing {
                device_states: DeviceStates {
                    camera: CameraState::Disconnected,
                    door: DoorState::Connected(Instant::now()),
                },
            },
            vec![Effect::LockDoor],
        ),

        (State::Error { door_state, .. }, Event::CameraEvent(DeviceCameraEvent::Connected)) => (
            State::DevicesInitializing {
                device_states: DeviceStates {
                    camera: CameraState::Connected(Instant::now()),
                    door: door_state,
                },
            },
            vec![Effect::StartCamera],
        ),

        // Periodic checks
        (
            State::Idle {
                door_state,
                last_classification,
                ..
            },
            Event::Tick(now),
        ) => {
            if now.duration_since(last_classification) >= config.analyze_rate {
                (
                    State::AnalyzingFramesCapture { door_state },
                    vec![Effect::CaptureFrames],
                )
            } else {
                (state, vec![])
            }
        }

        // Device disconnection
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
            if matches!(state, State::UnlockedGracePeriod { .. }) {
                vec![Effect::LockDoor]
            } else {
                vec![]
            },
        ),

        // Default case - no state change
        (state, _) => (state, vec![]),
    }
}
