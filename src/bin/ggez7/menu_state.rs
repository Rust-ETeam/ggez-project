use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;

use ggez::event::{EventHandler, KeyCode, KeyMods, Button, MouseButton};
use ggez::{Context, GameError};
use ggez::graphics::{draw, Color, DrawParam, Image, Text};
use ggez::mint::Point2;

use crate::helper::{load_image, IState, EState};
use crate::game_state::GameState;

enum EInnerState {
    unkown,         // guest인지 host인지 선택하지 않은 상태
    waiting_guest, // host로서 guest를 기다리는 상태
    typing_host_ip, // guest로서 접속할 host의 ip를 입력하는 상태
}

struct ButtonRect {
    x : f32, 
    y : f32, 
    s_x : f32,
    s_y : f32, 
}

impl ButtonRect {
    fn isInIt(&mut self, x : f32, y: f32) -> bool {
        if self.x - self.s_x / 2.0 <= x && x <= self.x + self.s_x / 2.0 {
            if self.y - self.s_y / 2.0 <= y && y <= self.y + self.s_y / 2.0 {
                return true;
            }
        }

        return false;
    }
}

pub struct MenuState {
    state : EInnerState,
    ip_str : String, 
    host_button_image : Rc<Image>, 
    guest_button_image : Rc<Image>, 
    // cancel_button_image : Rc<Image>, 

    host_button_rect : ButtonRect, 
    guest_button_rect : ButtonRect, 

    should_end_state : bool,

    sender : SyncSender<TcpStream>, 
    receiver : Receiver<TcpStream>,
    pub tcp_stream : Option<Rc<RefCell<TcpStream>>>,
}

impl MenuState {
    pub fn new(
        ctx: &mut Context, 
        image_pool: &mut HashMap<String, Rc<Image>>,
    ) -> MenuState {
        let host_button_image = load_image(ctx, String::from("/host_button.png"), image_pool);
        let guest_button_image = load_image(ctx, String::from("/guest_button.png"), image_pool);
        // let cancel_button_image = load_image(ctx, String::from("/cancel_button.png"), image_pool);

        let (sender, receiver) = mpsc::sync_channel(1);

        MenuState { 
            state: EInnerState::unkown, 
            ip_str: String::new(), 
            host_button_image: host_button_image, 
            guest_button_image: guest_button_image, 
            // cancel_button_image: cancel_button_image,
            host_button_rect : ButtonRect { x: 640.0, y: 320.0, s_x: 195.0, s_y: 49.0 }, 
            guest_button_rect : ButtonRect { x: 640.0, y: 395.0, s_x: 207.0, s_y: 49.0 }, 
            should_end_state: false, 
            tcp_stream : None,
            sender: sender, 
            receiver: receiver, 
        }
    }

    pub fn IsServer(&mut self) -> bool {
        match self.state {
            EInnerState::waiting_guest  => {
                return true;
            },
            _ => {
                return false;
            }
        }
    }
}

impl IState for MenuState {
    fn update(&mut self, _ctx: &mut ggez::Context) -> EState {
        if self.should_end_state {
            return EState::Game
        }

        match self.state {
            EInnerState::unkown => {},
            EInnerState::waiting_guest => {
                let res = self.receiver.try_recv();
                if res.is_ok() {
                    self.tcp_stream = Some(Rc::new(RefCell::new(res.unwrap())));
                    self.should_end_state = true;
                }
            },
            EInnerState::typing_host_ip => {},
        }

        EState::None
    }

    fn draw(&mut self, ctx: &mut ggez::Context){
        match self.state {
            EInnerState::unkown => {
                let host_button_param = DrawParam::new()
                    .dest(Point2{ x: self.host_button_rect.x, y: self.host_button_rect.y })
                    .offset(Point2{ x: 0.5, y: 0.5});
                draw(ctx, self.host_button_image.as_ref(), host_button_param).expect("draw failed");

                let guest_button_param = DrawParam::new()
                    .dest(Point2{ x: self.guest_button_rect.x, y: self.guest_button_rect.y })
                    .offset(Point2{ x: 0.5, y: 0.5});
                draw(ctx, self.guest_button_image.as_ref(), guest_button_param).expect("draw failed");

                let param1 = DrawParam::new()
                .dest(Point2 { x: 600.0, y: 200.0 })
                .offset(Point2 { x: 0.5, y: 0.5 })
                .scale([3.0, 3.0])
                .color(Color::WHITE);
                draw(
                    ctx,
                    &Text::new(String::from("GGEZ")),
                    param1,
                ).expect("draw failed");
            },
            EInnerState::waiting_guest => {
                let param = DrawParam::new()
                    .dest(Point2 { x: 560.0, y: 220.0 })
                    .offset(Point2 { x: 0.5, y: 0.5 })
                    .scale([2.0, 2.0])
                    .color(Color::WHITE);
                draw(
                    ctx,
                    &Text::new(format!("waiting geuest...")),
                    param,
                ).expect("draw failed");
            },
            EInnerState::typing_host_ip => {
                let param1 = DrawParam::new()
                .dest(Point2 { x: 140.0, y: 270.0 })
                .offset(Point2 { x: 0.5, y: 0.5 })
                .scale([2.0, 2.0])
                .color(Color::WHITE);
                draw(
                    ctx,
                    &Text::new(String::from("Type Host Socket Addr and Press Enter! (??.??.??.??:9999)")),
                    param1,
                ).expect("draw failed");
            
                let param2 = DrawParam::new()
                .dest(Point2 { x: 540.0, y: 320.0 })
                .offset(Point2 { x: 0.5, y: 0.5 })
                .scale([2.0, 2.0])
                .color(Color::WHITE);
            draw(
                ctx,
                &Text::new(self.ip_str.clone()),
                param2,
            ).expect("draw failed");
            },
        }
    }

    fn key_down_event(&mut self, _: &mut Context, keycode: KeyCode, keymods: KeyMods, repeat: bool) {
        match self.state {
            EInnerState::typing_host_ip => {
                match keycode {
                    KeyCode::Key0 | KeyCode::Numpad0 => {
                        self.ip_str.push('0');
                    },
                    KeyCode::Key1 | KeyCode::Numpad1 => {
                        self.ip_str.push('1');
                    },
                    KeyCode::Key2 | KeyCode::Numpad2 => {
                        self.ip_str.push('2');
                    },
                    KeyCode::Key3 | KeyCode::Numpad3 => {
                        self.ip_str.push('3');
                    },
                    KeyCode::Key4 | KeyCode::Numpad4 => {
                        self.ip_str.push('4');
                    },
                    KeyCode::Key5 | KeyCode::Numpad5 => {
                        self.ip_str.push('5');
                    },
                    KeyCode::Key6 | KeyCode::Numpad6 => {
                        self.ip_str.push('6');
                    },
                    KeyCode::Key7 | KeyCode::Numpad7 => {
                        self.ip_str.push('7');
                    },
                    KeyCode::Key8 | KeyCode::Numpad8 => {
                        self.ip_str.push('8');
                    },
                    KeyCode::Key9 | KeyCode::Numpad9 => {
                        self.ip_str.push('9');
                    },
                    KeyCode::Back => {
                        self.ip_str.pop();
                    },
                    KeyCode::Colon | KeyCode::Semicolon => {
                        self.ip_str.push(':');
                    },
                    KeyCode::Period => {
                        self.ip_str.push('.');
                    },
                    KeyCode::Return | KeyCode::NumpadEnter => {
                        println!("connecting as guest... ");
                        println!("TCP {} connect...", self.ip_str);
                        let stream = TcpStream::connect(self.ip_str.clone()).unwrap();
                        let opponent_ip_address = stream.peer_addr().unwrap();
                        println!("Connected to opponent: {}", opponent_ip_address);
                        stream.set_nonblocking(true).unwrap();
                        self.tcp_stream = Some(Rc::new(RefCell::new(stream)));
                        self.should_end_state = true;
                    }
                    _ => {}
                }
            },
            _ => {}
        }
    }

    fn key_up_event(&mut self, ctx: &mut Context, keycode: KeyCode, keymods: KeyMods) {
    }

    fn mouse_button_down_event(&mut self, _ctx: &mut Context, _button: MouseButton, x: f32, y: f32
    ) {
        print!("{} {} \n", x, y);

        match self.state {
            EInnerState::unkown => {
                if self.host_button_rect.isInIt(x, y) {
                    print!("host! \n");
                    self.state = EInnerState::waiting_guest;
                    
                    let sender2 = self.sender.clone();

                    let tcp_listener = TcpListener::bind("127.0.0.1:9999").expect("tcp bind failed");
                    thread::spawn(move || {
                        println!("host waiting guest... ");
                        println!("TCP port 9999 listen... ");
                        let stream  = tcp_listener.incoming().next().unwrap().unwrap();
                        let opponent_ip_address = stream.peer_addr().unwrap();
                        println!("Opponent connected: {}", opponent_ip_address);
                        stream.set_nonblocking(true).unwrap();
                        sender2.send(stream).unwrap();
                    });
                    self.state = EInnerState::waiting_guest;
                } else if self.guest_button_rect.isInIt(x, y) {
                    print!("guest! \n");
                    self.state = EInnerState::typing_host_ip;
                }
            }
            EInnerState::waiting_guest => {},
            EInnerState::typing_host_ip => {},
        }
    }
}