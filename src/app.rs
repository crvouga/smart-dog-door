use crate::config::Config;
use crate::device_camera::interface::DeviceCamera;
use crate::device_display::interface::DeviceDisplay;
use crate::device_dog_door::interface::DeviceDogDoor;
use crate::image_classifier::interface::{Classification, ImageClassifier};
use crate::logger::interface::Logger;
use std::sync::mpsc::Sender;

pub struct Deps {
    config: Config,
    logger: Box<dyn Logger>,
    device_camera: Box<dyn DeviceCamera>,
    device_dog_door: Box<dyn DeviceDogDoor>,
    device_display: Box<dyn DeviceDisplay>,
    image_classifier: Box<dyn ImageClassifier>,
}

pub enum State {
    CameraDeviceConnecting,
    DogDoorDeviceConnecting,
    CapturingCameraFrame,
    ClassifyingFrame,
    UnlockingDogDoor,
    LockingDogDoor,
    Sleeping,
}

pub enum Event {
    CameraDisconnected,
    CameraConnected,
    CameraStarting,
    CameraStartDone(Result<(), Box<dyn std::error::Error>>),
    CameraStopping,
    CameraStopDone(Result<(), Box<dyn std::error::Error>>),
    DogDoorConnected,
    DogDoorDisconnected,
    DogDoorLocking,
    DogDoorLockDone(Result<(), Box<dyn std::error::Error>>),
    DogDoorUnlocking,
    DogDoorUnlockDone(Result<(), Box<dyn std::error::Error>>),
    FrameClassifying,
    FrameClassifyDone {
        classifications: Vec<Classification>,
    },
    FrameCapturing,
    FrameCaptured {
        frame: Vec<u8>,
    },
    Sleeping,
    SleepCompleted(Result<(), Box<dyn std::error::Error>>),
}

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

pub struct Output {
    state: State,
    effects: Vec<Effect>,
}

pub fn init(config: &Config) -> Output {
    Output {
        state: State::CameraDeviceConnecting,
        effects: vec![
            Effect::SubscribeToCameraEvents,
            Effect::SubscribeToDoorEvents,
            Effect::StartCamera,
        ],
    }
}

pub fn reducer(state: State, event: Event, config: &Config) -> Output {
    match (state, event) {
        (State::CameraDeviceConnecting, Event::CameraConnected) => Output {
            state: State::CameraDeviceConnecting,
            effects: vec![Effect::StartCamera],
        },
        (State::CameraDeviceConnecting, Event::CameraDisconnected) => Output {
            state: State::CameraDeviceConnecting,
            effects: vec![Effect::StartCamera],
        },
        (State::CameraDeviceConnecting, Event::DogDoorConnected) => Output {
            state: State::DogDoorDeviceConnecting,
            effects: vec![Effect::LockDogDoor],
        },
        (State::DogDoorDeviceConnecting, Event::DogDoorDisconnected) => Output {
            state: State::CameraDeviceConnecting,
            effects: vec![Effect::StartCamera],
        },
        (State::DogDoorDeviceConnecting, Event::DogDoorLockDone) => Output {
            state: State::CapturingCameraFrame,
            effects: vec![Effect::CaptureFrame],
        },
        (State::CapturingCameraFrame, Event::FrameCaptured { frame }) => Output {
            state: State::ClassifyingFrame,
            effects: vec![Effect::ClassifyFrame { frame }],
        },
        (State::ClassifyingFrame, Event::FrameClassifyDone { classifications }) => {
            let is_dog_in_frame = does_probably_have_dog_in_frame(&classifications, config);

            let is_cat_in_frame = does_probably_have_cat_in_frame(&classifications, config);

            let should_unlock = is_dog_in_frame && !is_cat_in_frame;

            if should_unlock {
                Output {
                    state,
                    effects: vec![Effect::UnlockDogDoor],
                }
            } else {
                Output {
                    state,
                    effects: vec![Effect::LockDogDoor],
                }
            }
        }
        (State::Sleeping, Event::SleepCompleted(_)) => Output {
            state: State::CapturingCameraFrame,
            effects: vec![Effect::CaptureFrame],
        },
        (State::Sleeping, Event::DogDoorUnlockDone) => Output {
            state: State::CapturingCameraFrame,
            effects: vec![Effect::CaptureFrame],
        },
        (State::Sleeping, Event::DogDoorLockDone) => Output {
            state: State::CapturingCameraFrame,
            effects: vec![Effect::CaptureFrame],
        },
        (State::Sleeping, Event::DogDoorConnected) => Output {
            state: State::DogDoorDeviceConnecting,
            effects: vec![Effect::LockDogDoor],
        },
        (State::Sleeping, Event::DogDoorDisconnected) => Output {
            state: State::CameraDeviceConnecting,
            effects: vec![Effect::StartCamera],
        },
        _ => Output {
            state,
            effects: vec![Effect::None],
        },
    }
}

fn does_probably_have_dog_in_frame(classifications: &[Classification], config: &Config) -> bool {
    classifications.iter().any(|c| {
        c.label.to_lowercase().contains("dog")
            && c.confidence >= config.classification_min_confidence_dog
    })
}

fn does_probably_have_cat_in_frame(classifications: &[Classification], config: &Config) -> bool {
    classifications.iter().any(|c| {
        c.label.to_lowercase().contains("cat")
            && c.confidence >= config.classification_min_confidence_cat
    })
}

fn run_effect(deps: &Deps, effect: Effect, event_queue: Sender<Event>) {
    match effect {
        Effect::ListenForCameraEvents => {
            let camera_events_tx = deps.device_camera.events();
            let rx = camera_events_tx.recv();
            loop {
                match rx {
                    Ok(event) => {
                        event_queue.send(event).unwrap();
                        rx = camera_events_tx.recv();
                    }
                    Err(_) => continue,
                }
            }
        }
        Effect::ListenForDoorEvents => {
            let dog_door_events_tx = deps.device_dog_door.events();
            let rx = dog_door_events_tx.recv();
            loop {
                match rx {
                    Ok(event) => {
                        event_queue.send(event).unwrap();
                        rx = dog_door_events_tx.recv();
                    }
                    Err(_) => continue,
                }
            }
        }
        Effect::StartCamera => {
            event_queue.send(Event::CameraStarting).unwrap();
            let started = deps.device_camera.start();
            event_queue.send(Event::CameraStartDone(started)).unwrap();
        }
        Effect::StopCamera => {
            event_queue.send(Event::CameraStopping).unwrap();
            let stopped = deps.device_camera.stop();
            event_queue.send(Event::CameraStopDone(stopped)).unwrap();
        }
        Effect::LockDogDoor => {
            event_queue.send(Event::DogDoorLocking).unwrap();
            let locked = deps.device_dog_door.lock();
            event_queue.send(Event::DogDoorLockDone(locked)).unwrap();
        }
        Effect::UnlockDogDoor => {
            event_queue.send(Event::DogDoorUnlocking).unwrap();
            let unlocked = deps.device_dog_door.unlock();
            event_queue
                .send(Event::DogDoorUnlockDone(unlocked))
                .unwrap();
        }
        Effect::CaptureFrame => {
            event_queue.send(Event::FrameCapturing).unwrap();
            let frame = deps.device_camera.capture_frame();
            event_queue.send(Event::FrameCaptured { frame }).unwrap();
        }
        Effect::ClassifyFrame { frame } => {
            event_queue.send(Event::FrameClassifying).unwrap();
            let classifications = deps.image_classifier.classify(&frame);
            event_queue
                .send(Event::FrameClassifyDone { classifications })
                .unwrap();
        }
        Effect::Sleep => {
            event_queue.send(Event::Sleeping).unwrap();
            let slept = deps.sleep();
            event_queue.send(Event::SleepCompleted(slept)).unwrap();
        }
        Effect::None => {}
    }
}

pub fn render(deps: &Deps, state: State) {
    match state {
        State::CameraDeviceConnecting => println!("Camera device connecting..."),
        State::DogDoorDeviceConnecting => println!("Dog door device connecting..."),
        State::CapturingCameraFrame => println!("Capturing camera frame..."),
        State::ClassifyingFrame => println!("Classifying frame..."),
        State::UnlockingDogDoor => println!("Unlocking dog door..."),
        State::LockingDogDoor => println!("Locking dog door..."),
        State::Sleeping => println!("Sleeping..."),
    }
}

pub fn run(deps: Deps) -> Result<(), Box<dyn std::error::Error>> {
    let (event_sender, event_receiver) = mpsc::channel();

    let (mut state, initial_effects) = init(&deps.config);

    for effect in initial_effects {
        let deps = deps.clone();
        let event_sender = event_sender.clone();
        std::thread::spawn(move || {
            run_effect(&deps, effect, event_sender);
        });
    }

    loop {
        match event_receiver.recv() {
            Ok(event) => {
                let (new_state, effects) = reduce(state, event, &deps.config);
                state = new_state;
                render(config, state);

                for effect in effects {
                    let deps = deps.clone();
                    let event_sender = event_sender.clone();
                    std::thread::spawn(move || {
                        run_effect(&deps, effect, event_sender);
                    });
                }
            }
            Err(e) => {
                return Err(Box::new(e));
            }
        }
    }
}
