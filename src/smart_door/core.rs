use crate::config::Config;
use crate::device_camera::interface::{DeviceCameraEvent, Frame};
use crate::device_door::interface::DeviceDoorEvent;
use crate::image_classifier::interface::Classification;
use std::time::Instant;

//
//
//

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
pub struct ModelConnecting {
    pub camera: ModelDeviceConnection,
    pub door: ModelDeviceConnection,
}

impl ModelConnecting {
    pub fn init() -> Self {
        ModelConnecting {
            camera: ModelDeviceConnection::default(),
            door: ModelDeviceConnection::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ModelDeviceConnection {
    #[default]
    Connecting,
    Connected,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ModelReady {
    pub camera: ModelCamera,
    pub door: ModelDoor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelCamera {
    Idle,
    Capturing,
    Classifying,
}

impl Default for ModelCamera {
    fn default() -> Self {
        ModelCamera::Idle
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
pub enum Msg {
    Tick(Instant),
    CameraEvent(DeviceCameraEvent),
    DoorEvent(DeviceDoorEvent),
    DoorLockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    DoorUnlockDone(Result<(), Box<dyn std::error::Error + Send + Sync>>),
    FramesCaptureDone(Result<Vec<Frame>, Box<dyn std::error::Error + Send + Sync>>),
    FramesClassifyDone(Result<Vec<Vec<Classification>>, Box<dyn std::error::Error + Send + Sync>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    LockDoor,
    UnlockDoor,
    CaptureFrames,
    ClassifyFrames { frames: Vec<Frame> },
    SubscribeCamera,
    SubscribeDoor,
    SubscribeTick,
}

//
//
//

//
//
//

pub fn init() -> (Model, Vec<Effect>) {
    (
        Model::default(),
        vec![
            Effect::SubscribeDoor,
            Effect::SubscribeCamera,
            Effect::SubscribeTick,
        ],
    )
}

//
//
//

//
//
//

pub fn transition(config: &Config, model: Model, msg: Msg) -> (Model, Vec<Effect>) {
    match (model, msg) {
        (Model::Connecting(child), event) => transition_connecting(config, child, event),

        (Model::Ready(child), msg) => transition_ready(config, child, msg),
    }
}

fn transition_connecting(
    _config: &Config,
    state: ModelConnecting,
    event: Msg,
) -> (Model, Vec<Effect>) {
    let model_new = ModelConnecting {
        camera: transition_connecting_camera(state.camera, &event),
        door: transition_connecting_door(state.door, &event),
    };

    if is_all_devices_connected(&model_new) {
        (Model::Ready(ModelReady::default()), vec![])
    } else {
        (Model::Connecting(model_new), vec![])
    }
}

fn is_all_devices_connected(model: &ModelConnecting) -> bool {
    model.camera == ModelDeviceConnection::Connected
        && model.door == ModelDeviceConnection::Connected
}

fn transition_connecting_camera(
    model: ModelDeviceConnection,
    event: &Msg,
) -> ModelDeviceConnection {
    match (model.clone(), event) {
        (ModelDeviceConnection::Connecting, Msg::CameraEvent(DeviceCameraEvent::Connected)) => {
            ModelDeviceConnection::Connected
        }
        (ModelDeviceConnection::Connecting, Msg::CameraEvent(DeviceCameraEvent::Disconnected)) => {
            ModelDeviceConnection::Connecting
        }
        (ModelDeviceConnection::Connected, Msg::CameraEvent(DeviceCameraEvent::Connected)) => {
            ModelDeviceConnection::Connected
        }
        _ => model,
    }
}

fn transition_connecting_door(model: ModelDeviceConnection, event: &Msg) -> ModelDeviceConnection {
    match (model.clone(), event) {
        (ModelDeviceConnection::Connecting, Msg::DoorEvent(DeviceDoorEvent::Connected)) => {
            ModelDeviceConnection::Connected
        }
        (ModelDeviceConnection::Connecting, Msg::DoorEvent(DeviceDoorEvent::Disconnected)) => {
            ModelDeviceConnection::Connecting
        }
        (ModelDeviceConnection::Connected, Msg::DoorEvent(DeviceDoorEvent::Connected)) => {
            ModelDeviceConnection::Connected
        }
        _ => model,
    }
}

fn transition_ready(config: &Config, model: ModelReady, msg: Msg) -> (Model, Vec<Effect>) {
    match (model.clone(), &msg) {
        (_, Msg::CameraEvent(DeviceCameraEvent::Disconnected)) => (
            Model::Connecting(ModelConnecting {
                camera: ModelDeviceConnection::Connecting,
                door: ModelDeviceConnection::Connected,
            }),
            vec![],
        ),

        (_, Msg::DoorEvent(DeviceDoorEvent::Disconnected)) => (
            Model::Connecting(ModelConnecting {
                camera: ModelDeviceConnection::Connected,
                door: ModelDeviceConnection::Connecting,
            }),
            vec![],
        ),

        _ => transition_ready_main(config, model, &msg),
    }
}

fn transition_ready_main(config: &Config, model: ModelReady, msg: &Msg) -> (Model, Vec<Effect>) {
    let door_result = transition_ready_door(model.door, &msg);
    let camera_result = transition_ready_camera(config, model.camera, &msg);

    let combined = ModelReady {
        camera: camera_result.0,
        door: door_result.0,
    };

    let mut combined_effects = door_result.1;
    combined_effects.extend(camera_result.1);

    (Model::Ready(combined), combined_effects)
}

fn transition_ready_door(model: ModelDoor, msg: &Msg) -> (ModelDoor, Vec<Effect>) {
    match (model.clone(), msg) {
        (ModelDoor::Locking { .. }, Msg::DoorLockDone(Ok(_))) => (ModelDoor::Locked, vec![]),

        (ModelDoor::Unlocking { .. }, Msg::DoorUnlockDone(Ok(_))) => (ModelDoor::Unlocked, vec![]),

        (ModelDoor::Locking { .. }, Msg::DoorLockDone(Err(_))) => {
            (ModelDoor::Locked, vec![Effect::LockDoor])
        }

        (ModelDoor::Unlocking { .. }, Msg::DoorUnlockDone(Err(_))) => {
            (ModelDoor::Locked, vec![Effect::UnlockDoor])
        }

        _ => (model, vec![]),
    }
}

fn transition_ready_camera(
    config: &Config,
    model: ModelCamera,
    msg: &Msg,
) -> (ModelCamera, Vec<Effect>) {
    match (model.clone(), msg) {
        (ModelCamera::Idle, Msg::Tick(_)) => (ModelCamera::Capturing, vec![Effect::CaptureFrames]),

        (ModelCamera::Capturing { .. }, Msg::FramesCaptureDone(Ok(frames))) => {
            if frames.is_empty() {
                return (ModelCamera::Idle, vec![]);
            }

            (
                ModelCamera::Classifying,
                vec![Effect::ClassifyFrames {
                    frames: frames.clone(),
                }],
            )
        }

        (ModelCamera::Capturing { .. }, Msg::FramesCaptureDone(Err(_))) => {
            (ModelCamera::Idle, vec![])
        }

        (ModelCamera::Classifying { .. }, Msg::FramesClassifyDone(Ok(classifications))) => {
            let outcome = to_classification_outcome(config, &classifications);

            match outcome {
                ClassificationOutcome::CatDetected => (ModelCamera::Idle, vec![Effect::LockDoor]),
                ClassificationOutcome::DogDetected => (ModelCamera::Idle, vec![Effect::UnlockDoor]),
                ClassificationOutcome::NoDetection => (ModelCamera::Idle, vec![]),
            }
        }

        (ModelCamera::Classifying { .. }, Msg::FramesClassifyDone(Err(_))) => {
            (ModelCamera::Idle, vec![])
        }

        (model, _) => (model, vec![]),
    }
}

enum ClassificationOutcome {
    CatDetected,
    DogDetected,
    NoDetection,
}

fn to_classification_outcome(
    config: &Config,
    classifications: &Vec<Vec<Classification>>,
) -> ClassificationOutcome {
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

    if cat_detected {
        ClassificationOutcome::CatDetected
    } else if dog_detected {
        ClassificationOutcome::DogDetected
    } else {
        ClassificationOutcome::NoDetection
    }
}
