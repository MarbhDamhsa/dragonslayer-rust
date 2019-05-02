extern crate tcod;

use tcod::console::*;
use tcod::colors::{self, Color};

//Actual size of the window
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;

//FPS Maximum
const FPS_LIMIT: i32 = 20;


const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;

const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOR_DARK_GROUND: Color = Color { r: 50, g: 50, b: 150 };

type Map = Vec<Vec<Tile>>;

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

fn make_map() -> Map {
	// fill the map with unblocked tiles
	let mut map = vec![vec![Tile::empty(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];

	// place two pillars to test the map
	map[30][22] = Tile::wall();
	map[50][22] = Tile::wall();

	map
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

    // create object representing the player
    let player = Object::new(SCREEN_WIDTH / 2, SCREEN_HEIGHT / 2, '@', colors::WHITE);

    // create an NPC
    let npc = Object::new(SCREEN_WIDTH / 2 - 5, SCREEN_HEIGHT / 2, '@', colors::YELLOW);

    // the list of objects with those two
    let mut objects = [player, npc];

    // generate map
    let map = make_map();
    
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
