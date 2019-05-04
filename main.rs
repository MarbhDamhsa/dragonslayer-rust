extern crate tcod;
extern crate rand;

use std::cmp;
use rand::Rng;

use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::map::{Map as FovMap, FovAlgorithm};

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
const COLOR_LIGHT_WALL: Color = Color { r: 130, g: 110, b: 50 };
const COLOR_DARK_GROUND: Color = Color { r: 50, g: 50, b: 150 };
const COLOR_LIGHT_GROUND: Color = Color { r: 200, g: 180, b: 50 };


//Room constraints
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;


// Field of View
const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
const FOV_LIGHT_WALLS: bool = true;
const TORCH_RADIUS: i32 = 10;


const MAX_ROOM_MONSTERS: i32 = 3;

const PLAYER: usize = 0;

type Map = Vec<Vec<Tile>>;

/////////////////////////////
//////
//////    Structs, etc.
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
	explored: bool,
}

impl Tile {
	pub fn empty() -> Self {
		Tile{ blocked: false, explored: false, block_sight: false }
	}

	pub fn wall() -> Self {
		Tile{ blocked: true, explored: false, block_sight: true }
	}
}

#[derive(Debug)]
struct Object {
	x: i32,
	y: i32,
	char: char,
	color: Color,
	name: String,
	blocks: bool,
	alive: bool,
}

impl Object {
	pub fn new(x: i32, y: i32, char: char, name: &str, color: Color, blocks: bool) -> Self {
		Object {
			x: x,
			y: y,
			char: char,
			color: color,
			name: name.into(),
			blocks: blocks,
			alive: false,
		}
	}

	// set the color and draw the character that represents this object at its position
	pub fn draw(&self, con: &mut Console) {
		con.set_default_foreground(self.color);
		con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
	}

	pub fn pos(&self) -> (i32, i32) {
		(self.x, self.y)
	}

	pub fn set_pos(&mut self, x: i32, y: i32) {
		self.x = x;
		self.y = y;
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
	TookTurn,
	DidntTakeTurn,
	Exit,
}


/////////////////////
/////
/////
/////
/////////////////////

fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Object>) {
	// choose random number of monsters
	let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);


		for _ in 0..num_monsters {
			// choose random location for the monster
			let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
			let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);
		
			// Only place if the tile is not blocked
			if !is_blocked(x, y, map, objects) {
				let mut monster = if rand::random::<f32>() < 0.8 { // 80% chance of getting an orc
					// create an orc
					Object::new(x, y, 'o', "orc", colors::DESATURATED_GREEN, true)
				} else {
					Object::new(x, y, 'T', "troll", colors::DARKER_GREEN, true)
				};
		
			monster.alive = true;
			objects.push(monster);
		}
	}
}

fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
	// first test the map tile
	if map[x as usize][y as usize].blocked {
		return true;
	}

	// now check for any blocking objects
	objects.iter().any(|object| {
		object.blocks && object.pos() == (x, y)
	})
}


fn make_map(objects: &mut Vec<Object>) -> Map {
	// fill map with wall tiles
	let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];
	
	let mut rooms = vec![];

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

				// Add content to the room
				place_objects(new_room, &map, objects);

				// center coordinates of the new room, useful later
				let (new_x, new_y) = new_room.center();

				if rooms.is_empty() {
					// this is the first room where the player starts
					objects[PLAYER].set_pos(new_x, new_y);
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


	map
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



fn handle_keys(root: &mut Root, objects: &mut [Object], map: &Map) -> PlayerAction {

	use PlayerAction::*;
	use tcod::input::Key;
	use tcod::input::KeyCode::*;

	let key = root.wait_for_keypress(true);
	let player_alive = objects[PLAYER].alive;
	match (key, player_alive) {

		//Alt+Enter: Toggle Fullscreen
		(Key { code: Enter, alt: true, .. }, _) => {
			let fullscreen = root.is_fullscreen();
			root.set_fullscreen(!fullscreen);
			DidntTakeTurn
		}

		// Exit game
		(Key { code: Escape, .. }, _) => return Exit,


		// Movement Keys
		(Key { code: Up, .. }, true) => {
			player_move_or_attack(0, -1, map, objects);
			TookTurn
		},
		(Key { code: Down, .. }, true) => {
			player_move_or_attack(0, 1, map, objects);
			TookTurn
		},
		(Key { code: Left, .. }, true) => {
			player_move_or_attack(-1, 0, map, objects);
			TookTurn
		},
		(Key { code: Right, .. }, true) => {
			player_move_or_attack(1, 0, map, objects);
			TookTurn
		},

		_ => DidntTakeTurn,
	}
}

// Move by the given amount if destination isn't blocked
//
//  DEPRECIATED
//
fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]){
	let (x, y) = objects[id].pos();
	if !is_blocked(x + dx, y + dy, map, objects) {
		objects[id].set_pos(x + dx, y + dy);
	}
}

fn player_move_or_attack(dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
	// the coordinates the player is moving to/attacking
	let x = objects[PLAYER].x + dx;
	let y = objects[PLAYER].y + dy;

	// Look for an attackable object there
	let target_id = objects.iter().position(|object| {
		object.pos() == (x, y)
	});

	// Attack if target found, otherwise move
	match target_id {
		Some(target_id) => {
			println!("The {} laughs at your puny efforts to attack him!", objects[target_id].name);
		}
		None => {
			move_by(PLAYER, dx, dy, map, objects);
		}
	}
}


fn render_all(root: &mut Root, con: &mut Offscreen, objects: &[Object], map: &mut Map,
			fov_map: &mut FovMap, fov_recompute: bool){
	if fov_recompute {
		// recompute FOV if needed
		let player = &objects[0];
		fov_map.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);
	}

	//go through all the tiles and set their background color
	for y in 0..MAP_HEIGHT {
		for x in 0..MAP_WIDTH {
			let visible = fov_map.is_in_fov(x, y);
			let wall = map[x as usize][y as usize].block_sight;
			let color = match (visible, wall) {
				// outside of field of view:
				(false, true) => COLOR_DARK_WALL,
				(false, false) => COLOR_DARK_GROUND,
				// inside fov:
				(true, true) => COLOR_LIGHT_WALL,
				(true, false) => COLOR_LIGHT_GROUND,
			};

			let explored = &mut map[x as usize][y as usize].explored;
			if visible {
				// since it's visible, explore it
				*explored = true;
			}
			if *explored {
				// show explored tiles only
				con.set_char_background(x, y, color, BackgroundFlag::Set);
			}
		}
	}

	//draw all the objects in the list
	for object in objects {
		if fov_map.is_in_fov(object.x, object.y) {
			object.draw(con);
		}
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

    // Place player inside first room
    let mut player = Object::new(0, 0, '@', "player", colors::WHITE, true);
    player.alive = true;

    // the list of objects with just the player
    let mut objects = vec![player];

    // generate map
    let mut map = make_map(&mut objects);


    // create an NPC
    //let npc = Object::new(SCREEN_WIDTH / 2 - 5, SCREEN_HEIGHT / 2, '@', "npc", colors::YELLOW, true);

    // create the FOV map
    let mut fov_map = FovMap::new(MAP_WIDTH, MAP_HEIGHT);
    for y in 0..MAP_HEIGHT {
    	for x in 0..MAP_WIDTH {
    		fov_map.set(x, y,
    					!map[x as usize][y as usize].block_sight,
    					!map[x as usize][y as usize].blocked);
    	}
    }

    // Force FOV to recompute the first time through the loop
    let mut previous_player_position = (-1, -1);

    while !root.window_closed() {

    	// Clear the screen of the previous frame
    	con.clear();

    	// render the screen
    	let fov_recompute = previous_player_position != (objects[0].x, objects[0].y);
    	render_all(&mut root, &mut con, &objects, &mut map, &mut fov_map, fov_recompute);

    	root.flush();

    	// handle keys and exit game if needed
    	previous_player_position = objects[PLAYER].pos();
    	let player_action = handle_keys(&mut root, &mut objects, &map);
    	if player_action == PlayerAction::Exit {
    		break
    	}

    	// let monsters take their turn
    	if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
    		for object in &objects {
    			// only if object is not player
    			if (object as *const _) != (&objects[PLAYER] as *const _) {
    				println!("The {} growls!", object.name);
    			}
    		}
    	}
    }
}
