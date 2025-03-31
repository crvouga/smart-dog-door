use crate::config::Config;
use crate::device_camera::interface::DeviceCameraEvent;
use crate::device_door::interface::DeviceDoorEvent;
use crate::image_classifier::interface::Classification;
use std::time::Instant;

#[derive(Default, Clone)]
pub struct DeviceStates {
    pub camera: CameraState,
    pub door: DoorState,
}

#[derive(Default, Clone)]
pub enum CameraState {
    #[default]
    Disconnected,
    Connected(Instant),
    Started,
}

#[derive(Default, Clone)]
pub enum DoorState {
    #[default]
    Disconnected,
    Connected(Instant),
    Initialized,
    Locked,
    Unlocked,
}

#[derive(Clone)]
pub enum State {
    DevicesInitializing {
        device_states: DeviceStates,
    },
    AnalyzingFramesCapture {
        door_state: DoorState,
    },
    AnalyzingFramesClassifying {
        door_state: DoorState,
    },
    ControllingDoor {
        action: DoorAction,
        #[allow(dead_code)]
        door_state: DoorState,
        start_time: Instant,
    },
    Idle {
        #[allow(dead_code)]
        action: DoorAction,
        door_state: DoorState,
        message: String,
        message_time: Instant,
        last_classification: Instant,
    },
    UnlockedGracePeriod {
        door_state: DoorState,
        countdown_start: Instant,
    },
    LockingGracePeriod {
        door_state: DoorState,
        countdown_start: Instant,
    },
}

#[derive(Clone)]
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
            Event::FramesCaptureDone(Ok(_frames)) => {
                format!("{:?}", Event::FramesCaptureDone(Ok(vec![])))
            }
            event => format!("{:?}", event),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Effect {
    StartCamera,
    LockDoor,
    UnlockDoor,
    CaptureFrames,
    ClassifyFrames { frames: Vec<Vec<u8>> },
    SubscribeToCameraEvents,
    SubscribeToDoorEvents,
    SubscribeTick,
}

impl Effect {
    pub fn to_display_string(&self) -> String {
        match self {
            Effect::ClassifyFrames { .. } => {
                format!("{:?}", Effect::ClassifyFrames { frames: vec![] })
            }
            effect => format!("{:?}", effect),
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
        // Device connection handling
        (
            State::DevicesInitializing { mut device_states },
            Event::CameraEvent(DeviceCameraEvent::Connected),
        ) => {
            device_states.camera = CameraState::Connected(Instant::now());
            (
                State::DevicesInitializing { device_states },
                vec![Effect::StartCamera],
            )
        }
        (State::DevicesInitializing { mut device_states }, Event::CameraStartDone(Ok(()))) => {
            device_states.camera = CameraState::Started;

            if matches!(device_states.door, DoorState::Initialized) {
                (
                    State::AnalyzingFramesCapture {
                        door_state: DoorState::Locked,
                    },
                    vec![Effect::CaptureFrames],
                )
            } else {
                (State::DevicesInitializing { device_states }, vec![])
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
        (State::DevicesInitializing { mut device_states }, Event::DoorLockDone(Ok(()))) => {
            device_states.door = DoorState::Initialized;

            if matches!(device_states.camera, CameraState::Started) {
                (
                    State::AnalyzingFramesCapture {
                        door_state: DoorState::Locked,
                    },
                    vec![Effect::CaptureFrames],
                )
            } else {
                (State::DevicesInitializing { device_states }, vec![])
            }
        }

        // Main loop
        (State::AnalyzingFramesCapture { door_state }, Event::FramesCaptureDone(Ok(frames))) => {
            if !frames.is_empty() {
                (
                    State::AnalyzingFramesClassifying { door_state },
                    vec![Effect::ClassifyFrames { frames }],
                )
            } else {
                (
                    State::AnalyzingFramesCapture { door_state },
                    vec![Effect::CaptureFrames],
                )
            }
        }
        (
            State::AnalyzingFramesClassifying { door_state },
            Event::FramesClassifyDone(Ok(classifications)),
        ) => {
            let should_unlock = classifications.iter().any(|frame_class| {
                frame_class.iter().any(|c| {
                    config.unlock_list.iter().any(|unlock_config| {
                        c.label
                            .to_lowercase()
                            .contains(&unlock_config.label.to_lowercase())
                            && c.confidence >= unlock_config.min_confidence
                    })
                })
            });

            let should_lock = classifications.iter().any(|frame_class| {
                frame_class.iter().any(|c| {
                    config.lock_list.iter().any(|lock_config| {
                        c.label
                            .to_lowercase()
                            .contains(&lock_config.label.to_lowercase())
                            && c.confidence >= lock_config.min_confidence
                    })
                })
            });

            if should_unlock && !should_lock {
                (
                    State::ControllingDoor {
                        action: DoorAction::Unlocking,
                        door_state,
                        start_time: Instant::now(),
                    },
                    vec![Effect::UnlockDoor],
                )
            } else {
                let message = if should_lock {
                    "Lock entity detected".to_string()
                } else {
                    "Unlock entity not detected".to_string()
                };

                match door_state {
                    DoorState::Unlocked => (
                        State::LockingGracePeriod {
                            door_state,
                            countdown_start: Instant::now(),
                        },
                        vec![],
                    ),
                    _ => (
                        State::Idle {
                            action: DoorAction::Locking,
                            door_state,
                            message,
                            message_time: Instant::now(),
                            last_classification: Instant::now(),
                        },
                        vec![],
                    ),
                }
            }
        }
        (
            State::LockingGracePeriod {
                door_state,
                countdown_start,
            },
            Event::Tick(now),
        ) => {
            let elapsed = now.duration_since(countdown_start);
            if elapsed >= config.locking_grace_period {
                (
                    State::ControllingDoor {
                        action: DoorAction::Locking,
                        door_state,
                        start_time: Instant::now(),
                    },
                    vec![Effect::LockDoor],
                )
            } else {
                (
                    State::LockingGracePeriod {
                        door_state,
                        countdown_start,
                    },
                    vec![],
                )
            }
        }
        (State::ControllingDoor { action, .. }, Event::DoorLockDone(_)) => (
            State::Idle {
                action,
                door_state: DoorState::Locked,
                message: "Door locked".to_string(),
                message_time: Instant::now(),
                last_classification: Instant::now(),
            },
            vec![],
        ),
        (State::ControllingDoor { action, .. }, Event::DoorUnlockDone(result)) => match result {
            Ok(_) => (
                State::UnlockedGracePeriod {
                    door_state: DoorState::Unlocked,
                    countdown_start: Instant::now(),
                },
                vec![],
            ),
            Err(_) => (
                State::Idle {
                    action,
                    door_state: DoorState::Locked,
                    message: "Door locked".to_string(),
                    message_time: Instant::now(),
                    last_classification: Instant::now(),
                },
                vec![],
            ),
        },
        (
            State::UnlockedGracePeriod {
                door_state,
                countdown_start,
            },
            Event::Tick(now),
        ) => {
            let elapsed = now.duration_since(countdown_start);
            if elapsed >= config.unlock_grace_period {
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

        (_, Event::CameraEvent(DeviceCameraEvent::Disconnected)) => (
            State::DevicesInitializing {
                device_states: DeviceStates::default(),
            },
            vec![],
        ),
        (_, Event::DoorEvent(DeviceDoorEvent::Disconnected)) => {
            let mut effects = vec![];
            if matches!(state, State::UnlockedGracePeriod { .. }) {
                effects.push(Effect::LockDoor);
            }
            (
                State::DevicesInitializing {
                    device_states: DeviceStates::default(),
                },
                effects,
            )
        }

        // Default case
        _ => (state, vec![]),
    }
}
