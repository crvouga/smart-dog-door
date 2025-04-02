#[cfg(test)]
mod tests {
    use crate::{
        config::Config,
        smart_door::core::{
            transition, Effect, Model, ModelCamera, ModelCameraState, ModelConnecting, ModelReady,
            Msg,
        },
    };
    use std::time::Instant;

    #[test]
    fn test_init() {
        let config = Config::default();
        let model = Model::default();
        let (model, effects) = transition(&config, model, Msg::Tick(Instant::now()));

        assert_eq!(model, Model::Connecting(ModelConnecting::default()));
        assert_eq!(
            effects,
            vec![
                Effect::SubscribeDoor,
                Effect::SubscribeCamera,
                Effect::SubscribeTick,
            ]
        );
    }

    #[test]
    fn test_camera_idle_to_capturing() {
        let config = Config::default();
        let now = Instant::now();

        let model = Model::Ready(ModelReady {
            camera: ModelCamera {
                state: ModelCameraState::Idle {
                    start_time: now - config.minimal_rate_camera_process,
                },
                ..Default::default()
            },
            ..Default::default()
        });

        let (_new_model, effects) = transition(&config, model, Msg::Tick(now));

        assert_eq!(effects, vec![Effect::CaptureFrames]);
    }
}
