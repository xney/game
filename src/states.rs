use bevy::prelude::*;
use iyes_loopless::prelude::*;

pub mod server {

    use super::*;
    // TODO: figure out if this is necessary
    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    pub enum GameState {
        Running,
    }

    /// Initial GameState
    impl Default for GameState {
        fn default() -> Self {
            GameState::Running
        }
    }

    /// Adds game state
    pub struct StatePlugin;

    impl Plugin for StatePlugin {
        fn build(&self, app: &mut App) {
            app.add_loopless_state(GameState::default());
        }
    }
}

pub mod client {
    use super::*;

    /// Represents runtime "flow" of the game
    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    pub enum GameState {
        ///menu
        Menu,
        /// Main game loop, game world exists with player
        InGame,
        /// Credits screen
        Credits,
    }

    /// Initial GameState
    impl Default for GameState {
        fn default() -> Self {
            GameState::Menu
        }
    }

    /// Adds game state
    pub struct StatePlugin;

    impl Plugin for StatePlugin {
        fn build(&self, app: &mut App) {
            app.add_loopless_state(GameState::default())
                .add_system(input_state_change)
                .add_system(ctrl_q_quit);
        }
    }

    /// Simple system to facilitate changing GameState via F1 key
    /// TODO: This is good enough for debugging, but should be reworked eventually
    fn input_state_change(
        mut commands: Commands,
        state: Res<CurrentState<GameState>>,
        input: Res<Input<KeyCode>>,
    ) {
        if input.just_pressed(KeyCode::F1) {
            let new_state = match state.0 {
                GameState::Menu => GameState::InGame,
                GameState::Credits => GameState::Menu,
                GameState::InGame => GameState::Credits,
            };
            info!(
                "attempting to change GameState from {:?} to {:?}",
                state.0, new_state
            );
            commands.insert_resource(NextState(new_state));
        }
    }
}

/// Immediately end the process
fn ctrl_q_quit(input: Res<Input<KeyCode>>) {
    if input.pressed(KeyCode::Q) && input.pressed(KeyCode::LControl) {
        warn!("ctrl-Q detected -- exiting!");
        std::process::exit(0);
    }
}
