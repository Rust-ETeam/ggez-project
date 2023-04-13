use ggez::{Context, ContextBuilder, GameResult};
use ggez::conf::{WindowSetup, WindowMode};
use ggez::graphics::{Image, Color, DrawParam, draw, clear, present};
use ggez::event::{run, EventHandler, KeyCode, KeyMods};
use ggez::input::keyboard::is_key_pressed;
use ggez::timer::delta;
use ggez::event::quit;
use ggez::mint::Point2;

use std::f32::consts::PI;

use rand::{Rng, thread_rng};
use rand::rngs::ThreadRng;

use std::net::{TcpListener, TcpStream};
use std::io::Write;
use std::io::Read;


struct GGEZ {
    background_image: Image,
    foreground_image: Image,
    target_image: Image,
    player_image: Image,
    player_grab_image: Image,
    grab_string_image: Image,
    grab_hand_image: Image,

    player_x: f32,
    player_target_x: f32,
    player_target_direction: i32,
    player_grab_time: f32,
    player_grab_max_time: f32,
    player_grab_distance: f32,
    player_grab_target_radian: f32,
    player_grab_string_position: Point2<f32>,
    player_move_state: i32,
    player_grab_state: i32,

    opponent_x: f32,
    opponent_target_x: f32,
    opponent_target_direction: i32,
    opponent_grab_time: f32,
    opponent_grab_max_time: f32,
    opponent_grab_distance: f32,
    opponent_grab_target_radian: f32,
    opponent_grab_string_position: Point2<f32>,
    opponent_move_state: i32,
    opponent_grab_state: i32,

    player_speed: f32,
    target_speed: f32,
    grab_speed: f32,
    rng: ThreadRng,
    stream: TcpStream,
}

impl GGEZ {
    fn send_player_data(&mut self) {    
        let x = &self.player_x.to_ne_bytes();
        let target_x = &self.player_target_x.to_ne_bytes();
        let target_direction = &self.player_target_direction.to_ne_bytes();
        let grab_max_time = &self.player_grab_max_time.to_ne_bytes();
        let grab_distance = &self.player_grab_distance.to_ne_bytes();
        let grab_target_radian = &self.player_grab_target_radian.to_ne_bytes();
        let grab_string_position_x = &self.player_grab_string_position.x.to_ne_bytes();
        let grab_string_position_y = &self.player_grab_string_position.y.to_ne_bytes();
        let move_state = &self.player_move_state.to_ne_bytes();
        let grab_state = &self.player_grab_state.to_ne_bytes();
        let grab_time = &self.player_grab_time.to_ne_bytes();

        let mut data = Vec::new();
        data.extend_from_slice(x);
        data.extend_from_slice(target_x);
        data.extend_from_slice(target_direction);
        data.extend_from_slice(grab_max_time);
        data.extend_from_slice(grab_distance);
        data.extend_from_slice(grab_target_radian);
        data.extend_from_slice(grab_string_position_x);
        data.extend_from_slice(grab_string_position_y);
        data.extend_from_slice(move_state);
        data.extend_from_slice(grab_state);
        data.extend_from_slice(grab_time);

        self.stream.write(data.as_slice()).unwrap();
        self.stream.flush().unwrap();
    }

    fn recv_opponent_data(&mut self) {
        let mut buf = [0u8; 44];
        match self.stream.read_exact(&mut buf) {
            Ok(_) => {
                let x = f32::from_ne_bytes(buf[0..4].try_into().unwrap());
                let target_x = f32::from_ne_bytes(buf[4..8].try_into().unwrap());
                let target_direction = i32::from_ne_bytes(buf[8..12].try_into().unwrap());
                let grab_max_time = f32::from_ne_bytes(buf[12..16].try_into().unwrap());
                let grab_distance = f32::from_ne_bytes(buf[16..20].try_into().unwrap());
                let grab_target_radian = f32::from_ne_bytes(buf[20..24].try_into().unwrap());
                let grab_string_position_x = f32::from_ne_bytes(buf[24..28].try_into().unwrap());
                let grab_string_position_y = f32::from_ne_bytes(buf[28..32].try_into().unwrap());
                let move_state = i32::from_ne_bytes(buf[32..36].try_into().unwrap());
                let grab_state = i32::from_ne_bytes(buf[36..40].try_into().unwrap());
                let grab_time = f32::from_ne_bytes(buf[40..44].try_into().unwrap());

                self.opponent_x = x;
                self.opponent_move_state = move_state;
                if self.opponent_grab_state == 1 {
                    if grab_state == -1 {
                        self.opponent_target_x = target_x;
                        self.opponent_target_direction = -target_direction;
                        self.opponent_grab_max_time = grab_max_time;
                        self.opponent_grab_time = grab_time;
                        self.opponent_grab_distance = grab_distance;
                        self.opponent_grab_target_radian = grab_target_radian;
                        self.opponent_grab_string_position = Point2 {x: grab_string_position_x, y: grab_string_position_y};
                        self.opponent_grab_state = grab_state;

                        if self.opponent_grab_time > 0.0 {
                            self.player_x = self.rng.gen_range(300.0..980.0);
                            self.send_player_data();
                        }
                    }
                }
                else if self.opponent_grab_state == 0 {
                    if grab_state == 1 {
                        self.opponent_target_x = target_x;
                        self.opponent_target_direction = -target_direction;
                        self.opponent_grab_max_time = grab_max_time;
                        self.opponent_grab_time = grab_time;
                        self.opponent_grab_distance = grab_distance;
                        self.opponent_grab_target_radian = grab_target_radian;
                        self.opponent_grab_string_position = Point2 {x: grab_string_position_x, y: grab_string_position_y};
                        self.opponent_grab_state = grab_state;
                    }
                }
            },
            Err(err) => {}
        }
    }
}

impl EventHandler for GGEZ {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let delta = delta(ctx);
        let dt = delta.as_secs() as f32 + delta.subsec_nanos() as f32 * 1e-9;

        self.recv_opponent_data();

        if self.player_grab_state == 1 {
            self.player_grab_time += dt;
            if self.player_grab_time >= self.player_grab_max_time {
                if self.player_target_x - 50.0 <= 1280.0 - self.opponent_x && 1280.0 - self.opponent_x <= self.player_target_x + 150.0 {
                    self.player_grab_state = -1;
                    self.send_player_data();
                }
                else {
                    self.player_grab_state = -1;
                    self.player_grab_time = 0.0;
                    self.send_player_data();
                }
            }
        }
        else if self.player_grab_state == -1 {
            self.player_grab_time -= dt;
            if self.player_grab_time <= 0.0 {
                self.player_grab_state = 0;
                self.send_player_data();
            }
        }
        else {
            if self.opponent_grab_state != -1 {
                if self.player_move_state == -1 { self.player_x -= dt * self.player_speed; }
                if self.player_move_state == 1 { self.player_x += dt * self.player_speed; }

                if self.player_x < 300.0 { self.player_x = 300.0; }
                if self.player_x > 980.0 { self.player_x = 980.0; }

                if self.player_target_direction == -1 {
                    self.player_target_x -= self.target_speed * dt;
                    if self.player_target_x < 200.0 {
                        self.player_target_x = 200.0;
                        self.player_target_direction = 1;
                    }
                }
                else if self.player_target_direction == 1 {
                    self.player_target_x += self.target_speed * dt;
                    if self.player_target_x > 1080.0 {
                        self.player_target_x = 1080.0;
                        self.player_target_direction = -1;
                    }
                }
            }
        }

        if self.opponent_grab_state == 1 {
            self.opponent_grab_time += dt;
        }
        else if self.opponent_grab_state == -1 {
            self.opponent_grab_time -= dt;
            if self.opponent_grab_time <= 0.0 {
                self.opponent_grab_state = 0;
            }
        }
        else {
            if self.player_grab_state != -1 {
                if self.opponent_move_state == -1 { self.opponent_x -= dt * self.player_speed; }
                if self.opponent_move_state == 1 { self.opponent_x += dt * self.player_speed; }

                if self.opponent_x < 300.0 { self.opponent_x = 300.0; }
                if self.opponent_x > 980.0 { self.opponent_x = 980.0; }

                if self.opponent_target_direction == -1 {
                    self.opponent_target_x += self.target_speed * dt;
                    if self.opponent_target_x > 1080.0 {
                        self.opponent_target_x = 1080.0;
                        self.opponent_target_direction = 1;
                    }
                }
                else if self.opponent_target_direction == 1 {
                    self.opponent_target_x -= self.target_speed * dt;
                    if self.opponent_target_x < 200.0 {
                        self.opponent_target_x = 200.0;
                        self.opponent_target_direction = -1;
                    }
                }
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        clear(ctx, Color::WHITE);
        draw(ctx, &self.background_image, DrawParam::new())?;
        draw(ctx, &self.foreground_image, DrawParam::new())?;

        if self.player_grab_state == -1 {
            let grab_straigt_radian = self.player_grab_target_radian + 270.0 * PI / 180.0;
            let player_grab_hand_position = Point2 {x: self.player_grab_string_position.x - grab_straigt_radian.cos() * self.player_grab_time * self.grab_speed, y: self.player_grab_string_position.y - grab_straigt_radian.sin() * self.player_grab_time * self.grab_speed};
            let opponent_draw_params = DrawParam::new()
                .dest(player_grab_hand_position)
                .offset(Point2 {x: 0.5, y: 0.5 });
            draw(ctx, &self.player_image, opponent_draw_params)?;
        }
        else {
            if self.opponent_grab_state == 0 {
                let opponent_target_radian = (580.0_f64).atan2((self.opponent_x - self.opponent_target_x) as f64) as f32 - PI / 2.0;
                let opponent_target_draw_params = DrawParam::new()
                    .dest(Point2 {x: 1280.0 - self.opponent_x, y: 720.0 - 650.0})
                    .rotation(opponent_target_radian)
                    .offset(Point2 {x: 0.5, y: 0.0 })
                    .color(Color::CYAN);
                draw(ctx, &self.target_image, opponent_target_draw_params)?;

                let opponent_draw_params = DrawParam::new()
                    .dest(Point2 {x: 1280.0 - self.opponent_x, y: 720.0 - 650.0})
                    .rotation(if self.opponent_move_state == -1 { -45.0 * PI / 180.0 } else { if self.opponent_move_state == 1 { 45.0 * PI / 180.0 } else { 0.0 }})
                    .offset(Point2 {x: 0.5, y: 0.5 });
                draw(ctx, &self.player_image, opponent_draw_params)?;
            }
            else {
                let opponent_target_draw_params = DrawParam::new()
                    .dest(Point2 {x: 1280.0 - self.opponent_x, y: 720.0 - 650.0})
                    .rotation(self.opponent_grab_target_radian + PI)
                    .offset(Point2 {x: 0.5, y: 0.0 })
                    .color(Color::BLACK);
                draw(ctx, &self.target_image, opponent_target_draw_params)?;

                let opponent_draw_params = DrawParam::new()
                    .dest(Point2 {x: 1280.0 - self.opponent_x, y: 720.0 - 650.0})
                    .rotation(self.opponent_grab_target_radian + PI)
                    .offset(Point2 {x: 0.5, y: 0.5 });
                draw(ctx, &self.player_grab_image, opponent_draw_params)?;

                let opponent_grab_string_draw_params = DrawParam::new()
                    .dest(Point2 {x: 1280.0 - self.opponent_grab_string_position.x as f32, y: 720.0 - self.opponent_grab_string_position.y as f32})
                    .rotation(self.opponent_grab_target_radian + PI)
                    .offset(Point2 {x: 0.5, y: 0.0 })
                    .scale([1.0, self.opponent_grab_distance / self.grab_string_image.height() as f32 * self.opponent_grab_time / self.opponent_grab_max_time]);
                draw(ctx, &self.grab_string_image, opponent_grab_string_draw_params)?;

                let opponent_grab_straigt_radian = self.opponent_grab_target_radian + 270.0 * PI / 180.0;
                let opponent_grab_hand_position = Point2 {x: self.opponent_grab_string_position.x - opponent_grab_straigt_radian.cos() * self.opponent_grab_time * self.grab_speed, y: self.opponent_grab_string_position.y - opponent_grab_straigt_radian.sin() * self.opponent_grab_time * self.grab_speed};
                let opponent_grab_hand_draw_params = DrawParam::new()
                    .dest(Point2 {x: 1280.0 - opponent_grab_hand_position.x as f32, y: 720.0 - opponent_grab_hand_position.y as f32})
                    .rotation(self.opponent_grab_target_radian + PI)
                    .offset(Point2 {x: 0.5, y: 0.0 });
                draw(ctx, &self.grab_hand_image, opponent_grab_hand_draw_params)?;
            }
        }

        if self.opponent_grab_state == -1 {
            let opponent_grab_straigt_radian = self.opponent_grab_target_radian + 270.0 * PI / 180.0;
            let opponent_grab_hand_position = Point2 {x: self.opponent_grab_string_position.x - opponent_grab_straigt_radian.cos() * self.opponent_grab_time * self.grab_speed, y: self.opponent_grab_string_position.y - opponent_grab_straigt_radian.sin() * self.opponent_grab_time * self.grab_speed};
            let player_draw_params = DrawParam::new()
                .dest(Point2 {x: 1280.0 - opponent_grab_hand_position.x as f32, y: 720.0 - opponent_grab_hand_position.y as f32})
                .rotation(PI)
                .offset(Point2 {x: 0.5, y: 0.5 });
            draw(ctx, &self.player_image, player_draw_params)?;
        }
        else {
            if self.player_grab_state == 0 {
                let player_target_radian = (580.0_f64).atan2((self.player_x - self.player_target_x) as f64) as f32 + PI / 2.0;
                let player_target_draw_params = DrawParam::new()
                    .dest(Point2 {x: self.player_x, y: 650.0})
                    .rotation(player_target_radian)
                    .offset(Point2 {x: 0.5, y: 0.0 })
                    .color(Color::CYAN);
                draw(ctx, &self.target_image, player_target_draw_params)?;
    
                let player_draw_params = DrawParam::new()
                    .dest(Point2 {x: self.player_x, y: 650.0})
                    .rotation(if self.player_move_state == -1 { 135.0 * PI / 180.0 } else { if self.player_move_state == 1 { 225.0 * PI / 180.0 } else { 180.0 * PI / 180.0 }})
                    .offset(Point2 {x: 0.5, y: 0.5 });
                draw(ctx, &self.player_image, player_draw_params)?;
            }
            else {
                let player_target_draw_params = DrawParam::new()
                    .dest(Point2 {x: self.player_x, y: 650.0})
                    .rotation(self.player_grab_target_radian)
                    .offset(Point2 {x: 0.5, y: 0.0 })
                    .color(Color::BLACK);
                draw(ctx, &self.target_image, player_target_draw_params)?;
    
                let player_draw_params = DrawParam::new()
                    .dest(Point2 {x: self.player_x, y: 650.0})
                    .rotation(self.player_grab_target_radian)
                    .offset(Point2 {x: 0.5, y: 0.5 });
                draw(ctx, &self.player_grab_image, player_draw_params)?;
    
                let grab_string_draw_params = DrawParam::new()
                    .dest(self.player_grab_string_position)
                    .rotation(self.player_grab_target_radian)
                    .offset(Point2 {x: 0.5, y: 0.0 })
                    .scale([1.0, self.player_grab_distance / self.grab_string_image.height() as f32 * self.player_grab_time / self.player_grab_max_time]);
                draw(ctx, &self.grab_string_image, grab_string_draw_params)?;
    
                let grab_straigt_radian = self.player_grab_target_radian + 270.0 * PI / 180.0;
                let player_grab_hand_position = Point2 {x: self.player_grab_string_position.x - grab_straigt_radian.cos() * self.player_grab_time * self.grab_speed, y: self.player_grab_string_position.y - grab_straigt_radian.sin() * self.player_grab_time * self.grab_speed};
                let grab_hand_draw_params = DrawParam::new()
                    .dest(player_grab_hand_position)
                    .rotation(self.player_grab_target_radian)
                    .offset(Point2 {x: 0.5, y: 0.0 });
                draw(ctx, &self.grab_hand_image, grab_hand_draw_params)?;   
            }
        }
        
        present(ctx)?;
        Ok(())
    }

    fn key_down_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymods: KeyMods, repeat: bool) {
        if is_key_pressed(ctx, KeyCode::Escape) {
            quit(ctx);
        }

        if keycode == KeyCode::Space && self.player_grab_state == 0 {
            self.player_grab_target_radian = (580.0_f64).atan2((self.player_x - self.player_target_x) as f64) as f32 + PI / 2.0;
            
            let grab_string_radian = self.player_grab_target_radian + 130.0 * PI / 180.0;
            let grab_straigt_radian = self.player_grab_target_radian + 270.0 * PI / 180.0;
            self.player_grab_string_position = Point2 {x: self.player_x + grab_string_radian.cos() * 80.0, y: 650.0 + grab_string_radian.sin() * 80.0};

            self.player_grab_max_time = (650.0 + grab_string_radian.sin() * 80.0 - 120.0) / (grab_straigt_radian.sin() * self.grab_speed);
            
            self.player_grab_distance = (((self.player_target_x - self.player_grab_string_position.x as f32) * (self.player_target_x - self.player_grab_string_position.x as f32) + (120.0 - self.player_grab_string_position.y as f32) * (120.0 - self.player_grab_string_position.y as f32)) as f64).sqrt() as f32;

            self.player_grab_time = 0.0;
            self.player_grab_state = 1;

            self.send_player_data();
        }

        self.player_move_state = 0;
        if is_key_pressed(ctx, KeyCode::Left) { self.player_move_state = -1 }
        if is_key_pressed(ctx, KeyCode::Right) { self.player_move_state = 1 }
        self.send_player_data();
    }

    fn key_up_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymod: KeyMods) {
        self.player_move_state = 0;
        if is_key_pressed(ctx, KeyCode::Left) { self.player_move_state = -1 }
        if is_key_pressed(ctx, KeyCode::Right) { self.player_move_state = 1 }
        self.send_player_data();
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

    let background_image = Image::new(&mut ctx, "/background.png").unwrap();
    let foreground_image = Image::new(&mut ctx, "/foreground.png").unwrap();
    let player_image = Image::new(&mut ctx, "/player.png").unwrap();
    let player_grab_image = Image::new(&mut ctx, "/player_grab.png").unwrap();
    let grab_string_image = Image::new(&mut ctx, "/grab_string.png").unwrap();
    let grab_hand_image = Image::new(&mut ctx, "/grab_hand.png").unwrap();
    let target_image = Image::new(&mut ctx, "/target.png").unwrap();
    let rng = thread_rng();

    let tcp_stream = match TcpListener::bind("127.0.0.1:9999") {
        Ok(res) => {
            println!("== Server ==");
            println!("TCP port 9999 listen...");
            let mut incoming_streams = res.incoming();
            let stream = incoming_streams.next().unwrap().unwrap();
            let opponent_ip_address = stream.peer_addr().unwrap();
            println!("Opponent connected: {}", opponent_ip_address);
            stream.set_nonblocking(true).unwrap();
            stream
        },
        Err(err) => {
            println!("== Client ==");
            println!("TCP port 9999 connect...");
            let stream = TcpStream::connect("127.0.0.1:9999").unwrap();
            let opponent_ip_address = stream.peer_addr().unwrap();
            println!("Connected to opponent: {}", opponent_ip_address);
            stream.set_nonblocking(true).unwrap();
            stream
        }
    };

    run(ctx, event_loop, GGEZ {
        background_image: background_image,
        foreground_image: foreground_image,
        player_image: player_image,
        player_grab_image: player_grab_image,
        grab_string_image: grab_string_image,
        grab_hand_image: grab_hand_image,
        target_image: target_image,

        player_x: 640.0,
        player_target_x: 980.0,
        player_target_direction: -1,
        player_grab_time: 0.0,
        player_grab_max_time: 0.0,
        player_grab_distance: 0.0,
        player_grab_target_radian: 0.0,
        player_grab_string_position: Point2 {x: 0.0, y: 0.0},
        player_move_state: 0,
        player_grab_state: 0,

        opponent_x: 640.0,
        opponent_target_x: 300.0,
        opponent_target_direction: 1,
        opponent_grab_time: 0.0,
        opponent_grab_max_time: 0.0,
        opponent_grab_distance: 0.0,
        opponent_grab_target_radian: 0.0,
        opponent_grab_string_position: Point2 {x: 0.0, y: 0.0},
        opponent_move_state: 0,
        opponent_grab_state: 0,

        player_speed: 300.0,
        target_speed: 800.0,
        grab_speed: 800.0,
        rng: rng,
        stream: tcp_stream,
    });
}
