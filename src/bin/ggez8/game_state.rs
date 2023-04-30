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
use std::io::{BufWriter, Read};
use std::io::Write;

use crate::helper::{load_image, EState, IState};

use std::fs::File;
use std::io::{BufReader, BufRead};
use std::ptr::write;
use ggez::winit::event::DeviceEvent::Key;

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
    fx_image0: Rc<Image>,
    fx_image1: Rc<Image>,
    fx_image2: Rc<Image>,
    fx_image3: Rc<Image>,
    fx_image4: Rc<Image>,
    cooldown_image: Rc<Image>,
    cooltime: f32,
    cooldown: f32,
    distance: f32,
    position: Point2<f32>,
    is_opponent: bool,
}

impl Flash {
    fn new(ctx: &mut Context, image_pool: &mut HashMap<String, Rc<Image>>) -> Flash {
        Flash {
            image: load_image(ctx, String::from("/flash.png"), image_pool),
            fx_image0: load_image(ctx, String::from("/flash0.png"), image_pool),
            fx_image1: load_image(ctx, String::from("/flash1.png"), image_pool),
            fx_image2: load_image(ctx, String::from("/flash2.png"), image_pool),
            fx_image3: load_image(ctx, String::from("/flash3.png"), image_pool),
            fx_image4: load_image(ctx, String::from("/flash4.png"), image_pool),
            cooldown_image: load_image(ctx, String::from("/cooldown.png"), image_pool),
            cooltime: 5.0,
            cooldown: 0.0,
            distance: 200.0,
            position: Point2 {x: 0.0, y: 0.0},
            is_opponent: false,
        }
    }

    fn use_skill(&mut self, rc_gameobject: Rc<RefCell<GameObject>>, direction: f32) {
        if self.cooldown > 0.0 || direction == 0.0 {
            return;
        }
        self.cooldown = self.cooltime;
        {
            let mut gameobject = rc_gameobject.borrow_mut();
            self.position = gameobject.global_transform.position;
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
        let fx_draw_params = DrawParam::new()
            .dest(Point2 {x: self.position.x - 50.0, y: self.position.y - 50.0});
        if self.cooltime - self.cooldown < 0.05 {
            draw(ctx, self.fx_image0.as_ref(), fx_draw_params)?;
        }
        else if self.cooltime - self.cooldown < 0.1 {
            draw(ctx, self.fx_image1.as_ref(), fx_draw_params)?;
        }
        else if self.cooltime - self.cooldown < 0.15 {
            draw(ctx, self.fx_image2.as_ref(), fx_draw_params)?;
        }
        else if self.cooltime - self.cooldown < 0.2 {
            draw(ctx, self.fx_image3.as_ref(), fx_draw_params)?;
        }
        else if self.cooltime - self.cooldown < 0.25 {
            draw(ctx, self.fx_image4.as_ref(), fx_draw_params)?;
        }

        if self.is_opponent {
            let text_draw_params = DrawParam::new()
                .dest(Point2 { x: 1150.0, y: 135.0 })
                .scale([1.4, 1.4])
                .color(Color::YELLOW);
            draw(ctx, &Text::new("Shift"), text_draw_params)?;

            let draw_param = DrawParam::new()
                .dest(Point2 { x: 1180.0, y: 90.0 })
                .offset(Point2 { x: 0.5, y: 0.5 })
                .scale(Point2 { x: 0.2, y: 0.2 });
            draw(ctx, self.image.as_ref(), draw_param)?;

            if self.cooldown > 0.0 {
                let cooldown_rect_draw_params = DrawParam::new()
                    .dest(Point2 { x: 1180.0, y: 55.0 })
                    .offset(Point2 { x: 0.5, y: 0.0 })
                    .scale([0.32, 0.32])
                    .color(Color::from_rgba(0, 0, 0, 200));
                draw(ctx, self.cooldown_image.as_ref(), cooldown_rect_draw_params)?;

                let cooldown_text_draw_params = DrawParam::new()
                    .dest(Point2 { x: 1150.0, y: 70.0 })
                    .scale([2.4, 2.4])
                    .color(Color::WHITE);
                draw(
                    ctx,
                    &Text::new(format!("{:.1}", self.cooldown)),
                    cooldown_text_draw_params,
                )?;
            }
        }
        else {
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
        }


        Ok(())
    }
}

struct Character {
    default_image: Rc<Image>,
    motion_image: Rc<Image>,
    rc_gameobject: Rc<RefCell<GameObject>>,
    move_state: f32,
    move_speed: f32,
    score: i32,
    is_opponent: bool,

    target: Target,
    grab: Grab,
    is_grabbed_by: bool,
    flash: Flash,

    replay: bool,
    replay_dt: Vec<f32>,
    replay_act: Vec<i32>,
    replay_idx: usize,
    replay_left_press: bool,
    replay_right_press: bool,

    record: bool,
    record_player_buffer: BufWriter<File>,
    record_opponent_buffer: BufWriter<File>,

    total_dt: f32,
    first_dt: f32,
}

impl Character {
    fn new(
        ctx: &mut Context,
        image_pool: &mut HashMap<String, Rc<Image>>,
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

        let player_record_file = File::create("player_record.txt").unwrap();
        let opponent_record_file = File::create("opponent_record.txt").unwrap();

        let mut player_writer = BufWriter::new(player_record_file);
        let mut opponent_writer = BufWriter::new(opponent_record_file);

        Character {
            default_image: load_image(ctx, String::from("/player.png"), image_pool),
            motion_image: load_image(ctx, String::from("/player_grab.png"), image_pool),
            rc_gameobject,
            move_state: 0.0,
            move_speed: 300.0,
            score: 0,
            is_opponent: false,

            target,
            grab,
            is_grabbed_by: false,
            flash: Flash::new(ctx, image_pool),

            replay: false,
            replay_dt: vec![],
            replay_act: vec![],
            replay_idx: 0,
            replay_left_press: false,
            replay_right_press: false,

            record: false,
            record_player_buffer: player_writer,
            record_opponent_buffer: opponent_writer,

            total_dt: 0.0,
            first_dt: 0.0,
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
                y: if self.is_opponent {
                    -290.0
                } else {
                    290.0
                },
            };
            gameobject.transform.rotation = if self.is_opponent {
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

impl EventHandler for Character {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        if !self.is_grabbed_by {
            let delta = ggez::timer::delta(ctx);
            let dt = delta.as_secs() as f32 + delta.subsec_nanos() as f32 * 1e-9;

            if self.total_dt < 0.01 {
                self.first_dt = dt;
            }

            self.total_dt += dt;
            if self.replay {
                while self.replay_idx < self.replay_dt.len() && self.replay_dt[self.replay_idx] <= self.total_dt - self.first_dt {
                    if self.replay_act[self.replay_idx] == 1 {
                        self.key_down_event(ctx, KeyCode::Left, KeyMods::empty(), false);
                    } else if self.replay_act[self.replay_idx] == 2 {
                        self.key_down_event(ctx, KeyCode::Right, KeyMods::empty(), false);
                    } else if self.replay_act[self.replay_idx] == 3 {
                        self.key_down_event(ctx, KeyCode::Space, KeyMods::empty(), false);
                    } else if self.replay_act[self.replay_idx] == 4 {
                        self.key_down_event(ctx, KeyCode::LShift, KeyMods::empty(), false);
                    } else if self.replay_act[self.replay_idx] == -1 {
                        self.key_up_event(ctx, KeyCode::Left, KeyMods::empty());
                    } else if self.replay_act[self.replay_idx] == -2 {
                        self.key_up_event(ctx, KeyCode::Right, KeyMods::empty());
                    }
                    self.replay_idx += 1;
                }
            }

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
        self.flash.draw(ctx)?;
        Ok(())
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        keycode: KeyCode,
        keymods: KeyMods,
        repeat: bool,
    ) {
        if self.record {
            if self.is_opponent {
                if keycode == KeyCode::Left {
                    write!(self.record_opponent_buffer, "{} {}\n", self.total_dt - self.first_dt, 1).unwrap();
                    self.record_opponent_buffer.flush().unwrap();
                }
                else if keycode == KeyCode::Right {
                    write!(self.record_opponent_buffer, "{} {}\n", self.total_dt - self.first_dt, 2).unwrap();
                    self.record_opponent_buffer.flush().unwrap();
                }
                else if keycode == KeyCode::Space {
                    write!(self.record_opponent_buffer, "{} {}\n", self.total_dt - self.first_dt, 3).unwrap();
                    self.record_opponent_buffer.flush().unwrap();
                }
                else if keycode == KeyCode::LShift {
                    write!(self.record_opponent_buffer, "{} {}\n", self.total_dt - self.first_dt, 4).unwrap();
                    self.record_opponent_buffer.flush().unwrap();
                }
            }
            else {
                if keycode == KeyCode::Left {
                    write!(self.record_player_buffer, "{} {}\n", self.total_dt - self.first_dt, 1).unwrap();
                    self.record_player_buffer.flush().unwrap();
                }
                else if keycode == KeyCode::Right {
                    write!(self.record_player_buffer, "{} {}\n", self.total_dt - self.first_dt, 2).unwrap();
                    self.record_player_buffer.flush().unwrap();
                }
                else if keycode == KeyCode::Space {
                    write!(self.record_player_buffer, "{} {}\n", self.total_dt - self.first_dt, 3).unwrap();
                    self.record_player_buffer.flush().unwrap();
                }
                else if keycode == KeyCode::LShift {
                    write!(self.record_player_buffer, "{} {}\n", self.total_dt - self.first_dt, 4).unwrap();
                    self.record_player_buffer.flush().unwrap();
                }
            }
        }

        if keycode == KeyCode::Space && self.grab.state == 0.0 {
            self.grab.state = 1.0;

            self.grab.set_position(Transform::rotate_point(
                Point2 { x: 35.0, y: 60.0 },
                self.target.get_rotation(),
            ));
            self.grab.set_rotation(self.target.get_rotation());
        }

        if self.replay {
            if keycode == KeyCode::Left {
                self.replay_left_press = true;
            }
            if keycode == KeyCode::Right {
                self.replay_right_press = true;
            }

            if self.replay_left_press {
                self.move_state = -1.0
            }
            if self.replay_right_press {
                self.move_state = 1.0
            }
        }
        else {
            if ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Left) {
                self.move_state = -1.0
            }
            if ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Right) {
                self.move_state = 1.0
            }
        }

        if keycode == KeyCode::LShift {
            self.flash
                .use_skill(self.rc_gameobject.clone(), self.move_state);
        }
    }

    fn key_up_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymods: KeyMods) {
        if self.record {
            if self.is_opponent {
                if keycode == KeyCode::Left {
                    write!(self.record_opponent_buffer, "{} {}\n", self.total_dt - self.first_dt, -1).unwrap();
                    self.record_opponent_buffer.flush().unwrap();
                }
                else if keycode == KeyCode::Right {
                    write!(self.record_opponent_buffer, "{} {}\n", self.total_dt - self.first_dt, -2).unwrap();
                    self.record_opponent_buffer.flush().unwrap();
                }
            }
            else {
                if keycode == KeyCode::Left {
                    write!(self.record_player_buffer, "{} {}\n", self.total_dt - self.first_dt, -1).unwrap();
                    self.record_player_buffer.flush().unwrap();
                }
                else if keycode == KeyCode::Right {
                    write!(self.record_player_buffer, "{} {}\n", self.total_dt - self.first_dt, -2).unwrap();
                    self.record_player_buffer.flush().unwrap();
                }
            }
        }

        self.move_state = 0.0;
        if self.replay {
            if keycode == KeyCode::Left {
                self.replay_left_press = false;
            }
            if keycode == KeyCode::Right {
                self.replay_right_press = false;
            }

            if self.replay_left_press {
                self.move_state = -1.0
            }
            if self.replay_right_press {
                self.move_state = 1.0
            }
        }
        else {
            if ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Left) {
                self.move_state = -1.0
            }
            if ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Right) {
                self.move_state = 1.0
            }
        }
    }
}

pub struct GameState {
    background_image: Rc<Image>,
    foreground_image: Rc<Image>,
    rc_player: Rc<RefCell<Character>>,
    rc_opponent: Rc<RefCell<Character>>,
    rc_global: Rc<RefCell<GameObject>>,
    is_game_end: bool,
}

impl GameState {
    pub fn initialize(&mut self) {
        {
            let mut global = self.rc_global.borrow_mut();
            global.transform.position = Point2 { x: 640.0, y: 360.0 };
            // global.transform.rotation = PI;
            global.update_global_transform();
        }

        {
            let mut player = self.rc_player.borrow_mut();
            let mut opponent = self.rc_opponent.borrow_mut();

            player.grab.set_rc_target(&self.rc_opponent);
            opponent.grab.set_rc_target(&self.rc_player);
            opponent.target.direction = -1.0;
            opponent.is_opponent = true;
            opponent.flash.is_opponent = true;
            {
                let mut playerobject = player.rc_gameobject.borrow_mut();
                playerobject.set_rc_parent(&self.rc_global);

                let mut oppoentobject = opponent.rc_gameobject.borrow_mut();
                oppoentobject.set_rc_parent(&self.rc_global);
            }

            let player_file = File::open("player.txt").unwrap();
            let player_reader = BufReader::new(player_file);
            let mut player_replay_dt: Vec<f32> = Vec::new();
            let mut player_replay_act: Vec<i32> = Vec::new();
            for line in player_reader.lines() {
                let line = line.unwrap();
                let tokens: Vec<&str> = line.split_whitespace().collect();
                let dt = tokens[0].parse().unwrap();
                let act = tokens[1].parse().unwrap();
                player_replay_dt.push(dt);
                player_replay_act.push(act);
            }
            // player.replay = true;
            player.replay_dt = player_replay_dt;
            player.replay_act = player_replay_act;

            let opponent_file = File::open("opponent.txt").unwrap();
            let opponent_reader = BufReader::new(opponent_file);
            let mut opponent_replay_dt: Vec<f32> = Vec::new();
            let mut opponent_replay_act: Vec<i32> = Vec::new();
            for line in opponent_reader.lines() {
                let line = line.unwrap();
                let tokens: Vec<&str> = line.split_whitespace().collect();
                let dt = tokens[0].parse().unwrap();
                let act = tokens[1].parse().unwrap();
                opponent_replay_dt.push(dt);
                opponent_replay_act.push(act);
            }
            opponent.replay = true;
            opponent.replay_dt = opponent_replay_dt;
            opponent.replay_act = opponent_replay_act;

            player.record = true;

            player.rebirth(false);
            opponent.rebirth(false);
        }
    }

    // Game Setting
    pub fn new(ctx: &mut Context, image_pool: &mut HashMap<String, Rc<Image>>) -> GameState {
        let rc_global = Rc::new(RefCell::new(GameObject::new()));

        let background_image = load_image(ctx, String::from("/background.png"), image_pool);
        let foreground_image = load_image(ctx, String::from("/foreground.png"), image_pool);

        let rc_player = Rc::new(RefCell::new(Character::new(ctx, image_pool)));
        let rc_opponent = Rc::new(RefCell::new(Character::new(ctx, image_pool)));

        GameState {
            background_image,
            foreground_image,
            rc_player,
            rc_opponent,
            rc_global,
            is_game_end: false,
        }
    }
}

impl IState for GameState {
    fn update(&mut self, _ctx: &mut ggez::Context) -> EState {
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
