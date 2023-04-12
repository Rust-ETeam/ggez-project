use ggez::{Context, ContextBuilder, GameResult};
use ggez::conf::{WindowSetup, WindowMode};
use ggez::graphics::{Image, Color, DrawParam, draw, clear, present};
use ggez::event::{run, EventHandler, KeyCode, KeyMods};
use ggez::timer::delta;
use ggez::input::keyboard::is_key_pressed;
use ggez::event::quit;
use ggez::mint::Point2;
use std::f32::consts::PI;

struct GGEZ {
    background_image: Image,
    foreground_image: Image,
    target_image: Image,
    player_image: Image,
    player_x: f32,
    player_target_x: f32,
    player_target_direction: i32,
    opponent_x: f32,
    player_state: i32,
    player_speed: f32,
    target_speed: f32,
}

impl EventHandler for GGEZ {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let delta = delta(ctx);
        let dt = delta.as_secs() as f32 + delta.subsec_nanos() as f32 * 1e-9;

        if self.player_state == -1 { self.player_x -= dt * self.player_speed; }
        if self.player_state == 1 { self.player_x += dt * self.player_speed; }

        if self.player_x < 300.0 { self.player_x = 300.0; }
        if self.player_x > 980.0 { self.player_x = 980.0; }

        if self.player_target_direction == -1 {
            self.player_target_x -= self.target_speed * dt;
            if self.player_target_x < 200.0 {
                self.player_target_x = 200.0;
                self.player_target_direction = 1;
            }
        }
        if self.player_target_direction == 1 {
            self.player_target_x += self.target_speed * dt;
            if self.player_target_x > 1080.0 {
                self.player_target_x = 1080.0;
                self.player_target_direction *= -1;
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        clear(ctx, Color::WHITE);
        draw(ctx, &self.background_image, DrawParam::new())?;
        draw(ctx, &self.foreground_image, DrawParam::new())?;

        let player_target_radian = (580.0_f64).atan2((self.player_x - self.player_target_x) as f64) as f32;
        let player_target_draw_params = DrawParam::new()
            .dest(Point2 {x: self.player_x, y: 650.0})
            .rotation(player_target_radian + PI / 2.0)
            .offset(Point2 {x: 0.5, y: 0.0 })
            .color(Color::CYAN);
        draw(ctx, &self.target_image, player_target_draw_params)?;

        let player_draw_params = DrawParam::new()
            .dest(Point2 {x: self.player_x, y: 650.0})
            .rotation(if self.player_state == -1 { 135.0 * PI / 180.0 } else { if self.player_state == 1 { 225.0 * PI / 180.0 } else { 180.0 * PI / 180.0 }})
            .offset(Point2 {x: 0.5, y: 0.5 });
        draw(ctx, &self.player_image, player_draw_params)?;

        let opponent_draw_params = DrawParam::new()
            .dest(Point2 {x: self.opponent_x, y: 70.0})
            .rotation(0.0)
            .offset(Point2 {x: 0.5, y: 0.5 });
        draw(ctx, &self.player_image, opponent_draw_params)?;

        present(ctx)?;
        Ok(())
    }

    fn key_down_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymods: KeyMods, repeat: bool) {
        if is_key_pressed(ctx, KeyCode::Escape) {
            quit(ctx);
        }

        self.player_state = 0;
        if is_key_pressed(ctx, KeyCode::Left) { self.player_state = -1 }
        if is_key_pressed(ctx, KeyCode::Right) { self.player_state = 1 }
    }

    fn key_up_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymod: KeyMods) {
        self.player_state = 0;
        if is_key_pressed(ctx, KeyCode::Left) { self.player_state = -1 }
        if is_key_pressed(ctx, KeyCode::Right) { self.player_state = 1 }
    }
}

fn main() {
    let (mut ctx, event_loop) = match ContextBuilder::new("GGEZ", "GGEZ")
        .window_setup(WindowSetup::default().title("GGEZ"))
        .window_mode(WindowMode::default().dimensions(1280.0, 720.0))
        .build() {
        Ok(res) => res,
        Err(err) => panic!("Failed to build context: {}", err)
    };

    let background_image = match Image::new(&mut ctx, "/background.png") {
        Ok(res) => res,
        Err(err) => panic!("Failed to load image: {}", err)
    };
    let foreground_image = match Image::new(&mut ctx, "/foreground.png") {
        Ok(res) => res,
        Err(err) => panic!("Failed to load image: {}", err)
    };
    let player_image = match Image::new(&mut ctx, "/player.png") {
        Ok(res) => res,
        Err(err) => panic!("Failed to load image: {}", err)
    };
    let target_image = match Image::new(&mut ctx, "/target.png") {
        Ok(res) => res,
        Err(err) => panic!("Failed to load image: {}", err)
    };

    run(ctx, event_loop, GGEZ {
        background_image: background_image,
        foreground_image: foreground_image,
        player_image: player_image,
        target_image: target_image,
        player_speed: 300.0,
        player_x: 640.0,
        player_target_x: 980.0,
        player_target_direction: -1,
        opponent_x: 640.0,
        player_state: 0,
        target_speed: 800.0,
    });
}