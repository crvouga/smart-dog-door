use crate::app::{init, reducer, Effect, Event, Output, State};
use crate::config::Config;

#[test]
fn test_init() {
    let config = Config::default();
    let output = init(&config);

    assert!(matches!(output.state, State::WaitingForCamera));
    assert_eq!(output.effects.len(), 2);
    assert!(matches!(output.effects[0], Effect::SubscribeToCameraEvents));
    assert!(matches!(output.effects[1], Effect::SubscribeToDoorEvents));
}

#[test]
fn test_camera_connection_flow() {
    let config = Config::default();

    // Camera connects
    let output = reducer(State::WaitingForCamera, Event::CameraConnected, &config);
    assert!(matches!(output.state, State::StartingCamera));
    assert_eq!(output.effects.len(), 1);
    assert!(matches!(output.effects[0], Effect::StartCamera));

    // Camera starts successfully
    let output = reducer(
        State::StartingCamera,
        Event::CameraStartDone(Ok(())),
        &config,
    );
    assert!(matches!(output.state, State::WaitingForDogDoor));
    assert!(output.effects.is_empty());

    // Camera fails to start
    let output = reducer(
        State::StartingCamera,
        Event::CameraStartDone(Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "test error",
        )))),
        &config,
    );
    assert!(matches!(output.state, State::WaitingForCamera));
    assert!(output.effects.is_empty());
}

#[test]
fn test_dog_door_connection_flow() {
    let config = Config::default();

    // Dog door connects
    let output = reducer(State::WaitingForDogDoor, Event::DogDoorConnected, &config);
    assert!(matches!(output.state, State::InitializingDogDoor));
    assert_eq!(output.effects.len(), 1);
    assert!(matches!(output.effects[0], Effect::LockDogDoor));

    // Dog door initializes
    let output = reducer(
        State::InitializingDogDoor,
        Event::DogDoorLockDone(Ok(())),
        &config,
    );
    assert!(matches!(output.state, State::CapturingFrame));
    assert_eq!(output.effects.len(), 1);
    assert!(matches!(output.effects[0], Effect::CaptureFrame));
}

#[test]
fn test_main_loop_flow() {
    let config = Config::default();
    let frame = vec![0u8; 100];

    // Capture frame
    let output = reducer(
        State::CapturingFrame,
        Event::FrameCaptured {
            frame: frame.clone(),
        },
        &config,
    );
    assert!(matches!(output.state, State::ClassifyingFrame));
    assert_eq!(output.effects.len(), 1);
    assert!(matches!(
        output.effects[0],
        Effect::ClassifyFrame { frame: _ }
    ));

    // Classify frame - dog detected
    let output = reducer(
        State::ClassifyingFrame,
        Event::FrameClassifyDone {
            classifications: vec![crate::image_classifier::interface::Classification {
                label: "dog".to_string(),
                confidence: 0.9,
            }],
        },
        &config,
    );
    assert!(matches!(output.state, State::ControllingDoor));
    assert_eq!(output.effects.len(), 1);
    assert!(matches!(output.effects[0], Effect::UnlockDogDoor));

    // Door control complete
    let output = reducer(
        State::ControllingDoor,
        Event::DogDoorUnlockDone(Ok(())),
        &config,
    );
    assert!(matches!(output.state, State::Sleeping));
    assert_eq!(output.effects.len(), 1);
    assert!(matches!(output.effects[0], Effect::Sleep));

    // Sleep complete
    let output = reducer(State::Sleeping, Event::SleepCompleted(Ok(())), &config);
    assert!(matches!(output.state, State::CapturingFrame));
    assert_eq!(output.effects.len(), 1);
    assert!(matches!(output.effects[0], Effect::CaptureFrame));
}

#[test]
fn test_device_disconnection_handling() {
    let config = Config::default();

    // Camera disconnects from any state
    let output = reducer(State::CapturingFrame, Event::CameraDisconnected, &config);
    assert!(matches!(output.state, State::WaitingForCamera));
    assert!(output.effects.is_empty());

    // Dog door disconnects from any state
    let output = reducer(State::CapturingFrame, Event::DogDoorDisconnected, &config);
    assert!(matches!(output.state, State::WaitingForDogDoor));
    assert_eq!(output.effects.len(), 1);
    assert!(matches!(output.effects[0], Effect::LockDogDoor));
}

#[test]
fn test_cat_detection() {
    let config = Config::default();

    // Classify frame - cat detected alongside dog
    let output = reducer(
        State::ClassifyingFrame,
        Event::FrameClassifyDone {
            classifications: vec![
                crate::image_classifier::interface::Classification {
                    label: "dog".to_string(),
                    confidence: 0.9,
                },
                crate::image_classifier::interface::Classification {
                    label: "cat".to_string(),
                    confidence: 0.9,
                },
            ],
        },
        &config,
    );
    assert!(matches!(output.state, State::ControllingDoor));
    assert_eq!(output.effects.len(), 1);
    assert!(matches!(output.effects[0], Effect::LockDogDoor));
}
