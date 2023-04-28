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
use std::net::{TcpListener, TcpStream};

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
        Game {
            menu_state : MenuState::new(ctx, image_pool), 
            game_state : GameState::new(ctx, image_pool), 
            current_state : EState::Menu,
        }
    }
}

impl EventHandler for Game {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let ret = match self.current_state {
            EState::Menu => self.menu_state.update(ctx),
            EState::Game => self.game_state.update(ctx),
            EState::None => EState::None, 
        };

        match ret {
            EState::Game => {
                self.current_state = EState::Game;
                self.game_state.initialize(
                    self.menu_state.IsServer(), 
                    &self.menu_state.tcp_stream
                );
                println!("Game Started!");
            },
            (_) => {},
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        clear(ctx, Color::BLACK);

        match self.current_state {
            EState::Menu => self.menu_state.draw(ctx),
            EState::Game => self.game_state.draw(ctx),
            EState::None => todo!(), 
        };

        present(ctx)?;
        Ok(())
    }

    fn key_down_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymods: KeyMods, repeat: bool) {
        match self.current_state {
            EState::Menu => self.menu_state.key_down_event(ctx, keycode, keymods, repeat),
            EState::Game => self.game_state.key_down_event(ctx, keycode, keymods, repeat),
            EState::None => todo!(), 
        }
    }

    fn key_up_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymods: KeyMods) {
        match self.current_state {
            EState::Menu => self.menu_state.key_up_event(ctx, keycode, keymods), 
            EState::Game => self.game_state.key_up_event(ctx, keycode, keymods), 
            EState::None => todo!(), 
        }
    }

    fn mouse_button_down_event(&mut self, ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
        match self.current_state {
            EState::Menu => self.menu_state.mouse_button_down_event(ctx, button, x, y), 
            EState::Game => self.game_state.mouse_button_down_event(ctx, button, x, y), 
            EState::None => todo!(), 
        }
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