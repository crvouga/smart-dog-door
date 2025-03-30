use std::marker::PhantomData;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct StateMachine<TState, TEvent, TEffect, T, R, E>
where
    T: Fn(TState, TEvent) -> (TState, Vec<TEffect>) + Send + Sync,
    R: Fn(&TState) + Send + Sync,
    E: Fn(TEffect, std::sync::mpsc::Sender<TEvent>) + Send + Sync,
{
    pub init: (TState, Vec<TEffect>),
    pub transition_fn: Arc<T>,
    pub render_fn: Arc<R>,
    pub run_effect_fn: Arc<E>,
    _event: PhantomData<TEvent>,
}

impl<TState, TEvent, TEffect, T, R, E> StateMachine<TState, TEvent, TEffect, T, R, E>
where
    TState: Clone + Send + 'static,
    TEvent: Send + 'static,
    TEffect: Clone + Send + 'static,
    T: Fn(TState, TEvent) -> (TState, Vec<TEffect>) + Send + Sync + 'static,
    R: Fn(&TState) + Send + Sync + 'static,
    E: Fn(TEffect, std::sync::mpsc::Sender<TEvent>) + Send + Sync + 'static,
{
    pub fn new(
        init: (TState, Vec<TEffect>),
        transition_fn: T,
        render_fn: R,
        run_effect_fn: E,
    ) -> Self {
        Self {
            init,
            transition_fn: Arc::new(transition_fn),
            render_fn: Arc::new(render_fn),
            run_effect_fn: Arc::new(run_effect_fn),
            _event: PhantomData,
        }
    }

    pub fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (event_sender, event_receiver) = std::sync::mpsc::channel();
        let (state, effects) = self.init.clone();

        // Process initial effects
        for effect in effects {
            let effect_sender = event_sender.clone();
            let effect_clone = effect.clone();
            let run_effect_fn = Arc::clone(&self.run_effect_fn);
            std::thread::spawn(move || {
                run_effect_fn(effect_clone, effect_sender);
            });
        }

        // Main loop
        loop {
            match event_receiver.recv() {
                Ok(event) => {
                    let (new_state, new_effects) = (self.transition_fn)(state.clone(), event);
                    (self.render_fn)(&new_state);

                    // Process new effects
                    for effect in new_effects {
                        let effect_sender = event_sender.clone();
                        let effect_clone = effect.clone();
                        let run_effect_fn = Arc::clone(&self.run_effect_fn);
                        std::thread::spawn(move || {
                            run_effect_fn(effect_clone, effect_sender);
                        });
                    }
                }
                Err(e) => {
                    return Err(Box::new(e));
                }
            }
        }
    }
}
