#[cfg(test)]
mod core_test {

    use crate::config::{ClassificationConfig, Config};
    use crate::device_camera::interface::DeviceCameraEvent;
    use crate::device_door::interface::DeviceDoorEvent;
    use crate::image_classifier::interface::Classification;
    use crate::smart_door::core::{
        init, transition, CameraState, DoorAction, DoorState, Effect, Event, State,
    };

    #[test]
    fn test_init() {
        let (state, effects) = init();

        assert!(matches!(state, State::DevicesInitializing { .. }));
        assert_eq!(effects.len(), 3);
        assert!(effects.contains(&Effect::SubscribeToDoorEvents));
        assert!(effects.contains(&Effect::SubscribeToCameraEvents));
        assert!(effects.contains(&Effect::SubscribeTick));
    }

    #[test]
    fn test_camera_connection_flow() {
        let config = Config::default();
        let (initial_state, _) = init();

        // Test camera connects
        let (state, effects) = transition(
            &config,
            initial_state,
            Event::CameraEvent(DeviceCameraEvent::Connected),
        );

        match state.clone() {
            State::DevicesInitializing { device_states } => {
                assert!(matches!(device_states.camera, CameraState::Connected(_)));
                assert!(matches!(device_states.door, DoorState::Disconnected));
            }
            _ => panic!("Unexpected state"),
        }
        assert_eq!(effects, vec![Effect::StartCamera]);

        // Test camera start completes
        let (state, effects) = transition(&config, state, Event::CameraStartDone(Ok(())));

        match state {
            State::DevicesInitializing { device_states } => {
                assert!(matches!(device_states.camera, CameraState::Started));
            }
            _ => panic!("Unexpected state"),
        }
        assert!(effects.is_empty());
    }

    #[test]
    fn test_door_connection_flow() {
        let config = Config::default();
        let (initial_state, _) = init();

        // Test door connects
        let (state, effects) = transition(
            &config,
            initial_state,
            Event::DoorEvent(DeviceDoorEvent::Connected),
        );

        match state.clone() {
            State::DevicesInitializing { device_states } => {
                assert!(matches!(device_states.door, DoorState::Connected(_)));
                assert!(matches!(device_states.camera, CameraState::Disconnected));
            }
            _ => panic!("Unexpected state"),
        }
        assert_eq!(effects, vec![Effect::LockDoor]);

        // Test door lock completes
        let (state, effects) = transition(&config, state, Event::DoorLockDone(Ok(())));

        match state {
            State::DevicesInitializing { device_states } => {
                assert!(matches!(device_states.door, DoorState::Initialized));
            }
            _ => panic!("Unexpected state"),
        }
        assert!(effects.is_empty());
    }

    #[test]
    fn test_analyzing_frames_flow() {
        let mut config = Config::default();
        config.unlock_list = vec![ClassificationConfig {
            label: "dog".to_string(),
            min_confidence: 0.8,
        }];

        let state = State::AnalyzingFramesCapture {
            door_state: DoorState::Locked,
        };

        // Test empty frames
        let frames: Vec<Vec<u8>> = vec![];
        let (state, effects) =
            transition(&config, state.clone(), Event::FramesCaptureDone(Ok(frames)));

        match state {
            State::AnalyzingFramesCapture { .. } => (),
            _ => panic!("Unexpected state"),
        }
        assert_eq!(effects, vec![Effect::CaptureFrames]);

        // Test with frames
        let frames = vec![vec![1, 2, 3]];
        let (state, effects) =
            transition(&config, state, Event::FramesCaptureDone(Ok(frames.clone())));

        match state {
            State::AnalyzingFramesClassifying { .. } => (),
            _ => panic!("Unexpected state"),
        }
        assert_eq!(effects, vec![Effect::ClassifyFrames { frames }]);
    }

    #[test]
    fn test_unlocking_flow() {
        let mut config = Config::default();
        config.unlock_list = vec![ClassificationConfig {
            label: "dog".to_string(),
            min_confidence: 0.8,
        }];

        let state = State::AnalyzingFramesClassifying {
            door_state: DoorState::Locked,
        };

        let classifications = vec![vec![Classification {
            label: "dog".to_string(),
            confidence: 0.9,
        }]];

        let (state, effects) = transition(
            &config,
            state,
            Event::FramesClassifyDone(Ok(classifications)),
        );

        match state {
            State::ControllingDoor {
                action: DoorAction::Unlocking,
                ..
            } => (),
            _ => panic!("Unexpected state"),
        }
        assert_eq!(effects, vec![Effect::UnlockDoor]);
    }

    #[test]
    fn test_device_disconnection() {
        let config = Config::default();
        let state = State::AnalyzingFramesCapture {
            door_state: DoorState::Locked,
        };

        // Test camera disconnection
        let (state, effects) = transition(
            &config,
            state.clone(),
            Event::CameraEvent(DeviceCameraEvent::Disconnected),
        );

        match state.clone() {
            State::DevicesInitializing { device_states } => {
                assert!(matches!(device_states.camera, CameraState::Disconnected));
                assert!(matches!(device_states.door, DoorState::Disconnected));
            }
            _ => panic!("Unexpected state"),
        }
        assert!(effects.is_empty());

        // Test door disconnection
        let (state, effects) = transition(
            &config,
            state,
            Event::DoorEvent(DeviceDoorEvent::Disconnected),
        );

        match state {
            State::DevicesInitializing { device_states } => {
                assert!(matches!(device_states.camera, CameraState::Disconnected));
                assert!(matches!(device_states.door, DoorState::Disconnected));
            }
            _ => panic!("Unexpected state"),
        }
        assert!(effects.is_empty());
    }
}
