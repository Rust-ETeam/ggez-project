use ggez::{Context, ContextBuilder, GameResult};
use ggez::graphics::{self, Image};
use ggez::event::{self, EventHandler, KeyCode, KeyMods};
use ggez::mint::Point2;

struct GGEZ {
    background_image: Image,
    foreground_image: Image,
    player_image: Image,
    player_x: f32,
    opponent_x: f32,
    player_state: i32,
    player_speed: f32,
}

impl EventHandler for GGEZ {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let delta = ggez::timer::delta(ctx);
        let dt = delta.as_secs() as f32 + delta.subsec_nanos() as f32 * 1e-9;

        if self.player_state == -1 { self.player_x -= dt * self.player_speed; }
        if self.player_state == 1 { self.player_x += dt * self.player_speed; }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, graphics::Color::WHITE);
        graphics::draw(ctx, &self.background_image, graphics::DrawParam::new())?;
        graphics::draw(ctx, &self.foreground_image, graphics::DrawParam::new())?;

        let player_draw_params = graphics::DrawParam::new()
            .dest(Point2 {x: self.player_x, y: 650.0})
            .rotation(if self.player_state == -1 { 135.0 * std::f32::consts::PI / 180.0 } else { if self.player_state == 1 { 225.0 * std::f32::consts::PI / 180.0 } else { 180.0 * std::f32::consts::PI / 180.0 }})
            .offset(Point2 {x: 0.5, y: 0.5 });
        graphics::draw(ctx, &self.player_image, player_draw_params)?;

        let opponent_draw_params = graphics::DrawParam::new()
            .dest(Point2 {x: self.opponent_x, y: 70.0})
            .rotation(0.0)
            .offset(Point2 {x: 0.5, y: 0.5 });
        graphics::draw(ctx, &self.player_image, opponent_draw_params)?;

        graphics::present(ctx)?;
        Ok(())
    }

    fn key_down_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymods: KeyMods, repeat: bool) {
        if ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Escape) {
            ggez::event::quit(ctx);
        }

        self.player_state = 0;
        if ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Left) { self.player_state = -1 }
        if ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Right) { self.player_state = 1 }
    }

    fn key_up_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymod: KeyMods) {
        self.player_state = 0;
        if ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Left) { self.player_state = -1 }
        if ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Right) { self.player_state = 1 }
    }
}

fn main() {
    let (mut ctx, event_loop) = match ggez::ContextBuilder::new("GGEZ", "GGEZ")
        .window_setup(ggez::conf::WindowSetup::default().title("GGEZ"))
        .window_mode(ggez::conf::WindowMode::default().dimensions(1280.0, 720.0))
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

    event::run(ctx, event_loop, GGEZ {
        background_image: background_image,
        foreground_image: foreground_image,
        player_image: player_image,
        player_speed: 300.0,
        player_x: 640.0,
        opponent_x: 640.0,
        player_state: 0,
    });
}
