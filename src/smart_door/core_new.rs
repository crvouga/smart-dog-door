use crate::config::Config;
use crate::device_camera::interface::{DeviceCameraEvent, Frame};
use crate::device_door::interface::DeviceDoorEvent;
use crate::image_classifier::interface::Classification;
use std::time::Instant;

//
//
//
// Model
//
//
//

#[derive(Clone, Debug, PartialEq)]
pub enum Model {
    Connecting(ModelConnecting),
    Ready(ModelReady),
}

impl Default for Model {
    fn default() -> Self {
        Model::Connecting(ModelConnecting::default())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ModelConnecting {
    #[default]
    Connecting,
    OnlyCameraConnecting,
    OnlyDoorConnecting,
}

impl ModelConnecting {
    pub fn init() -> Self {
        ModelConnecting::Connecting
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ModelReady {
    camera: ModelCamera,
    door: ModelDoor,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ModelCamera {
    status: ModelCameraStatus,
    classifications: Vec<Vec<Classification>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelCameraStatus {
    Idle { start_time: Instant },
    Capturing,
    Classifying(Vec<Frame>),
}

impl Default for ModelCameraStatus {
    fn default() -> Self {
        ModelCameraStatus::Idle {
            start_time: Instant::now(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelDoor {
    LockingGracePeriod {
        start_time: Instant,
        countdown_start: Instant,
    },
    Locking,
    Locked,
    UnlockingGracePeriod {
        start_time: Instant,
        countdown_start: Instant,
    },
    Unlocking,
    Unlocked,
}

impl Default for ModelDoor {
    fn default() -> Self {
        ModelDoor::Unlocked
    }
}

#[derive(Debug)]
pub enum Event {
    Tick(Instant),
    CameraEvent(DeviceCameraEvent),
    CameraStartDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    DoorEvent(DeviceDoorEvent),
    DoorLockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    DoorUnlockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    FramesCaptureDone(Result<Vec<Frame>, Box<dyn std::error::Error + Send + Sync>>),
    FramesClassifyDone(Result<Vec<Vec<Classification>>, Box<dyn std::error::Error + Send + Sync>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    StartCamera,
    LockDoor,
    UnlockDoor,
    CaptureFrames,
    ClassifyFrames { frames: Vec<Frame> },
    SubscribeToCameraEvents,
    SubscribeToDoorEvents,
    SubscribeTick,
}

//
//
//
// Init
//
//
//

pub fn init() -> (Model, Vec<Effect>) {
    (
        Model::default(),
        vec![
            Effect::SubscribeToDoorEvents,
            Effect::SubscribeToCameraEvents,
            Effect::SubscribeTick,
        ],
    )
}

//
//
//
// Transition
//
//
//

pub fn transition(config: &Config, state: Model, event: Event) -> (Model, Vec<Effect>) {
    // Handle other state transitions
    match (state, event) {
        (_, Event::CameraEvent(DeviceCameraEvent::Disconnected)) => {
            (Model::Connecting(ModelConnecting::Connecting), vec![])
        }

        (_, Event::DoorEvent(DeviceDoorEvent::Disconnected)) => {
            (Model::Connecting(ModelConnecting::Connecting), vec![])
        }

        (Model::Connecting(child), event) => transition_connecting(config, child, event),

        (Model::Ready(child), event) => transition_ready(config, child, event),
    }
}

fn transition_connecting(
    _config: &Config,
    state: ModelConnecting,
    event: Event,
) -> (Model, Vec<Effect>) {
    match (state, event) {
        // Initial connection of both devices
        (ModelConnecting::Connecting, Event::CameraEvent(DeviceCameraEvent::Connected)) => (
            Model::Connecting(ModelConnecting::OnlyDoorConnecting),
            vec![Effect::StartCamera],
        ),

        (ModelConnecting::Connecting, Event::DoorEvent(DeviceDoorEvent::Connected)) => (
            Model::Connecting(ModelConnecting::OnlyCameraConnecting),
            vec![],
        ),

        // Camera connection handling
        (ModelConnecting::OnlyDoorConnecting, Event::CameraStartDone(Ok(()))) => (
            Model::Ready(ModelReady {
                camera: ModelCamera {
                    status: ModelCameraStatus::Capturing,
                    classifications: vec![],
                },
                door: ModelDoor::Unlocked,
            }),
            vec![],
        ),

        // Door connection handling
        (ModelConnecting::OnlyCameraConnecting, Event::DoorEvent(DeviceDoorEvent::Connected)) => (
            Model::Ready(ModelReady {
                camera: ModelCamera {
                    status: ModelCameraStatus::Capturing,
                    classifications: vec![],
                },
                door: ModelDoor::Unlocked,
            }),
            vec![],
        ),

        // Error cases
        (ModelConnecting::OnlyDoorConnecting, Event::CameraStartDone(Err(_))) => (
            Model::Connecting(ModelConnecting::OnlyDoorConnecting),
            vec![Effect::StartCamera], // Retry
        ),

        // Disconnection handling
        (_, Event::CameraEvent(DeviceCameraEvent::Disconnected)) => {
            (Model::Connecting(ModelConnecting::Connecting), vec![])
        }

        (_, Event::DoorEvent(DeviceDoorEvent::Disconnected)) => {
            (Model::Connecting(ModelConnecting::Connecting), vec![])
        }

        // Default case - no state change
        (state, _) => (Model::Connecting(state), vec![]),
    }
}

fn transition_ready(config: &Config, state: ModelReady, event: Event) -> (Model, Vec<Effect>) {
    let door_result = transition_door(state.door, &event);
    let camera_result = transition_camera(config, state.camera, event);

    let combined = ModelReady {
        camera: camera_result.0,
        door: door_result.0,
    };

    let mut combined_effects = door_result.1;
    combined_effects.extend(camera_result.1);

    (Model::Ready(combined), combined_effects)
}

fn transition_door(current: ModelDoor, event: &Event) -> (ModelDoor, Vec<Effect>) {
    match (current.clone(), event) {
        (ModelDoor::Locking { .. }, Event::DoorLockDone(Ok(_))) => (ModelDoor::Locked, vec![]),

        (ModelDoor::Unlocking { .. }, Event::DoorUnlockDone(Ok(_))) => {
            (ModelDoor::Unlocked, vec![])
        }

        (ModelDoor::Locking { .. }, Event::DoorLockDone(Err(_))) => (
            ModelDoor::Locked,
            vec![Effect::LockDoor], // Retry
        ),

        (ModelDoor::Unlocking { .. }, Event::DoorUnlockDone(Err(_))) => (
            ModelDoor::Locked,
            vec![Effect::UnlockDoor], // Retry
        ),

        // No change for other events
        _ => (current, vec![]),
    }
}

fn transition_camera(
    config: &Config,
    current: ModelCamera,
    event: Event,
) -> (ModelCamera, Vec<Effect>) {
    match (current, event) {
        // Frame capture transitions
        (
            ModelCamera {
                status: ModelCameraStatus::Capturing,
                ..
            },
            Event::FramesCaptureDone(Ok(frames)),
        ) => {
            if frames.is_empty() {
                (
                    ModelCamera {
                        status: ModelCameraStatus::Capturing,
                        classifications: vec![],
                    },
                    vec![Effect::CaptureFrames],
                )
            } else {
                (
                    ModelCamera {
                        status: ModelCameraStatus::Classifying(frames.clone()),
                        classifications: vec![],
                    },
                    vec![Effect::ClassifyFrames {
                        frames: frames.clone(),
                    }],
                )
            }
        }

        // Frame classification transitions
        (
            ModelCamera {
                status: ModelCameraStatus::Classifying(..),
                ..
            },
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

            // Note: Door effects are handled separately now
            if dog_detected && !cat_detected {
                (
                    ModelCamera {
                        status: ModelCameraStatus::Capturing,
                        classifications: vec![],
                    },
                    vec![Effect::UnlockDoor],
                )
            } else if cat_detected {
                (
                    ModelCamera {
                        status: ModelCameraStatus::Capturing,
                        classifications: vec![],
                    },
                    vec![Effect::LockDoor],
                )
            } else {
                (
                    ModelCamera {
                        status: ModelCameraStatus::Capturing,
                        classifications: vec![],
                    },
                    vec![Effect::CaptureFrames],
                )
            }
        }

        // Error cases
        (
            ModelCamera {
                status: ModelCameraStatus::Capturing,
                classifications,
            },
            Event::FramesCaptureDone(Err(_)),
        ) => (
            ModelCamera {
                status: ModelCameraStatus::Capturing,
                classifications,
            },
            vec![Effect::CaptureFrames], // Retry
        ),

        (
            ModelCamera {
                status: ModelCameraStatus::Classifying(..),
                classifications,
            },
            Event::FramesClassifyDone(Err(_)),
        ) => (
            ModelCamera {
                status: ModelCameraStatus::Capturing,
                classifications,
            },
            vec![Effect::CaptureFrames], // Retry with new frames
        ),
        // Periodic checks
        (
            ModelCamera {
                status: ModelCameraStatus::Capturing,
                classifications,
            },
            Event::Tick(_),
        ) => (
            ModelCamera {
                status: ModelCameraStatus::Capturing,
                classifications,
            },
            vec![Effect::CaptureFrames],
        ),

        // No change for other events
        (current, _) => (current, vec![]),
    }
}
