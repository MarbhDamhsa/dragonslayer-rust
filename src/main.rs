extern crate tcod;
extern crate rand;

use std::cmp;
use rand::Rng;

use tcod::console::*;
use tcod::colors::{self, Color};

//Actual size of the window
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;

//FPS Maximum
const FPS_LIMIT: i32 = 20;

//Map window size
const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;

//Tile colors
const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOR_DARK_GROUND: Color = Color { r: 50, g: 50, b: 150 };


//Room constraints
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;


type Map = Vec<Vec<Tile>>;

/////////////////////////////
//////
//////    Structs
//////
/////////////////////////////

#[derive(Clone, Copy, Debug)]
struct Rect {
	x1: i32,
	x2: i32,
	y1: i32,
	y2: i32,
}

impl Rect {
	pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
		Rect{ x1: x, y1: y, x2: x + w, y2: y + h}
	}

	pub fn center(&self) -> (i32, i32) {
		let center_x = (self.x1 + self.x2) / 2;
		let center_y = (self.y1 + self.y2) / 2;
		(center_x, center_y)
	}

	pub fn intersects_with(&self, other: &Rect) -> bool {
		//returns true if this rectangle intersects with another one
		(self.x1 <= other.x2) && (self.x2 >= other.x1) &&
			(self.y1 <= other.y2) && (self.y2 >= other.y1)
	}

}

#[derive(Clone, Copy, Debug)]
struct Tile {
	blocked: bool,
	block_sight: bool,
}

impl Tile {
	pub fn empty() -> Self {
		Tile{ blocked: false, block_sight: false }
	}

	pub fn wall() -> Self {
		Tile{ blocked: true, block_sight: true }
	}
}

#[derive(Debug)]
struct Object {
	x: i32,
	y: i32,
	char: char,
	color: Color,
}

impl Object {
	pub fn new(x: i32, y: i32, char: char, color: Color) -> Self {
		Object {
			x: x,
			y: y,
			char: char,
			color: color,
		}
	}

	// move by the given amount
	pub fn move_by(&mut self, dx: i32, dy: i32, map: &Map) {
		if !map[(self.x + dx) as usize][(self.y + dy) as usize].blocked
		{
 			self.x += dx;
			self.y += dy;

		}

	}

	// set the color and draw the character that represents this object at its position
	pub fn draw(&self, con: &mut Console) {
		con.set_default_foreground(self.color);
		con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
	}
}

/////////////////////
/////
/////
/////
/////////////////////

fn make_map() -> (Map, (i32, i32)) {
	// fill map with wall tiles
	let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];
	let mut rooms = vec![];
	let mut starting_position = (0, 0);

	for _ in 0..MAX_ROOMS {
		//random width and height
		let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
		let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);

		//random position without going out of the map boundaries
		let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
		let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

	

		let new_room = Rect::new(x, y, w, h);

		// run through the other rooms and see if they intersect with this one
		let failed = rooms.iter().any(|other_room| new_room.intersects_with(other_room));

		if !failed {
				// No intersections, so room is valid

				create_room(new_room, &mut map);

				// center coordinates of the new room, useful later
				let (new_x, new_y) = new_room.center();

				if rooms.is_empty() {
					// this is the first room where the player starts
					starting_position = (new_x, new_y);
				} else {
					// all rooms after the first:
					// Connect it to the previous room with a runnel

					// center coordinates of the previous room
					let (prev_x, prev_y) = rooms[rooms.len() -1].center();

					// flip a coin
					if rand::random() {
						//first move horizontally, then vertically
						create_h_tunnel(prev_x, new_x, prev_y, &mut map);
						create_v_tunnel(prev_y, new_y, new_x, &mut map);
					} else {
						// first move vertically, then horizontally
						create_v_tunnel(prev_y, new_y, prev_x, &mut map);
						create_h_tunnel(prev_x, new_x, new_y, &mut map);
					}
				
				}

			// finally append the new room to the list
			rooms.push(new_room);
		}
	}


	(map, starting_position)
}

fn create_room(room: Rect, map: &mut Map) {
	for x in (room.x1 + 1)..room.x2 {
		for y in (room.y1 + 1)..room.y2 {
			map[x as usize][y as usize] = Tile::empty();
		}
	}
}

fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map){
	for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
		map[x as usize][y as usize] = Tile::empty();
	}
}

fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map){
	for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
		map[x as usize][y as usize] = Tile::empty();
	}
}

fn handle_keys(root: &mut Root, player: &mut Object, map: &Map) -> bool {

	use tcod::input::Key;
	use tcod::input::KeyCode::*;

	let key = root.wait_for_keypress(true);
	match key {

		//Alt+Enter: Toggle Fullscreen
		Key { code: Enter, alt: true, .. } => {
			let fullscreen = root.is_fullscreen();
			root.set_fullscreen(!fullscreen);
		}

		// Exit 
		Key { code: Escape, .. } => return true,


		// Movement Keys
		Key { code: Up, .. } => player.move_by(0, -1, map),
		Key { code: Down, .. } => player.move_by(0, 1, map),
		Key { code: Left, .. } => player.move_by(-1, 0, map),
		Key { code: Right, .. } => player.move_by(1, 0, map),

		_ => {},
	}


	false
}

fn render_all(root: &mut Root, con: &mut Offscreen, objects: &[Object], map: &Map){
	//go through all the tiles and set their background color
	for y in 0..MAP_HEIGHT {
		for x in 0..MAP_WIDTH {
			let wall = map[x as usize][y as usize].block_sight;
			if wall {
				con.set_char_background(x, y, COLOR_DARK_WALL, BackgroundFlag::Set);
			} else {
				con.set_char_background(x, y, COLOR_DARK_GROUND, BackgroundFlag::Set);
			}
		}
	}

	//draw all the objects in the list
	for object in objects {
		object.draw(con);
	}

	// blit the contents of "con" to the root console
	blit(con, (0, 0), (MAP_WIDTH, MAP_HEIGHT), root, (0, 0), 1.0, 1.0);
}

fn main() {
    let mut root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Dragonslayer")
        .init();
    tcod::system::set_fps(FPS_LIMIT);

    let mut con = Offscreen::new(MAP_WIDTH, MAP_HEIGHT);

    
    // generate map
    let (map, (player_x, player_y)) = make_map();


    // Place player inside first room
    let player = Object::new(player_x, player_y, '@', colors::WHITE);

    // create an NPC
    let npc = Object::new(SCREEN_WIDTH / 2 - 5, SCREEN_HEIGHT / 2, '@', colors::YELLOW);

    // the list of objects with those two
    let mut objects = [player, npc];

    
    while !root.window_closed() {

    	// Clear the screen of the previous frame
    	con.clear();

    	// render the screen
    	render_all(&mut root, &mut con, &objects, &map);

    	root.flush();

    	// handle keys and exit game if needed
    	let player = &mut objects[0];    	
    	let exit = handle_keys(&mut root, player, &map);
    	if exit {
    		break;
    	}
    }
}
