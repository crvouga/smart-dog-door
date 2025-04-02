use super::{core::Effect, main::SmartDoor};
use crate::smart_door::core::{init, transition};

impl SmartDoor {
    pub fn run(&self) {
        let (mut current_model, effects) = init();

        self.execute_effects(effects);

        loop {
            println!("recv");
            let msg = self.recv();
            println!("recv done");

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

            self.execute_effects(effects);
        }
    }

    fn execute_effects(&self, effects: Vec<Effect>) {
        for effect in effects {
            let effect_clone = effect.clone();
            let self_clone = self.clone();
            std::thread::spawn(move || self_clone.execute_effect(effect_clone));
        }
    }
}
