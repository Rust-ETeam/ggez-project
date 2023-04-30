use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::quit;
use ggez::event::{run, EventHandler, KeyCode, KeyMods, MouseButton};
use ggez::graphics::{clear, draw, present, Color, DrawParam, Image, Text};
use ggez::input::gamepad::Event;
use ggez::input::keyboard::is_key_pressed;
use ggez::mint::Point2;
use ggez::timer::delta;
use ggez::{Context, ContextBuilder, GameResult, GameError};

use std::cell::RefCell;
use std::f32::consts::PI;
use std::rc::Rc;

use rand::rngs::ThreadRng;
use rand::{thread_rng, Rng};

use std::collections::HashMap;
use std::io::Read;
use std::io::Write;

mod helper;
use helper::{EState, load_image, IState};
mod game_state;
mod menu_state;
use game_state::GameState;
use menu_state::MenuState;

struct Game {
    game_state: GameState, 
    menu_state: MenuState, 
    current_state : EState, 
}

impl Game {
    fn new(
        ctx: &mut Context, 
        image_pool : &mut HashMap<String, Rc<Image>>,
    ) -> Game {
        let mut gs = GameState::new(ctx, image_pool);
        gs.initialize();

        Game {
            menu_state : MenuState::new(ctx, image_pool), 
            game_state : gs,
            current_state : EState::None,
        }
    }
}

impl EventHandler for Game {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        self.game_state.update(ctx);
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        clear(ctx, Color::BLACK);

        self.game_state.draw(ctx);

        present(ctx)?;
        Ok(())
    }

    fn key_down_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymods: KeyMods, repeat: bool) {
        self.game_state.key_down_event(ctx, keycode, keymods, repeat);
    }

    fn key_up_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymods: KeyMods) {
        self.game_state.key_up_event(ctx, keycode, keymods);
    }
}

fn main() {
    let (mut ctx, event_loop) = match ggez::ContextBuilder::new("GGEZ", "GGEZ")
        .window_setup(ggez::conf::WindowSetup::default().title("GGEZ"))
        .window_mode(ggez::conf::WindowMode::default().dimensions(1280.0, 720.0))
        .build()
    {
        Ok(res) => res,
        Err(err) => panic!("Failed to build context: {}", err),
    };

    let mut image_pool: HashMap<String, Rc<Image>> = HashMap::new();

    let ggez = Game::new(&mut ctx, &mut image_pool);
    run(ctx, event_loop, ggez);
}