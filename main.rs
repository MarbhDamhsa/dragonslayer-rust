extern crate tcod;
extern crate rand;

use std::cmp;
use rand::Rng;

use tcod::console::*;
use tcod::colors::{self, Color};
use tcod::map::{Map as FovMap, FovAlgorithm};
use tcod::input::{self, Event, Key, Mouse};


//Actual size of the window
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;


//FPS Maximum
const FPS_LIMIT: i32 = 20;


//Map window size
const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 43;


//Tile colors
const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOR_LIGHT_WALL: Color = Color { r: 130, g: 110, b: 50 };
const COLOR_DARK_GROUND: Color = Color { r: 50, g: 50, b: 150 };
const COLOR_LIGHT_GROUND: Color = Color { r: 200, g: 180, b: 50 };


//Room constraints
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;
const MAX_ROOM_MONSTERS: i32 = 3;
const MAX_ROOM_ITEMS: i32 = 2;


// Field of View
const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic;
const FOV_LIGHT_WALLS: bool = true;
const TORCH_RADIUS: i32 = 10;

// GUI Panel
const BAR_WIDTH: i32 = 20;
const PANEL_HEIGHT: i32 = 7;
const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;
const INVENTORY_WIDTH: i32 = 50;

// Message Bar
const MSG_X: i32 = BAR_WIDTH + 2;
const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
const MSG_HEIGHT: usize = PANEL_HEIGHT as usize -1;

const PLAYER: usize = 0;

const HEAL_AMOUNT: i32 = 4;

type Map = Vec<Vec<Tile>>;

type Messages = Vec<(String, Color)>;

/////////////////////////////
//////
//////    Structs, etc.
//////
/////////////////////////////

struct Tcod {
	root: Root,
	con: Offscreen,
	panel: Offscreen,
	fov: FovMap,
	mouse: Mouse,
}

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
	fighter: Option<Fighter>,
	ai: Option<Ai>,
	item: Option<Item>,
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
			fighter: None,
			ai: None,
			item: None,
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
	
	pub fn distance_to(&self, other: &Object) -> f32 {
		let dx = other.x - self.x;
		let dy = other.y - self.y;
		((dx.pow(2) + dy.pow(2)) as f32).sqrt()
	}

	pub fn take_damage(&mut self, damage: i32, messages: &mut Messages) {
		// apply damage if possible
		if let Some(fighter) = self.fighter.as_mut() {
			if damage > 0 {
				fighter.hp -= damage;
			}
		}

		// check for death, call the death function
		if let Some(fighter) = self.fighter {
			if fighter.hp <= 0 {
				self.alive = false;
				fighter.on_death.callback(self, messages);
			}
		}
	}

	pub fn heal(&mut self, amount: i32) {
		if let Some(ref mut fighter) = self.fighter {
			fighter.hp += amount;
			if fighter.hp > fighter.max_hp {
				fighter.hp = fighter.max_hp;
			}
		}
	}

	pub fn attack(&mut self, target: &mut Object, messages: &mut Messages) {
		// a simple damage formula
		let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defense);
		if damage > 0 {
			// target takes dmaage
			message(messages, format!("{} attacks {} for {} hit points.", self.name, target.name, damage), colors::WHITE);
			target.take_damage(damage, messages);
		} else {
			message(messages, format!("{} attacks {} but it has no effect!", self.name, target.name), colors::WHITE);
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
	TookTurn,
	DidntTakeTurn,
	Exit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DeathCallBack {
	Player,
	Monster,
}

impl DeathCallBack {
	fn callback(self, object: &mut Object, messages: &mut Messages) {
		use DeathCallBack::*;
		let callback: fn(&mut Object, &mut Messages) = match self {
			Player => player_death,
			Monster => monster_death,
		};
		callback(object, messages);
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Fighter {
	max_hp: i32,
	hp: i32,
	defense: i32,
	power: i32,
	on_death: DeathCallBack,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Ai;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Item {
	Heal,
}

enum UseResult {
	UsedUp,
	Cancelled,
}

/////////////////////
/////
/////  Functions
/////
/////////////////////

fn player_death(player: &mut Object, messages: &mut Messages) {
	// the game ends
	message(messages, "You died!", colors::RED);

	// for added effect, transform player into a corpse
	player.char = '%';
	player.color = colors::DARK_RED;
}

fn monster_death(monster: &mut Object, messages: &mut Messages) {
	// transform the monster into a corpse
	// Doesn't block, cant be attacked, doesn't move
	message(messages, format!("{} is dead!", monster.name), colors::ORANGE);
	monster.char = '%';
	monster.color = colors::DARK_RED;
	monster.blocks = false;
	monster.fighter = None;
	monster.ai = None;
	monster.name = format!("remains of {}", monster.name);
}

fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) -> (&mut T, &mut T) {
	assert!(first_index != second_index);
	let split_at_index = cmp::max(first_index, second_index);
	let (first_slice, second_slice) = items.split_at_mut(split_at_index);
	if first_index < second_index {
		(&mut first_slice[first_index], &mut second_slice[0])
	} else {
		(&mut second_slice[0], &mut first_slice[second_index])
	}
}

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
					
					let mut orc = Object::new(x, y, 'o', "orc", colors::DESATURATED_GREEN, true);
					orc.fighter = Some(Fighter{max_hp: 10, hp: 10, defense: 0, power: 3, on_death: DeathCallBack::Monster});
					orc.ai = Some(Ai);
					orc
				} else {
					// create a troll
					let mut troll = Object::new(x, y, 'T', "troll", colors::DARKER_GREEN, true);
					troll.fighter = Some(Fighter{max_hp: 16, hp: 16, defense: 1, power: 4, on_death: DeathCallBack::Monster});
					troll.ai = Some(Ai);
					troll
				};
		
			monster.alive = true;
			objects.push(monster);
		}
	}

	// Choose random number of items
	let num_items = rand::thread_rng().gen_range(0, MAX_ROOM_ITEMS + 1);

	for _ in 0..num_items {
		// choose random spot for this item
		let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
		let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

		// only place item if the tile is not blocked
		if !is_blocked(x, y, map, objects) {
			// create a healing potion
			let mut object = Object::new(x, y, '!', "healing potion", colors::VIOLET, false);
			object.item = Some(Item::Heal);
			objects.push(object);
		}
	}
}

fn pick_item_up(object_id: usize, objects: &mut Vec<Object>, inventory: &mut Vec<Object>,
				messages: &mut Messages) {
	if inventory.len() >= 26 {
		message(messages,
			format!("Your inventory is full, cannot pick up {}.", objects[object_id].name), colors::RED);
	} else {
		let item = objects.swap_remove(object_id);
		message(messages, format!("You picked up a {}!", item.name), colors::GREEN);
		inventory.push(item);
	}
}

fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [Object]){
	// vector from this object to the target, and distance
	let dx = target_x - objects[id].x;
	let dy = target_y - objects[id].y;
	let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

	// normalise it to length 1 (preservind direction), then round it and
	// convert to integer so the movement is restricted to the grid
	let dx = (dx as f32 /distance).round() as i32;
	let dy = (dy as f32/ distance).round() as i32;
	move_by(id, dx, dy, map, objects);
}

fn ai_take_turn(monster_id: usize, map: &Map, objects: &mut [Object], fov_map: &FovMap, messages: &mut Messages) {
	// a basic monster takes its turn. If you can see it, it can see you
	let (monster_x, monster_y) = objects[monster_id].pos();
	if fov_map.is_in_fov(monster_x, monster_y) {
		if objects[monster_id].distance_to(&objects[PLAYER]) >= 2.0 {
			// move towards the player if far enough away
			let (player_x, player_y) = objects[PLAYER].pos();
			move_towards(monster_id, player_x, player_y, map, objects);
		} else if objects[PLAYER].fighter.map_or(false, |f| f.hp > 0) {
			// attack if the player is still alive
			let (monster, player) = mut_two(monster_id, PLAYER, objects);
			monster.attack(player, messages);
			println!("The attack of the {} bounces off your shiny metal armor!", monster.name);
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



fn handle_keys(key: Key, tcod: &mut Tcod, objects: &mut Vec<Object>, map: &Map, 
	messages: &mut Messages, inventory: &mut Vec<Object>) -> PlayerAction {

	use PlayerAction::*;
	use tcod::input::Key;
	use tcod::input::KeyCode::*;

	
	let player_alive = objects[PLAYER].alive;
	match (key, player_alive) {

		//Alt+Enter: Toggle Fullscreen
		(Key { code: Enter, alt: true, .. }, _) => {
			let fullscreen = tcod.root.is_fullscreen();
			tcod.root.set_fullscreen(!fullscreen);
			DidntTakeTurn
		}

		// Exit game
		(Key { code: Escape, .. }, _) => return Exit,


		// Movement Keys
		(Key { code: Up, .. }, true) => {
			player_move_or_attack(0, -1, map, objects, messages);
			TookTurn
		},
		(Key { code: Down, .. }, true) => {
			player_move_or_attack(0, 1, map, objects, messages);
			TookTurn
		},
		(Key { code: Left, .. }, true) => {
			player_move_or_attack(-1, 0, map, objects, messages);
			TookTurn
		},
		(Key { code: Right, .. }, true) => {
			player_move_or_attack(1, 0, map, objects, messages);
			TookTurn
		},

		(Key { printable: 'g', .. }, true) => {
			// pick up an item
			let item_id = objects.iter().position(|object| {
				object.pos() == objects[PLAYER].pos() && object.item.is_some()
			});
			if let Some(item_id) = item_id {
				pick_item_up(item_id, objects, inventory, messages);
			}
			DidntTakeTurn
		}

		(Key { printable: 'i', .. }, true) => {
			// show the inventory: if an item is selected, use it
			let inventory_index = inventory_menu(
				inventory, "Press the key next to an item to use it or any other to cancel\n",
				&mut tcod.root);
			if let Some(inventory_index) = inventory_index {
				use_item(inventory_index, inventory, objects, messages);
			}
			DidntTakeTurn
		}

		// (Key { printable: 'd', .. }, true) => {
		// 	// show the inventory; if an item is selected, drop it
		// 	let inventory_index = inventory_menu(
		// 		inventory,
		// 		"Press the key next to an item to drop it, or any other to cancel.\n", &mut tcod.root);
		// 	if let Some(inventory_index) = inventory_index {
		// 		drop_item(inventory_index, inventory, objects, messages);
		// 	}
		// 	DidntTakeTurn
		// }

		_ => DidntTakeTurn,
	}
}

fn cast_heal(inventory_id: usize, objects: &mut [Object], messages: &mut Messages) -> UseResult
{
	// heal the player
	if let Some(fighter) = objects[PLAYER].fighter {
		if fighter.hp == fighter.max_hp {
			message(messages, "You are already at full health", colors::RED);
			return UseResult::Cancelled;
		}
		message(messages, "Your wounds start to feel better!", colors::LIGHT_VIOLET);
		objects[PLAYER].heal(HEAL_AMOUNT);
		return UseResult::UsedUp;
	}
	UseResult::Cancelled
}

fn use_item(inventory_id: usize, inventory: &mut Vec<Object>, objects: &mut [Object],
			messages: &mut Messages) {
	use Item::*;

	// just call the use_function if it is defined
	if let Some(item) = inventory[inventory_id].item {
		let on_use = match item {
			Heal => cast_heal,
		};

		match on_use(inventory_id, objects, messages) {
			UseResult::UsedUp => {
				// Destroy after use, unless it was cancelled
				inventory.remove(inventory_id);
			}
			UseResult::Cancelled => {
				message(messages, "Cancelled", colors::WHITE);
			}
		}
	} else {
		message(messages,
				format!("The {} cannot be used.", inventory[inventory_id].name),
				colors::WHITE);
	}
}

fn get_names_under_mouse(mouse: Mouse, objects: &[Object], fov_map: &FovMap) -> String {
	let (x, y) = (mouse.cx as i32, mouse.cy as i32);

	// create a list with the names of all objects at the mouse's coordinates and in FOV
	let names = objects
		.iter()
		.filter(|obj| {obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y)})
		.map(|obj| obj.name.clone())
		.collect::<Vec<_>>();

	names.join(", ") // Join the names, separated by commas
}

// Move by the given amount if destination isn't blocked
fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]){
	let (x, y) = objects[id].pos();
	if !is_blocked(x + dx, y + dy, map, objects) {
		objects[id].set_pos(x + dx, y + dy);
	}
}

fn player_move_or_attack(dx: i32, dy: i32, map: &Map, objects: &mut [Object], messages: &mut Messages) {
	// the coordinates the player is moving to/attacking
	let x = objects[PLAYER].x + dx;
	let y = objects[PLAYER].y + dy;

	// Look for an attackable object there
	let target_id = objects.iter().position(|object| {
		object.fighter.is_some() && object.pos() == (x, y)
	});

	// Attack if target found, otherwise move
	match target_id {
		Some(target_id) => {
			let (player, target) = mut_two(PLAYER, target_id, objects);
			player.attack(target, messages);
		}
		None => {
			move_by(PLAYER, dx, dy, map, objects);
		}
	}
}


fn render_all(tcod: &mut Tcod, objects: &[Object], map: &mut Map,
		 fov_recompute: bool, messages: &Messages){
	if fov_recompute {
		// recompute FOV if needed
		let player = &objects[0];
		tcod.fov.compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);
	}

	//go through all the tiles and set their background color
	for y in 0..MAP_HEIGHT {
		for x in 0..MAP_WIDTH {
			let visible = tcod.fov.is_in_fov(x, y);
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
				tcod.con.set_char_background(x, y, color, BackgroundFlag::Set);
			}
		}
	}

	let mut to_draw: Vec<_> = objects.iter().filter(|o| tcod.fov.is_in_fov(o.x, o.y)).collect();
	
	// sort so that non-blocking objects come first
	to_draw.sort_by(|o1, o2| { o1.blocks.cmp(&o2.blocks) });
	
	// draw the objects in the list
	for object in &to_draw {
		object.draw(&mut tcod.con);
	}


	// blit the contents of "con" to the root console
	blit(&tcod.con, (0, 0), (MAP_WIDTH, MAP_HEIGHT), &mut tcod.root, (0, 0), 1.0, 1.0);


	// Prepare to render the GUI panel
	tcod.panel.set_default_background(colors::BLACK);
	tcod.panel.clear();

	// Print the game messages, one line at a time
	let mut y = MSG_HEIGHT as i32;
	for &(ref msg, color) in messages.iter().rev() {
		let msg_height = tcod.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
		y -= msg_height;
		if y < 0 {
			break;
		}

		tcod.panel.set_default_foreground(color);
		tcod.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
	}

	// show the player's stats
	let hp = objects[PLAYER].fighter.map_or(0,|f| f.hp);
	let max_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp);
	render_bar(&mut tcod.panel, 1, 1, BAR_WIDTH, "HP", hp, max_hp, colors::LIGHT_RED, colors::DARKER_RED);

	// display names of objects under the mouse
	tcod.panel.set_default_foreground(colors::LIGHT_GREY);
	tcod.panel.print_ex(1, 0, BackgroundFlag::None, TextAlignment::Left,
					get_names_under_mouse(tcod.mouse, objects, &tcod.fov));

	// blit the contents of 'panel' to the root console
	blit(&tcod.panel, (0, 0), (SCREEN_WIDTH, PANEL_HEIGHT), &mut tcod.root, (0, PANEL_Y), 1.0, 1.0);
}

fn render_bar(panel: &mut Offscreen,
				x: i32,
				y: i32,
				total_width: i32,
				name: &str,
				value: i32,
				maximum: i32,
				bar_color: Color,
				back_color: Color)
{
	// render a bar (hp, exp, etc) First calculate width of bar
	let bar_width = (value as f32 / maximum as f32 * total_width as f32) as i32;

	// render the background first
	panel.set_default_background(back_color);
	panel.rect(x, y, total_width, 1, false, BackgroundFlag::Screen);

	// now render the bar on top
	panel.set_default_background(bar_color);
	if bar_width > 0 {
		panel.rect(x, y, bar_width, 1, false, BackgroundFlag::Screen);
	}

	// Centered text with the values
	panel.set_default_foreground(colors::WHITE);
	panel.print_ex(x + total_width / 2, y, BackgroundFlag::None, TextAlignment::Center,
				&format!("{}: {}/{}", name, value, maximum));
}

fn message<T: Into<String>>(messages: &mut Messages, message: T, color: Color) {
	// if the buffer is full, remove the first message to make room for the new one
	if messages.len() == MSG_HEIGHT {
		messages.remove(0);
	}
	// add the new line as a tuple with the text and the color
	messages.push((message.into(), color));

}

fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32,
						root: &mut Root) -> Option<usize> {
	assert!(options.len() <= 26, "Cannot have a menu with more than 26 options.");

	// calculate total height for the header (after auto-wrap) and one line per option
	let header_height = root.get_height_rect(0, 0, width, SCREEN_HEIGHT, header);
	let height = options.len() as i32 + header_height;

	// create off-screen console that represents the menu's window
	let mut window = Offscreen::new(width, height);

	// print the header, with auto-wrap
	window.set_default_foreground(colors::WHITE);
	window.print_rect_ex(0, 0, width, height, BackgroundFlag::None, TextAlignment::Left, header);

	// print all the options
	// enum method on iterator gets index for each loop through
	// and uses that to display the corresponding option letter
	for (index, option_text) in options.iter().enumerate() {
		let menu_letter= (b'a' + index as u8) as char;
		let text = format!("({}) {}", menu_letter, option_text.as_ref());
		window.print_ex(0, header_height + index as i32,
					BackgroundFlag::None, TextAlignment::Left, text);
	}

	// blit the contents of "window" to the root console
	let x = SCREEN_WIDTH / 2 - width / 2;
	let y = SCREEN_HEIGHT / 2 - height / 2;
	tcod::console::blit(&mut window, (0, 0), (width,height), root, (x, y), 1.0, 0.7);

	// present the root console to the player and wait for a keypress
	root.flush();
	let key = root.wait_for_keypress(true);

	// convert ASCII code to an index; if it corresponds to an option, return it
	if key.printable.is_alphabetic() {
		let index = key.printable.to_ascii_lowercase() as usize - 'a' as usize;
		if index < options.len() {
			Some(index)
		} else {
			None
		}
	} else {
		None
	}
}

fn inventory_menu(inventory: &[Object], header: &str, root: &mut Root) -> Option<usize> {
	// have a menu with each item of the inventory as an option
	let options = if inventory.len() == 0 {
		vec!["Inventory is empty".into()]
	} else {
		inventory.iter().map(|item| { item.name.clone() }).collect()
	};

	let inventory_index = menu(header, &options, INVENTORY_WIDTH, root);

	// if an item was chose, return it
	if inventory.len() > 0 {
		inventory_index
	} else {
		None
	}
}

///            //||\\      ///  //////   ///
/////        /// ||\\\     ///  /// ///  ///
// ////    ///   ||  \\    ///  ///  /// ///
//  //// ///     ||\\\\\   ///  ///   //////
//   //////      ||    \\  ///  ///    /////
//    ////       ||     \\ ///  ///     ////  
fn main() {
    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Dragonslayer")
        .init();
    tcod::system::set_fps(FPS_LIMIT);

    let mut tcod = Tcod {
    	root: root,
    	con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
    	panel: Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT),
    	fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
    	mouse: Default::default(),
    };

    // Place player inside first room
    let mut player = Object::new(0, 0, '@', "player", colors::WHITE, true);
    player.alive = true;
    player.fighter = Some(Fighter{max_hp: 30, hp: 30, defense: 2, power: 5,
    			 on_death: DeathCallBack::Player});

    // the list of objects with just the player
    let mut objects = vec![player];

    // create the list of game messages and their colors, starts empty
    let mut messages = vec![];

    // generate map
    let mut map = make_map(&mut objects);

    // create the FOV map
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            tcod.fov.set(x, y,
                         !map[x as usize][y as usize].block_sight,
                         !map[x as usize][y as usize].blocked);
        }
    }

    // create an NPC
    //let npc = Object::new(SCREEN_WIDTH / 2 - 5, SCREEN_HEIGHT / 2, '@', "npc", colors::YELLOW, true);

    // Create inventory
    let mut inventory = vec![];

    // Force FOV to recompute the first time through the loop
    let mut previous_player_position = (-1, -1);



    // Keep track of keyboard states
    let mut key = Default::default();

    // Welcome message
    message(&mut messages, "Welcome stranger! Prepare to slay the dragon", colors::RED);

    ///////////////////////
    //					 //
    ////// Main Loop //////
    //				     //
 	///////////////////////
    while !tcod.root.window_closed() {

    	// Clear the screen of the previous frame
    	tcod.con.clear();

    	match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
    		Some((_, Event::Mouse(m))) => tcod.mouse = m,
    		Some((_, Event::Key(k))) => key = k,
    		_ => key = Default::default(),
    	}

    	// render the screen
    	let fov_recompute = previous_player_position != (objects[PLAYER].pos());
    	render_all(&mut tcod,  &objects, &mut map,  fov_recompute,  &messages);

    	tcod.root.flush();

    	// handle keys and exit game if needed
    	previous_player_position = objects[PLAYER].pos();
    	let player_action = handle_keys(key, &mut tcod, &mut objects, &mut map, &mut messages, &mut inventory);
    	if player_action == PlayerAction::Exit {
    		break
    	}

    	// let monsters take their turn
    	if objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
    		for id in 0..objects.len() {
    			if objects[id].ai.is_some() {
    				ai_take_turn(id, &map, &mut objects, &tcod.fov, &mut messages);
    			}
    		}
    	}
    }
}
