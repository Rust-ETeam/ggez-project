use std::collections::HashMap;
use std::rc::Rc;

use ggez::{Context};
use ggez::event::{EventHandler, KeyCode, KeyMods, Button, MouseButton};
use ggez::graphics::Image;

pub enum EState {
    Menu, Game, 
    None    // None means no transision for IState::update() return value
}

pub trait IState {
    fn update(&mut self, _ctx: &mut ggez::Context) -> EState{
        EState::None
    }

    fn draw(&mut self, ctx: &mut ggez::Context){}

    fn key_down_event(&mut self, _: &mut Context, keycode: KeyCode, keymods: KeyMods, repeat: bool) {}

    fn key_up_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymods: KeyMods) {}

    fn mouse_button_down_event(&mut self, _ctx: &mut Context, _button: MouseButton, x: f32, y: f32) {}
}

pub fn load_image(
    ctx: &mut Context,
    path: String,
    image_pool: &mut HashMap<String, Rc<Image>>,
) -> Rc<Image> {
    match image_pool.get(&path) {
        Some(image) => Rc::clone(image),
        None => match Image::new(ctx, path.clone()) {
            Ok(res) => {
                let image = Rc::new(res);
                image_pool.insert(path.clone(), Rc::clone(&image));
                image
            }
            Err(err) => panic!("Failed to load image: {}", err),
        },
    }
}