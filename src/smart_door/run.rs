use super::main::SmartDoor;
use crate::smart_door::core::{transition, Model};
use std::sync::Arc;

impl SmartDoor {
    pub fn run(&self) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        let mut current_model = Model::default();

        loop {
            let msg = self.recv();

            let _ = self.logger.info(&format!(
                "\nold model:\n\t{:?}\n\\msg:\n\t{:?}",
                current_model, msg,
            ));

            let (new_model, effects) = transition(&self.config, current_model, msg);

            let _ = self.logger.info(&format!(
                "\nnew model:\n\t{:?}\n\neffects:\n\t{:?}",
                new_model, effects
            ));

            current_model = new_model.clone();

            _ = self.render(&current_model);

            for effect in effects {
                let effect_clone = effect.clone();
                let self_clone = self.clone();
                std::thread::spawn(move || self_clone.execute_effect(effect_clone));
            }
        }
    }
}
