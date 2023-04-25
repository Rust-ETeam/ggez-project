use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::quit;
use ggez::event::{run, EventHandler, KeyCode, KeyMods};
use ggez::graphics::{clear, draw, present, Color, DrawParam, Image, Text};
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
        let radian = point.y.atan2(point.y) + rotation;
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
            let magnitude = (scaled_pos_x.powf(2.0) + scaled_pos_y.powf(2.0)).sqrt();
            let radian = scaled_pos_y.atan2(scaled_pos_x);
            self.global_transform.position.x = parent.global_transform.position.x
                + (parent.global_transform.rotation + radian).cos() * magnitude;
            self.global_transform.position.y = parent.global_transform.position.y
                + (parent.global_transform.rotation + radian).sin() * magnitude;
        } else {
            self.global_transform = self.transform;
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

            if self.state == 1.0 && gameobject.transform.position.x > self.threshold {
                self.rc_target.as_ref().map(|rc_target| {
                    let mut target = rc_target.borrow_mut();
                    // Check target is in grab range (50.0)
                    if (target.get_global_position().x - gameobject.global_transform.position.x)
                        .abs()
                        < 50.0
                    {
                        self.state = -1.0;
                        self.check_grab_once = true;
                        target.is_grabbed_by = true;
                    } else {
                        self.state = 0.0;
                    }
                });
            } else if self.state == -1.0 {
                self.rc_target.as_ref().map(|rc_target| {
                    // check grab is fully pulled
                    if gameobject.transform.position.x < 0.0 {
                        self.state = 0.0;
                        let mut target = rc_target.borrow_mut();
                        target.rebirth()
                    } else {
                        // target position is same with grab position
                        rc_target
                            .borrow_mut()
                            .set_global_position(gameobject.global_transform.position);
                    }
                });
            }
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
            look_at_x: 640.0,
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

        if self.look_at_x > 1080.0 {
            self.direction = -1.0;
            self.look_at_x = 1080.0;
        } else if self.look_at_x < 200.0 {
            self.direction = 1.0;
            self.look_at_x = 200.0;
        }

        {
            let mut gameobject = self.rc_gameobject.borrow_mut();
            if let Some(rc_parent) = gameobject.rc_parent.clone() {
                let parent = rc_parent.borrow();
                let dist_x = self.look_at_x - parent.transform.position.x;
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
}

impl Character {
    fn new(ctx: &mut Context, image_pool: &mut HashMap<String, Rc<Image>>) -> Character {
        let rc_gameobject = Rc::new(RefCell::new(GameObject::new()));
        let mut target = Target::new(ctx, image_pool);
        {
            target
                .rc_gameobject
                .borrow_mut()
                .set_rc_parent(&rc_gameobject);
        }

        let mut grab = Grab::new(ctx, image_pool);
        {
            grab.rc_gameobject
                .borrow_mut()
                .set_rc_parent(&rc_gameobject);
        }
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
        }
    }

    fn rebirth(&mut self) {
        {
            let mut gameobject = self.rc_gameobject.borrow_mut();
            let mut rng = rand::thread_rng();
            gameobject.transform.position = Point2 {
                x: rng.gen_range(300.0..980.0),
                y: if self.is_opponent { 70.0 } else { 650.0 },
            }
        }
        self.is_grabbed_by = false;
    }

    fn set_global_position(&self, position: Point2<f32>) {
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
            let speed = dt * self.move_state * self.move_speed * (1.0 - self.grab.state.abs());
            self.target.speed = 800.0 * (1.0 - self.grab.state.abs());
            {
                let mut gameobject = self.rc_gameobject.borrow_mut();
                let delta_vec = gameobject.transform.right();
                gameobject.transform.position.x += speed * delta_vec.x;
                gameobject.transform.position.x =
                    gameobject.transform.position.x.min(980.0).max(300.0);
                gameobject.update_global_transform();
            }

            if self.grab.check_grab_once {
                self.score += 1;
                self.grab.check_grab_once = false;
            }

            self.target.update(ctx)?;
            self.grab.update(ctx)?;
        }
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
        Ok(())
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        keycode: KeyCode,
        keymods: KeyMods,
        repeat: bool,
    ) {
        // if keycode == KeyCode::Space && self.player_grab_state == 0 {
        if keycode == KeyCode::Space {
            self.grab.state = 1.0;

            self.grab.set_position(Transform::rotate_point(
                Point2 { x: 35.0, y: 60.0 },
                self.target.get_rotation(),
            ));
            self.grab.set_rotation(self.target.get_rotation());
        }

        self.move_state = 0.0;
        if ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Left) {
            self.move_state = -1.0
        }
        if ggez::input::keyboard::is_key_pressed(ctx, KeyCode::Right) {
            self.move_state = 1.0
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

struct GGEZ {
    background_image: Rc<Image>,
    foreground_image: Rc<Image>,
    rc_player: Rc<RefCell<Character>>,
    rc_opponent: Rc<RefCell<Character>>,
}

impl GGEZ {
    // Game Setting
    fn new(ctx: &mut Context, image_pool: &mut HashMap<String, Rc<Image>>) -> GGEZ {
        let background_image = load_image(ctx, String::from("/background.png"), image_pool);
        let foreground_image = load_image(ctx, String::from("/foreground.png"), image_pool);

        let rc_player = Rc::new(RefCell::new(Character::new(ctx, image_pool)));
        let rc_opponent = Rc::new(RefCell::new(Character::new(ctx, image_pool)));
        {
            let mut player = rc_player.borrow_mut();
            let mut opponent = rc_opponent.borrow_mut();
            player.grab.set_rc_target(&rc_opponent);
            opponent.grab.set_rc_target(&rc_player);
            opponent.is_opponent = true;

            let mut playerobject = player.rc_gameobject.borrow_mut();
            playerobject.transform.position = Point2 { x: 640.0, y: 650.0 };
            playerobject.transform.rotation = -PI / 2.0;

            let mut oppoentobject = opponent.rc_gameobject.borrow_mut();
            oppoentobject.transform.position = Point2 { x: 640.0, y: 70.0 };
            oppoentobject.transform.rotation = PI / 2.0;
        }

        GGEZ {
            background_image,
            foreground_image,
            rc_player,
            rc_opponent,
        }
    }
}

impl EventHandler for GGEZ {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        self.rc_player.borrow_mut().update(ctx)?;
        self.rc_opponent.borrow_mut().update(ctx)?;

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        clear(ctx, Color::WHITE);
        draw(ctx, self.background_image.as_ref(), DrawParam::new())?;
        draw(ctx, self.foreground_image.as_ref(), DrawParam::new())?;

        self.rc_player.borrow_mut().draw(ctx)?;
        self.rc_opponent.borrow_mut().draw(ctx)?;

        present(ctx)?;
        Ok(())
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
        // self.opponent.key_down_event(ctx, keycode, keymods, repeat);
    }

    fn key_up_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymods: KeyMods) {
        self.rc_player
            .borrow_mut()
            .key_up_event(ctx, keycode, keymods);
        // self.opponent.key_up_event(ctx, keycode, keymods);
    }
}

fn load_image(
    ctx: &mut Context,
    path: String,
    image_pool: &mut HashMap<String, Rc<Image>>,
) -> Rc<Image> {
    match unsafe { image_pool.get(&path) } {
        Some(image) => Rc::clone(image),
        None => match Image::new(ctx, path.clone()) {
            Ok(res) => {
                let image = Rc::new(res);
                unsafe {
                    image_pool.insert(path.clone(), Rc::clone(&image));
                }
                image
            }
            Err(err) => panic!("Failed to load image: {}", err),
        },
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

    let ggez = GGEZ::new(&mut ctx, &mut image_pool);
    run(ctx, event_loop, ggez);
}
