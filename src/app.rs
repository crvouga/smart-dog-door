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

#[derive(Default)]
pub struct DeviceStates {
    camera: CameraState,
    dog_door: DogDoorState,
}

#[derive(Default)]
pub enum CameraState {
    #[default]
    Disconnected,
    Connected,
    Started,
}

#[derive(Default)]
pub enum DogDoorState {
    #[default]
    Disconnected,
    Connected,
    Initialized,
}

pub enum State {
    DevicesInitializing(DeviceStates), // Initial state, waiting for devices to connect and initialize
    CapturingFrame,                    // Main loop - capturing frame
    ClassifyingFrame,                  // Main loop - classifying frame
    ControllingDoor(DoorAction),       // Main loop - controlling door based on classification
    Sleeping(DoorAction),              // Main loop - waiting before next iteration
}

pub enum DoorAction {
    Locking,
    Unlocking,
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
        state: State::DevicesInitializing(DeviceStates::default()),
        effects: vec![
            Effect::SubscribeToCameraEvents,
            Effect::SubscribeToDoorEvents,
        ],
    }
}

pub fn reducer(state: State, event: Event, config: &Config) -> Output {
    match (state, event) {
        // Device connection handling
        (State::DevicesInitializing(mut device_states), Event::CameraConnected) => {
            device_states.camera = CameraState::Connected;
            Output {
                state: State::DevicesInitializing(device_states),
                effects: vec![Effect::StartCamera],
            }
        }
        (State::DevicesInitializing(mut device_states), Event::CameraStartDone(Ok(()))) => {
            device_states.camera = CameraState::Started;
            let state = match (device_states.camera, device_states.dog_door) {
                (CameraState::Started, DogDoorState::Initialized) => State::CapturingFrame,
                _ => State::DevicesInitializing(device_states),
            };
            Output {
                state,
                effects: vec![],
            }
        }
        (State::DevicesInitializing(mut device_states), Event::DogDoorConnected) => {
            device_states.dog_door = DogDoorState::Connected;
            Output {
                state: State::DevicesInitializing(device_states),
                effects: vec![Effect::LockDogDoor],
            }
        }
        (State::DevicesInitializing(mut device_states), Event::DogDoorLockDone(Ok(()))) => {
            device_states.dog_door = DogDoorState::Initialized;
            let (state, effect) = match (device_states.camera, device_states.dog_door) {
                (CameraState::Started, DogDoorState::Initialized) => {
                    (State::CapturingFrame, Effect::CaptureFrame)
                }
                _ => (State::DevicesInitializing(device_states), Effect::None),
            };
            Output {
                state,
                effects: vec![effect],
            }
        }

        // Main loop
        (State::CapturingFrame, Event::FrameCaptured { frame }) => Output {
            state: State::ClassifyingFrame,
            effects: vec![Effect::ClassifyFrame { frame }],
        },
        (State::ClassifyingFrame, Event::FrameClassifyDone { classifications }) => {
            let is_dog_in_frame = does_probably_have_dog_in_frame(&classifications, config);
            let is_cat_in_frame = does_probably_have_cat_in_frame(&classifications, config);

            if is_dog_in_frame && !is_cat_in_frame {
                Output {
                    state: State::ControllingDoor(DoorAction::Unlocking),
                    effects: vec![Effect::UnlockDogDoor],
                }
            } else {
                Output {
                    state: State::ControllingDoor(DoorAction::Locking),
                    effects: vec![Effect::LockDogDoor],
                }
            }
        }
        (State::ControllingDoor(door_action), Event::DogDoorLockDone(_))
        | (State::ControllingDoor(door_action), Event::DogDoorUnlockDone(_)) => Output {
            state: State::Sleeping(door_action),
            effects: vec![Effect::Sleep],
        },
        (State::Sleeping(_), Event::SleepCompleted(_)) => Output {
            state: State::CapturingFrame,
            effects: vec![Effect::CaptureFrame], // Back to start of main loop
        },

        // Device disconnection handling
        (_, Event::CameraDisconnected) => Output {
            state: State::DevicesInitializing(DeviceStates::default()),
            effects: vec![], // Wait for camera to reconnect
        },
        (_, Event::DogDoorDisconnected) => Output {
            state: State::DevicesInitializing(DeviceStates::default()),
            effects: vec![Effect::LockDogDoor], // Try to lock door when reconnected
        },

        // Default case
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
        State::DevicesInitializing(device_states) => match device_states.camera {
            CameraState::Disconnected => println!("Waiting for camera to connect..."),
            CameraState::Connected => println!("Starting camera..."),
            CameraState::Started => match device_states.dog_door {
                DogDoorState::Disconnected => println!("Waiting for dog door to connect..."),
                DogDoorState::Connected => println!("Initializing dog door..."),
                DogDoorState::Initialized => {}
            },
        },
        State::CapturingFrame => println!("Capturing frame..."),
        State::ClassifyingFrame => println!("Classifying frame..."),
        State::ControllingDoor(action) => match action {
            DoorAction::Locking => println!("Locking door..."),
            DoorAction::Unlocking => println!("Unlocking door..."),
        },
        State::Sleeping(action) => match action {
            DoorAction::Locking => println!("Sleeping... Door is locked"),
            DoorAction::Unlocking => println!("Sleeping... Door is unlocked"),
        },
    }
}

pub fn run(deps: Deps) -> Result<(), Box<dyn std::error::Error>> {
    let (event_sender, event_receiver) = mpsc::channel();
    let mut state = init(&deps.config);

    // Create a single worker thread to process effects sequentially
    let effect_sender = event_sender.clone();
    let effect_deps = deps.clone();
    let effect_thread = std::thread::spawn(move || {
        while let Ok(effect) = effect_receiver.recv() {
            run_effect(&effect_deps, effect, effect_sender.clone());
        }
    });

    for effect in state.effects {
        let effect_deps = deps.clone();
        let effect_sender = event_sender.clone();
        std::thread::spawn(move || {
            run_effect(&effect_deps, effect, effect_sender);
        });
    }

    loop {
        match event_receiver.recv() {
            Ok(event) => {
                let output = reducer(state, event, &deps.config);
                state = output.state;
                render(&deps, state);

                for effect in output.effects {
                    let effect_deps = deps.clone();
                    let effect_sender = event_sender.clone();
                    std::thread::spawn(move || {
                        run_effect(&effect_deps, effect, effect_sender);
                    });
                }
            }
            Err(e) => {
                return Err(Box::new(e));
            }
        }
    }
}
