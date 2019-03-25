#[macro_use]
extern crate glium;
extern crate image;

mod game;
mod graphics;
mod protocol;

use glium::glutin;
use graphics::console::*;
use graphics::tileset::*;
use std::net::{TcpStream, SocketAddr};
use std::slice::from_raw_parts;
use std::time::Duration;
use std::io::prelude::*;
use std::thread;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;

use std::env::args;

const SECRET: &str = "15185185";
const SIZE: (u32, u32) = (80, 80);
const NAME: &str = "Roguelike";

fn main() {
    let argsv: Vec<String> = args().collect();
    let ip = if argsv.len() > 1 {
        argsv[1].clone()
    }else {
        panic!("IP to connect must be given!")
    };
    let display_name = if argsv.len() > 2 {
        argsv[2].clone()
    }else {
        "anonymous".to_owned()
    };
    let mut game_data = game::Game{entity_vec: HashMap::new(), player_pos:(0,0)};

    let mut closed = false;

    let mut name = [0; 128];
    let mut version = [0; 16];
    let mut secret = [0; 32];
    protocol::write_to_buff(&mut name, &display_name);
    protocol::write_to_buff(&mut version, "00000");
    protocol::write_to_buff(&mut secret, SECRET);
    let addres: SocketAddr = ip.parse().unwrap();
    let mut socket = TcpStream::connect(addres).unwrap();
    let mut read_socket = socket.try_clone().unwrap();
    let mut is_ok = false;
    let mut server_name = String::new();
    while !is_ok {

        sleep(Duration::from_millis(1500));
        unsafe {
            let hs_struct = protocol::protocol::HandShakeClient {
                displayName: name,
                clientVersion: version,
                characterSecret: secret,
            };

            socket.write(from_raw_parts(
                protocol::protocol::handShakeClientToBuffer(hs_struct),
                protocol::protocol::PACKET_SIZE as usize,
            )).unwrap();
            let mut buff = [0; protocol::protocol::PACKET_SIZE as usize];
            socket.read_exact(&mut buff[..]).unwrap();
            let hs_server = protocol::protocol::bufferToHandShakeServer(&mut buff[0]);
            if hs_server.status == protocol::protocol::Status_OK{
                is_ok = true;
            }
            server_name = String::from(String::from_utf8_lossy(&hs_server.serverName));
            std::fs::create_dir(format!("{}({})",server_name, ip)).unwrap();
        }
    }
    let mut len: u32 = 1;
    let mut buff_v = vec![];
    socket.read_to_end(&mut buff_v).unwrap();
    unsafe{
        let buff = buff_v.as_mut_slice();
        while len > 0 || protocol::protocol::identify(buff[0], 1) != protocol::protocol::PacketTypes_UPDATE_FILE_HEADER_TYPE{
            let packet = protocol::protocol::bufferToupdateFileHeader(&mut buff[0]);
            len = packet.length;
            let mut file = File::create(format!("{}({})\\{}",server_name, ip, String::from(String::from_utf8_lossy(&packet.name)))).unwrap();
            file.write_all(&buff[protocol::protocol::HEADER_SIZE as usize..]).unwrap();
        }
    }

    let ts = TileSet::new("tileset.png", (10, 10), (0, 0));
    let mut read_thread;
    let mut root = Root::new(ts, SIZE, NAME);
    let is_there_data = AtomicBool::new(false);
    let arc = Arc::new(is_there_data);

    let ark = arc.clone();
    read_thread = thread::spawn(move || {
        let mut buff = [0; protocol::protocol::PACKET_SIZE as usize];
        read_socket.read_exact(&mut buff[..]).unwrap();
        ark.store(true, Ordering::Relaxed);
        return buff;
    });
    while !closed {
        root.clear();
        for game_datum in game_data.entity_vec.iter() {
            let r_x = (game_datum.1).0 - game_data.player_pos.0 + SIZE.0  as i32 / 2;
            let r_y = (game_datum.1).1 - game_data.player_pos.1 + SIZE.1  as i32 / 2;
            if game_data.player_pos.0 - (SIZE.0  as i32 / 2) < (game_datum.1).0 && game_data.player_pos.0 + (SIZE.0 as i32 / 2) > (game_datum.1).0 &&
                game_data.player_pos.1 - (SIZE.1 as i32 / 2) < (game_datum.1).1 && game_data.player_pos.1 + (SIZE.1 as i32 / 2) > (game_datum.1).1{
                root.put_colored_char('P' as u32,[255,255,255], [0,0,0], (r_x as u32 ,r_y as u32));
            }
        }
        root.put_colored_char('@' as u32,[255,255,255], [0,0,0], (SIZE.0 / 2, SIZE.1 / 2));
        root.draw();
        root.events_loop.poll_events(|ev| match ev {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::CloseRequested => closed = true,
                glutin::WindowEvent::KeyboardInput { input, .. } => match input.virtual_keycode {
                    Some(key) => {
                        match key {
                            glutin::VirtualKeyCode::Up => {
                                unsafe {
                                    let move_packet = protocol::protocol::MovementPacket {
                                        direction: protocol::protocol::Direction_NORTH,
                                        characterSecret: secret,
                                    };
                                    socket.write(from_raw_parts(
                                        protocol::protocol::movementToBuffer(move_packet),
                                        protocol::protocol::PACKET_SIZE as usize,
                                    )).unwrap();
                                }
                                game_data.player_pos = (game_data.player_pos.0, game_data.player_pos.1 - 1);
                            }
                            glutin::VirtualKeyCode::Down => {
                                unsafe {
                                    let move_packet = protocol::protocol::MovementPacket {
                                        direction: protocol::protocol::Direction_SOUTH,
                                        characterSecret: secret,
                                    };
                                    socket.write(from_raw_parts(
                                        protocol::protocol::movementToBuffer(move_packet),
                                        protocol::protocol::PACKET_SIZE as usize,
                                    )).unwrap();
                                }

                                game_data.player_pos = (game_data.player_pos.0, game_data.player_pos.1 + 1);
                            }
                            glutin::VirtualKeyCode::Right => {
                                unsafe {
                                    let move_packet = protocol::protocol::MovementPacket {
                                        direction: protocol::protocol::Direction_EAST,
                                        characterSecret: secret,
                                    };
                                    socket.write(from_raw_parts(
                                        protocol::protocol::movementToBuffer(move_packet),
                                        protocol::protocol::PACKET_SIZE as usize,
                                    )).unwrap();
                                }
                                game_data.player_pos = (game_data.player_pos.0 + 1, game_data.player_pos.1);
                            }
                            glutin::VirtualKeyCode::Left => {
                                unsafe {
                                    let move_packet = protocol::protocol::MovementPacket {
                                        direction: protocol::protocol::Direction_WEST,
                                        characterSecret: secret,
                                    };
                                    socket.write(from_raw_parts(
                                        protocol::protocol::movementToBuffer(move_packet),
                                        protocol::protocol::PACKET_SIZE as usize,
                                    )).unwrap();
                                }
                                game_data.player_pos = (game_data.player_pos.0 - 1, game_data.player_pos.1);
                            }
                            _ => {}
                        }
                    }
                    _ => (),
                },
                _ => (),
            },
            _ => (),
        });
        //networking
        if arc.load(Ordering::Relaxed) {
            let mut data = read_thread.join().unwrap();
            arc.store(false, Ordering::Relaxed);
            read_socket = socket.try_clone().unwrap();
            let ark = arc.clone();
            read_thread = thread::spawn(move || {
                let mut buff = [0; protocol::protocol::PACKET_SIZE as usize];
                read_socket.read_exact(&mut buff[..]).unwrap();
                ark.store(true, Ordering::Relaxed);
                return buff;
            });
            let packet_type = unsafe { protocol::protocol::identify(data[0], 1)};
            unsafe {
                if packet_type == protocol::protocol::PacketTypes_UPDATE_POSITION_TYPE {
                    let packet = protocol::protocol::bufferToUpdatePosition(&mut data[0]);
                    game_data.entity_vec.insert(packet.id,(packet.x, packet.y));
                }
            }
            arc.store(false, Ordering::Relaxed);
        }
    }
}

