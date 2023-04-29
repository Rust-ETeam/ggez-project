use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::quit;
use ggez::event::{run, EventHandler, KeyCode, KeyMods};
use ggez::graphics::{clear, draw, Color, DrawParam, Image, Text};
use ggez::input::keyboard::is_key_pressed;
use ggez::mint::Point2;
use ggez::timer::delta;
use ggez::{Context, ContextBuilder, GameResult};

use std::cell::RefCell;
use std::f32::consts::PI;
use std::rc::Rc;

use rand::rngs::ThreadRng;
use rand::{thread_rng, Rng};

use std::collections::HashMap;
use std::io::Read;
use std::io::Write;
use std::net::{TcpListener, TcpStream};

use crate::helper::{load_image, EState, IState};

pub trait Communication {
    fn get_send_data(&self) -> Vec<u8>;
    fn set_recv_data(&mut self, buf: &mut Vec<u8>);
}

#[derive(Clone, Copy, PartialEq)]
struct Transform {
    position: Point2<f32>,
    rotation: f32,
    scale: Point2<f32>,
}

impl Transform {
    fn new() -> Transform {
        Transform {
            position: Point2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
            scale: Point2 { x: 1.0, y: 1.0 },
        }
    }

    fn forward(&self) -> Point2<f32> {
        Point2 {
            x: self.rotation.cos(),
            y: self.rotation.sin(),
        }
    }

    fn back(&self) -> Point2<f32> {
        Point2 {
            x: -self.rotation.cos(),
            y: -self.rotation.sin(),
        }
    }

    fn right(&self) -> Point2<f32> {
        Point2 {
            x: (self.rotation + PI / 2.0).cos(),
            y: (self.rotation + PI / 2.0).sin(),
        }
    }

    fn left(&self) -> Point2<f32> {
        Point2 {
            x: (self.rotation - PI / 2.0).cos(),
            y: (self.rotation - PI / 2.0).sin(),
        }
    }

    fn rotate(&mut self, rotation: f32) {
        self.rotation += rotation
    }

    fn move_offset(&mut self, offset: Point2<f32>) {
        self.position.x += offset.x;
        self.position.y += offset.y;
    }

    fn move_offset_x(&mut self, offset_x: f32) {
        self.position.x += offset_x;
    }

    fn move_offset_y(&mut self, offset_y: f32) {
        self.position.y += offset_y;
    }

    fn magnitude(&self) -> f32 {
        (self.position.x.powf(2.0) + self.position.y.powf(2.0)).sqrt()
    }

    fn rotate_point(point: Point2<f32>, rotation: f32) -> Point2<f32> {
        let magnitude = (point.x.powf(2.0) + point.y.powf(2.0)).sqrt();
        let radian = point.y.atan2(point.x) + rotation;
        Point2 {
            x: magnitude * radian.cos(),
            y: magnitude * radian.sin(),
        }
    }
}

struct GameObject {
    transform: Transform,
    global_transform: Transform,
    rc_parent: Option<Rc<RefCell<GameObject>>>,
}

impl GameObject {
    fn new() -> GameObject {
        GameObject {
            transform: Transform::new(),
            global_transform: Transform::new(),
            rc_parent: None,
        }
    }

    fn set_rc_parent(&mut self, rc_parent: &Rc<RefCell<GameObject>>) {
        self.rc_parent = Some(Rc::clone(rc_parent));
    }

    fn update_global_transform(&mut self) {
        if let Some(rc_parent) = self.rc_parent.clone() {
            let parent = rc_parent.borrow();
            self.global_transform.rotation =
                self.transform.rotation + parent.global_transform.rotation;

            self.global_transform.scale.x =
                self.transform.scale.x * parent.global_transform.scale.x;
            self.global_transform.scale.y =
                self.transform.scale.y * parent.global_transform.scale.y;

            let scaled_pos_x = self.transform.position.x * self.global_transform.scale.x;
            let scaled_pos_y = self.transform.position.y * self.global_transform.scale.y;
            let relative_position = Transform::rotate_point(
                Point2 {
                    x: scaled_pos_x,
                    y: scaled_pos_y,
                },
                parent.global_transform.rotation,
            );
            self.global_transform.position.x =
                parent.global_transform.position.x + relative_position.x;
            self.global_transform.position.y =
                parent.global_transform.position.y + relative_position.y;
        } else {
            self.global_transform = self.transform;
        }
    }

    fn update_local_transform(&mut self) {
        if let Some(rc_parent) = self.rc_parent.clone() {
            let parent = rc_parent.borrow();
            self.transform.rotation =
                self.global_transform.rotation - parent.global_transform.rotation;

            self.transform.scale.x =
                self.global_transform.scale.x / parent.global_transform.scale.x;
            self.global_transform.scale.y =
                self.global_transform.scale.y / parent.global_transform.scale.y;

            let relative_x = self.global_transform.position.x - parent.global_transform.position.x;
            let relative_y = self.global_transform.position.y - parent.global_transform.position.y;
            let local_scaled_position = Transform::rotate_point(
                Point2 {
                    x: relative_x,
                    y: relative_y,
                },
                -parent.global_transform.rotation,
            );
            self.transform.position.x = local_scaled_position.x / self.transform.scale.x;
            self.transform.position.y = local_scaled_position.y / self.transform.scale.y;
        } else {
            self.transform = self.global_transform;
        }
    }
}

impl Communication for GameObject {
    fn get_send_data(&self) -> Vec<u8> {
        let mut data = vec![];

        let px = self.transform.position.x.to_ne_bytes();
        let py = self.transform.position.y.to_ne_bytes();
        let r = self.transform.rotation.to_ne_bytes();
        let sx = self.transform.scale.x.to_ne_bytes();
        let sy = self.transform.scale.y.to_ne_bytes();

        data.extend_from_slice(&px);
        data.extend_from_slice(&py);
        data.extend_from_slice(&r);
        data.extend_from_slice(&sx);
        data.extend_from_slice(&sy);
        data
    }

    fn set_recv_data(&mut self, buf: &mut Vec<u8>) {
        let data = buf.as_slice();
        let px = f32::from_ne_bytes(data[0..4].try_into().unwrap());
        let py = f32::from_ne_bytes(data[4..8].try_into().unwrap());
        let r = f32::from_ne_bytes(data[8..12].try_into().unwrap());
        let sx = f32::from_ne_bytes(data[12..16].try_into().unwrap());
        let sy = f32::from_ne_bytes(data[16..20].try_into().unwrap());
        self.transform.position.x = px;
        self.transform.position.y = py;
        self.transform.rotation = r;
        self.transform.scale.x = sx;
        self.transform.scale.y = sy;
        self.update_global_transform();
    }
}

//' state {0: nothing, 1: throw, -1: catched}
struct Grab {
    hand_image: Rc<Image>,
    string_image: Rc<Image>,
    rc_gameobject: Rc<RefCell<GameObject>>,
    rc_target: Option<Rc<RefCell<Character>>>,
    threshold: f32,
    speed: f32,
    state: f32,
    check_grab_once: bool,
}

impl Grab {
    fn new(ctx: &mut Context, image_pool: &mut HashMap<String, Rc<Image>>) -> Grab {
        Grab {
            hand_image: load_image(ctx, String::from("/grab_hand.png"), image_pool),
            string_image: load_image(ctx, String::from("/grab_string.png"), image_pool),
            rc_gameobject: Rc::new(RefCell::new(GameObject::new())),
            rc_target: None,
            threshold: 580.0,
            speed: 800.0,
            state: 0.0,
            check_grab_once: false,
        }
    }

    fn set_rc_target(&mut self, rc_target: &Rc<RefCell<Character>>) {
        self.rc_target = Some(Rc::clone(rc_target));
    }

    fn set_rotation(&mut self, rotation: f32) {
        self.rc_gameobject.borrow_mut().transform.rotation = rotation;
    }

    fn set_position(&mut self, position: Point2<f32>) {
        self.rc_gameobject.borrow_mut().transform.position = position;
    }
}

impl Communication for Grab {
    fn get_send_data(&self) -> Vec<u8> {
        let mut data = Vec::new();
        let sp = self.speed.to_ne_bytes();
        let st = self.state.to_ne_bytes();
        let go = self.rc_gameobject.borrow().get_send_data();

        data.extend_from_slice(&sp);
        data.extend_from_slice(&st);
        data.extend(go);
        data
    }

    fn set_recv_data(&mut self, buf: &mut Vec<u8>) {
        let (data, buf) = buf.split_at(8);
        let sp = f32::from_ne_bytes(data[0..4].try_into().unwrap());
        let st = f32::from_ne_bytes(data[4..8].try_into().unwrap());
        self.speed = sp;
        self.state = st;

        let mut buf = Vec::from(buf);
        self.rc_gameobject.borrow_mut().set_recv_data(&mut buf);
    }
}

impl EventHandler for Grab {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let delta = delta(&ctx);
        let dt = delta.as_secs() as f32 + delta.subsec_nanos() as f32 * 1e-9;
        let speed = dt * self.speed * self.state;
        {
            let mut gameobject = self.rc_gameobject.borrow_mut();
            let delta_vec = gameobject.transform.forward();
            gameobject.transform.position.x += speed * delta_vec.x;
            gameobject.transform.position.y += speed * delta_vec.y;

            gameobject.update_global_transform();

            self.rc_target.as_ref().map(|rc_target| {
                let mut target = rc_target.borrow_mut();
                if self.state == 1.0 && gameobject.transform.position.x > self.threshold {
                    // Check target is in grab range (80.0)
                    if (target.get_global_position().x - gameobject.global_transform.position.x)
                        .abs()
                        < 80.0
                    {
                        self.state = -1.0;
                        target.set_global_rotation(gameobject.global_transform.rotation - PI);
                        target.is_grabbed_by = true;
                    } else {
                        self.state = 0.0;
                    }
                } else if self.state == -1.0 {
                    if gameobject.transform.position.x < 0.0 {
                        self.state = 0.0;
                        self.check_grab_once = true;
                    } else {
                        // target position is same with grab position
                        target.set_global_position(gameobject.global_transform.position);
                    }
                } else if self.state == 0.0 && target.is_grabbed_by {
                    target.rebirth(true);
                }
            });
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        if self.state != 0.0 {
            let gameobject = self.rc_gameobject.borrow();

            let scale_y =
                gameobject.transform.magnitude() / (self.string_image.as_ref().height() as f32);

            let string_draw_param = DrawParam::new()
                .dest(gameobject.global_transform.position)
                .rotation(gameobject.global_transform.rotation - PI / 2.0)
                .offset(Point2 { x: 0.5, y: 1.0 })
                .scale(Point2 { x: 1.0, y: scale_y });
            draw(ctx, self.string_image.as_ref(), string_draw_param)?;

            let hand_draw_param = DrawParam::new()
                .dest(gameobject.global_transform.position)
                .rotation(gameobject.global_transform.rotation - PI / 2.0)
                .offset(Point2 { x: 0.5, y: 0.5 });
            draw(ctx, self.hand_image.as_ref(), hand_draw_param)?;
        }
        Ok(())
    }
}

struct Target {
    image: Rc<Image>,
    rc_gameobject: Rc<RefCell<GameObject>>,
    speed: f32,
    direction: f32,
    look_at_x: f32,
}

impl Target {
    fn new(ctx: &mut Context, image_pool: &mut HashMap<String, Rc<Image>>) -> Target {
        Target {
            image: load_image(ctx, String::from("/target.png"), image_pool),
            rc_gameobject: Rc::new(RefCell::new(GameObject::new())),
            direction: 1.0,
            speed: 800.0,
            look_at_x: 0.0,
        }
    }

    fn get_rotation(&self) -> f32 {
        self.rc_gameobject.borrow().transform.rotation
    }
}

impl Communication for Target {
    fn get_send_data(&self) -> Vec<u8> {
        let mut data = vec![];
        let la = self.look_at_x.to_ne_bytes();

        data.extend_from_slice(&la);
        data
    }

    fn set_recv_data(&mut self, buf: &mut Vec<u8>) {
        let (data, buf) = buf.split_at(4);
        self.look_at_x = f32::from_ne_bytes(data[0..4].try_into().unwrap());
    }
}

impl EventHandler for Target {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let delta = ggez::timer::delta(ctx);
        let dt = delta.as_secs() as f32 + delta.subsec_nanos() as f32 * 1e-9;
        let delta_dist = dt * self.direction * self.speed;

        self.look_at_x += delta_dist;

        if self.look_at_x > 440.0 {
            self.direction = -1.0;
            self.look_at_x = 440.0;
        } else if self.look_at_x < -440.0 {
            self.direction = 1.0;
            self.look_at_x = -440.0;
        }

        {
            let mut gameobject = self.rc_gameobject.borrow_mut();
            if let Some(rc_parent) = gameobject.rc_parent.clone() {
                let parent = rc_parent.borrow();
                let offset = parent.transform.position.y / parent.transform.position.y.abs();
                let dist_x = offset * (self.look_at_x - parent.transform.position.x);
                gameobject.transform.rotation = dist_x.atan2(580.0_f32);
            }
            gameobject.update_global_transform();
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        {
            let gameobject = self.rc_gameobject.borrow();
            let draw_param = DrawParam::new()
                .dest(gameobject.global_transform.position)
                .rotation(gameobject.global_transform.rotation - PI / 2.0)
                .offset(Point2 { x: 0.5, y: 0.0 })
                .color(if self.speed > 0.0 {
                    Color::CYAN
                } else {
                    Color::BLACK
                });
            draw(ctx, self.image.as_ref(), draw_param)?;
        }
        Ok(())
    }
}

struct Flash {
    image: Rc<Image>,
    cooldown_image: Rc<Image>,
    cooltime: f32,
    cooldown: f32,
    distance: f32,
}

impl Flash {
    fn new(ctx: &mut Context, image_pool: &mut HashMap<String, Rc<Image>>) -> Flash {
        Flash {
            image: load_image(ctx, String::from("/flash.png"), image_pool),
            cooldown_image: load_image(ctx, String::from("/cooldown.png"), image_pool),
            cooltime: 5.0,
            cooldown: 0.0,
            distance: 200.0,
        }
    }

    fn use_skill(&mut self, rc_gameobject: Rc<RefCell<GameObject>>, direction: f32) {
        if self.cooldown > 0.0 || direction == 0.0 {
            return;
        }
        self.cooldown = self.cooltime;
        {
            let mut gameobject = rc_gameobject.borrow_mut();
            let direction = gameobject.transform.right().x * direction;
            gameobject
                .transform
                .move_offset_x(direction * self.distance);
        }
    }
}

impl EventHandler for Flash {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let delta = ggez::timer::delta(ctx);
        let dt = delta.as_secs() as f32 + delta.subsec_nanos() as f32 * 1e-9;
        self.cooldown = (self.cooldown - dt).max(0.0);
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        let text_draw_params = DrawParam::new()
            .dest(Point2 { x: 50.0, y: 685.0 })
            .scale([1.4, 1.4])
            .color(Color::YELLOW);
        draw(ctx, &Text::new("Shift"), text_draw_params)?;

        let draw_param = DrawParam::new()
            .dest(Point2 { x: 80.0, y: 640.0 })
            .offset(Point2 { x: 0.5, y: 0.5 })
            .scale(Point2 { x: 0.2, y: 0.2 });
        draw(ctx, self.image.as_ref(), draw_param)?;

        if self.cooldown > 0.0 {
            let cooldown_rect_draw_params = DrawParam::new()
                .dest(Point2 { x: 80.0, y: 605.0 })
                .offset(Point2 { x: 0.5, y: 0.0 })
                .scale([0.32, 0.32])
                .color(Color::from_rgba(0, 0, 0, 200));
            draw(ctx, self.cooldown_image.as_ref(), cooldown_rect_draw_params)?;

            let cooldown_text_draw_params = DrawParam::new()
                .dest(Point2 { x: 50.0, y: 620.0 })
                .scale([2.4, 2.4])
                .color(Color::WHITE);
            draw(
                ctx,
                &Text::new(format!("{:.1}", self.cooldown)),
                cooldown_text_draw_params,
            )?;
        }
        Ok(())
    }
}

struct Character {
    default_image: Rc<Image>,
    motion_image: Rc<Image>,
    rc_gameobject: Rc<RefCell<GameObject>>,
    is_server: bool,
    move_state: f32,
    move_speed: f32,
    score: i32,
    is_opponent: bool,

    target: Target,
    grab: Grab,
    is_grabbed_by: bool,
    flash: Flash,
}

impl Character {
    fn new(
        ctx: &mut Context,
        image_pool: &mut HashMap<String, Rc<Image>>,
        is_server: bool,
    ) -> Character {
        let rc_gameobject = Rc::new(RefCell::new(GameObject::new()));
        let target = Target::new(ctx, image_pool);
        {
            target
                .rc_gameobject
                .borrow_mut()
                .set_rc_parent(&rc_gameobject);
        }

        let grab = Grab::new(ctx, image_pool);
        {
            grab.rc_gameobject
                .borrow_mut()
                .set_rc_parent(&rc_gameobject);
        }
        Character {
            default_image: load_image(ctx, String::from("/player.png"), image_pool),
            motion_image: load_image(ctx, String::from("/player_grab.png"), image_pool),
            rc_gameobject,
            is_server,
            move_state: 0.0,
            move_speed: 300.0,
            score: 0,
            is_opponent: false,

            target,
            grab,
            is_grabbed_by: false,
            flash: Flash::new(ctx, image_pool),
        }
    }

    fn rebirth(&mut self, randomize: bool) {
        {
            let mut gameobject = self.rc_gameobject.borrow_mut();
            let mut rng = rand::thread_rng();
            gameobject.transform.position = Point2 {
                x: if randomize {
                    rng.gen_range(-340.0..340.0)
                } else {
                    0.0
                },
                y: if self.is_server ^ self.is_opponent {
                    -290.0
                } else {
                    290.0
                },
            };
            gameobject.transform.rotation = if self.is_server ^ self.is_opponent {
                PI / 2.0
            } else {
                -PI / 2.0
            };
            gameobject.update_global_transform();
        }
        self.is_grabbed_by = false;
    }

    fn set_global_rotation(&self, rotation: f32) {
        self.rc_gameobject.borrow_mut().global_transform.rotation = rotation;
        self.rc_gameobject.borrow_mut().update_local_transform();
    }

    fn set_global_position(&self, position: Point2<f32>) {
        self.rc_gameobject.borrow_mut().update_local_transform();
        self.rc_gameobject.borrow_mut().global_transform.position = position;
    }

    fn get_global_position(&self) -> Point2<f32> {
        self.rc_gameobject.borrow().global_transform.position
    }
}

impl Communication for Character {
    fn get_send_data(&self) -> Vec<u8> {
        let mut data = vec![];
        let ms = self.move_state.to_ne_bytes();
        let sc = self.score.to_ne_bytes();
        let go = self.rc_gameobject.borrow().get_send_data();

        data.extend_from_slice(&ms);
        data.extend_from_slice(&sc);
        data.extend(go);

        data.extend(self.target.get_send_data());
        data.extend(self.grab.get_send_data());

        data
    }

    fn set_recv_data(&mut self, buf: &mut Vec<u8>) {
        let (data, buf) = buf.split_at(8);
        self.move_state = f32::from_ne_bytes(data[0..4].try_into().unwrap());
        self.score = i32::from_ne_bytes(data[4..8].try_into().unwrap());

        let (data, buf) = buf.split_at(20);
        let mut data = Vec::from(data);
        self.rc_gameobject.borrow_mut().set_recv_data(&mut data);

        let (data, buf) = buf.split_at(4);
        let mut data = Vec::from(data);
        self.target.set_recv_data(&mut data);

        let (data, buf) = buf.split_at(28);
        let mut data = Vec::from(data);
        self.grab.set_recv_data(&mut data);
    }
}

impl EventHandler for Character {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        if !self.is_grabbed_by {
            let delta = ggez::timer::delta(ctx);
            let dt = delta.as_secs() as f32 + delta.subsec_nanos() as f32 * 1e-9;
            let speed = dt * self.move_state * self.move_speed * (1.0 - self.grab.state.abs());
            self.target.speed = 800.0 * (1.0 - self.grab.state.abs());
            {
                let mut gameobject = self.rc_gameobject.borrow_mut();

                let delta_vec = gameobject.transform.right();
                gameobject.transform.position.x += speed * delta_vec.x;
                gameobject.transform.position.x =
                    gameobject.transform.position.x.min(340.0).max(-340.0);
                gameobject.update_global_transform();
            }

            if self.grab.check_grab_once {
                self.score += 1;
                self.grab.check_grab_once = false;
            }

            self.target.update(ctx)?;
            self.grab.update(ctx)?;
        } else {
            self.grab.state = 0.0;
            self.move_state = 0.0;
        }
        self.flash.update(ctx)?;

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        if !self.is_grabbed_by {
            self.target.draw(ctx)?;
            self.grab.draw(ctx)?;
        }
        {
            let gameobject = self.rc_gameobject.borrow();
            let (draw_image, rotation) = if self.grab.state == 0.0 {
                (
                    self.default_image.clone(),
                    gameobject.global_transform.rotation - (2.0 - self.move_state) * PI / 4.0,
                )
            } else {
                let target = self.target.rc_gameobject.borrow();
                (
                    self.motion_image.clone(),
                    target.global_transform.rotation - PI / 2.0,
                )
            };
            let draw_param = DrawParam::new()
                .dest(gameobject.global_transform.position)
                .rotation(rotation)
                .offset(Point2 { x: 0.5, y: 0.5 });
            draw(ctx, draw_image.as_ref(), draw_param)?;
        }
        if !self.is_opponent {
            self.flash.draw(ctx)?;
        }
        Ok(())
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        keycode: KeyCode,
        keymods: KeyMods,
        repeat: bool,
    ) {
        if keycode == KeyCode::Space && self.grab.state == 0.0 {
            self.grab.state = 1.0;

            self.grab.set_position(Transform::rotate_point(
                Point2 { x: 35.0, y: 60.0 },
                self.target.get_rotation(),
            ));
            self.grab.set_rotation(self.target.get_rotation());
        }

        if ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Left) {
            self.move_state = -1.0
        }
        if ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Right) {
            self.move_state = 1.0
        }

        if keycode == KeyCode::LShift && self.grab.state == 0.0 {
            self.flash
                .use_skill(self.rc_gameobject.clone(), self.move_state);
        }
    }

    fn key_up_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymods: KeyMods) {
        self.move_state = 0.0;
        if is_key_pressed(ctx, KeyCode::Left) {
            self.move_state = -1.0
        }
        if is_key_pressed(ctx, KeyCode::Right) {
            self.move_state = 1.0
        }
    }
}

pub struct GameState {
    background_image: Rc<Image>,
    foreground_image: Rc<Image>,
    rc_player: Rc<RefCell<Character>>,
    rc_opponent: Rc<RefCell<Character>>,
    rc_global: Rc<RefCell<GameObject>>,
    stream: Option<Rc<RefCell<TcpStream>>>,
    is_game_end: bool,
    last_recv: f32,
}

impl GameState {
    pub fn initialize(&mut self, is_server: bool, tcp_stream: &Option<Rc<RefCell<TcpStream>>>) {
        if let Some(i) = tcp_stream {
            self.stream = Some(Rc::clone(&i));
        }

        {
            let mut global = self.rc_global.borrow_mut();
            global.transform.position = Point2 { x: 640.0, y: 360.0 };
            if is_server {
                global.transform.rotation = PI;
            }
            global.update_global_transform();
        }

        {
            let mut player = self.rc_player.borrow_mut();
            let mut opponent = self.rc_opponent.borrow_mut();

            player.is_server = is_server;
            opponent.is_server = is_server;

            player.grab.set_rc_target(&self.rc_opponent);
            opponent.grab.set_rc_target(&self.rc_player);
            if is_server {
                opponent.target.direction = -1.0;
            } else {
                player.target.direction = -1.0;
            }
            opponent.is_opponent = true;
            {
                let mut playerobject = player.rc_gameobject.borrow_mut();
                playerobject.set_rc_parent(&self.rc_global);

                let mut oppoentobject = opponent.rc_gameobject.borrow_mut();
                oppoentobject.set_rc_parent(&self.rc_global);
            }
            player.rebirth(false);
            opponent.rebirth(false);
        }
    }

    // Game Setting
    pub fn new(ctx: &mut Context, image_pool: &mut HashMap<String, Rc<Image>>) -> GameState {
        let rc_global = Rc::new(RefCell::new(GameObject::new()));

        let background_image = load_image(ctx, String::from("/background.png"), image_pool);
        let foreground_image = load_image(ctx, String::from("/foreground.png"), image_pool);

        let rc_player = Rc::new(RefCell::new(Character::new(ctx, image_pool, false)));
        let rc_opponent = Rc::new(RefCell::new(Character::new(ctx, image_pool, false)));

        GameState {
            background_image,
            foreground_image,
            rc_player,
            rc_opponent,
            rc_global,
            stream: None,
            is_game_end: false,
            last_recv: 0.0,
        }
    }

    fn send_data(&mut self, ctx: &mut Context) {
        let mut data = vec![];
        let sst = ggez::timer::time_since_start(ctx)
            .as_secs_f32()
            .to_ne_bytes();
        data.extend_from_slice(&sst);
        data.extend(self.rc_player.borrow_mut().get_send_data()); // 24 bytes
        if let Some(st) = &self.stream {
            let mut _st = st.try_borrow_mut().unwrap();
            _st.write(data.as_slice()).unwrap();
            _st.flush().unwrap();
        }
    }

    fn recv_data(&mut self) {
        let mut buf = [0u8; 64];
        if let Some(st) = &self.stream {
            let mut _st = st.try_borrow_mut().unwrap();

            match _st.read_exact(&mut buf) {
                Ok(_) => {
                    let (data, buf) = buf.split_at(4);
                    let recv_time = f32::from_ne_bytes(data[0..4].try_into().unwrap());
                    if recv_time > self.last_recv {
                        self.rc_opponent
                            .borrow_mut()
                            .set_recv_data(&mut Vec::from(buf));
                        self.last_recv = recv_time;
                    }
                }
                Err(err) => {}
            }
        }
    }
}

impl IState for GameState {
    fn update(&mut self, _ctx: &mut ggez::Context) -> EState {
        self.send_data(_ctx);
        self.recv_data();
        if self.is_game_end {
            return EState::None;
        }

        self.rc_player
            .borrow_mut()
            .update(_ctx)
            .expect("player update failed");
        self.rc_opponent
            .borrow_mut()
            .update(_ctx)
            .expect("opponent update failed");

        self.is_game_end =
            self.rc_player.borrow().score == 3 || self.rc_opponent.borrow().score == 3;

        EState::None
    }

    fn draw(&mut self, ctx: &mut ggez::Context) {
        clear(ctx, Color::WHITE);
        draw(ctx, self.background_image.as_ref(), DrawParam::new()).expect("draw failed");
        draw(ctx, self.foreground_image.as_ref(), DrawParam::new()).expect("draw failed");

        // Draw Score
        let opponent_score_draw_params = DrawParam::new()
            .dest(Point2 { x: 640.0, y: 220.0 })
            .offset(Point2 { x: 0.5, y: 0.5 })
            .scale([2.0, 2.0])
            .color(Color::WHITE);
        draw(
            ctx,
            &Text::new(format!("{}", self.rc_opponent.borrow().score)),
            opponent_score_draw_params,
        )
        .expect("draw failed");

        let player_score_draw_params = DrawParam::new()
            .dest(Point2 { x: 640.0, y: 450.0 })
            .offset(Point2 { x: 0.5, y: 0.5 })
            .scale([3.0, 3.0])
            .color(Color::WHITE);
        draw(
            ctx,
            &Text::new(format!("{}", self.rc_player.borrow().score)),
            player_score_draw_params,
        )
        .expect("draw failed");

        if self.is_game_end {
            let game_end_draw_params = DrawParam::new()
                .dest(Point2 { x: 640.0, y: 260.0 })
                .offset(Point2 { x: 0.5, y: 0.5 })
                .scale([5.0, 5.0])
                .color(Color::WHITE);
            if self.rc_player.borrow().score == 3 {
                draw(ctx, &Text::new("You Win!\nPress ESC"), game_end_draw_params)
                    .expect("draw failed");
            } else {
                draw(
                    ctx,
                    &Text::new("Opponent Win!\nPress ESC"),
                    game_end_draw_params,
                )
                .expect("draw failed");
            }
        } else {
            self.rc_player.borrow_mut().draw(ctx).expect("draw failed");
            self.rc_opponent
                .borrow_mut()
                .draw(ctx)
                .expect("draw failed");
        }

        // don't have to do this here. it makes flickering.
        // present(ctx).expect("draw failed");
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        keycode: KeyCode,
        keymods: KeyMods,
        repeat: bool,
    ) {
        if ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Escape) {
            ggez::event::quit(ctx);
        }
        self.rc_player
            .borrow_mut()
            .key_down_event(ctx, keycode, keymods, repeat);
    }

    fn key_up_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymods: KeyMods) {
        self.rc_player
            .borrow_mut()
            .key_up_event(ctx, keycode, keymods);
    }
}
