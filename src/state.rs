use libm::{cosf, fabsf, sinf, atan2f};

use rand::{SeedableRng, Rng};
use rand::rngs::SmallRng;

use core::fmt::Write;
use heapless::{String,Vec};

use crate::constants::{
    WIDTH, HEIGHT, NUM_CELLS, MAX_PASSAGES, STEP_SIZE, BULLET_SPEED, 
    RELOAD_TIME, NUM_BULLETS, NUM_PLAYERS, BULLETS_PER_PLAYER, PLAYER_WIDTH
};

use crate::wasm4::{
    tone,
    TONE_NOISE,
    trace
};

use maze_gen::{find_passages, find_walls, there_is_no_passage_here};
use crate::arms::{Ammo, Bullet};

use crate::util::{distance, get_index};

#[derive(Clone, Copy)]
pub enum View {
    FirstPerson,
    TopDown
}

pub struct State {
    pub player_x: [f32; NUM_PLAYERS],
    pub player_y: [f32; NUM_PLAYERS],
    pub player_angle: [f32; NUM_PLAYERS],
    pub player_ammo: [[Ammo;BULLETS_PER_PLAYER]; NUM_PLAYERS],
    pub player_life: [i32; NUM_PLAYERS],
    pub player_view: [View; NUM_PLAYERS],
    pub bullets: Vec<Bullet,NUM_BULLETS>,
    visited: Vec<bool,NUM_CELLS>,
    passages: Vec<(usize,usize),MAX_PASSAGES>,
    pub horizontal_walls: Vec<u16,{HEIGHT+1}>,
    pub vertical_walls: Vec<u16,{WIDTH+1}>,
    seed: u64
}

impl State {

    pub const fn new() -> State {
        State {
            player_x: [0.5; NUM_PLAYERS],
            player_y: [0.5; NUM_PLAYERS],
            player_angle: [0.0; NUM_PLAYERS],
            player_ammo: [[Ammo::Loaded; BULLETS_PER_PLAYER]; NUM_PLAYERS],
            player_life: [5; NUM_PLAYERS],
            player_view: [View::FirstPerson; NUM_PLAYERS],
            bullets: Vec::<Bullet,NUM_BULLETS>::new(),
            visited: Vec::<bool,NUM_CELLS>::new(),
            passages: Vec::<(usize,usize),MAX_PASSAGES>::new(),
            horizontal_walls: Vec::<u16,{HEIGHT+1}>::new(),
            vertical_walls: Vec::<u16,{WIDTH+1}>::new(),
            seed: 0
        }
    }

    /// Called from the game start(), this creates a random maze.
    pub fn generate_maze(&mut self, rng: &mut SmallRng) {
        // Initialize an empty maze
        self.visited.extend_from_slice(&[false;NUM_CELLS]).unwrap();
        self.horizontal_walls.extend_from_slice(&[0b0000000000000000;{HEIGHT+1}]).unwrap();
        self.vertical_walls.extend_from_slice(&[0b0000000000000000;{WIDTH+1}]).unwrap();

        // Randomly create passages to define the maze, starting from first index
        let index = 0;
        find_passages(index, WIDTH, HEIGHT, &mut self.visited, &mut self.passages, rng);

        // Use the passages to define the walls of the maze
        find_walls(WIDTH, HEIGHT, &mut self.passages, &mut self.horizontal_walls, &mut self.vertical_walls);

        self.seed = rng.gen::<u64>();
    }

    /// Update the game state based on user input.
    pub fn update(
        &mut self, 
        p1_up: bool, p1_down: bool, p1_left: bool, p1_right: bool, p1_shoot: bool, p1_toggle_view: bool,
        p2_up: bool, p2_down: bool, p2_left: bool, p2_right: bool, p2_shoot: bool, p2_toggle_view: bool,
        p3_up: bool, p3_down: bool, p3_left: bool, p3_right: bool, p3_shoot: bool, p3_toggle_view: bool,
        p4_up: bool, p4_down: bool, p4_left: bool, p4_right: bool, p4_shoot: bool, p4_toggle_view: bool
    ) {
        // Player 1
        if self.player_life[0] > 0 {
            self.update_player(0, p1_up, p1_down, p1_left, p1_right);
            self.update_ammo(0, p1_shoot);
            self.update_view(0, p1_toggle_view);
        }
        // Player 2
        if self.player_life[1] > 0 {
            self.update_player(1, p2_up, p2_down, p2_left, p2_right);
            self.update_ammo(1, p2_shoot);
            self.update_view(1, p2_toggle_view);
        }
        // Player 3
        if self.player_life[2] > 0 {
            self.update_player(2, p3_up, p3_down, p3_left, p3_right);
            self.update_ammo(2, p3_shoot);
            self.update_view(2, p3_toggle_view);
        }
        // Player 4
        if self.player_life[3] > 0 {
            self.update_player(3, p4_up, p4_down, p4_left, p4_right);
            self.update_ammo(3, p4_shoot);
            self.update_view(3, p4_toggle_view);
        }
        // Bullets in flight
        self.update_bullets();
    }

    /// Toggle a players view
    fn update_view(&mut self, pidx: usize, toggle_view: bool) {
        if toggle_view {
            self.player_view[pidx] = match self.player_view[pidx] {
                View::FirstPerson => View::TopDown,
                View::TopDown => View::FirstPerson
            }
        }
    }

    /// Moves a player around based on user input.
    fn update_player(&mut self, pidx: usize, up: bool, down: bool, left: bool, right: bool) {
        let mut player_x = self.player_x[pidx];
        let mut player_y = self.player_y[pidx];
        let mut player_angle = self.player_angle[pidx];

        // Store the current index in case we need to undo a move.
        let previous_index = get_index(player_x, player_y, WIDTH, HEIGHT);

        // Tentative updates to player position and orientation.
        if up {
            player_x += cosf(player_angle) * STEP_SIZE;
            player_y += -sinf(player_angle) * STEP_SIZE;
        }
        if down {
            player_x -= cosf(player_angle) * STEP_SIZE;
            player_y -= -sinf(player_angle) * STEP_SIZE;
        }
        if right {
            player_angle -= STEP_SIZE;
        }
        if left {
            player_angle += STEP_SIZE;
        }

        // If the player has moved to a new cell, then new_index will differ from previous_index.
        let new_index = get_index(player_x, player_y, WIDTH, HEIGHT);

        // Conditionally apply the move.
        if ( // If move would cause player to leave the maze...
            (player_x <= 0.0) ||
            (player_y <= 0.0) ||
            (player_x as usize >= WIDTH) ||
            (player_y as usize >= HEIGHT)
        ) ||
        ( // ...or if they would go through a wall...
            (previous_index != new_index) && 
            there_is_no_passage_here(previous_index, new_index, &self.passages)
        )
        { // ... do not apply the move.
            self.player_angle[pidx] = player_angle;
        } else {
            self.player_x[pidx] = player_x;
            self.player_y[pidx] = player_y;
            self.player_angle[pidx] = player_angle;
        }
    }

    /// Fires a bullet in response to player input; incrementally reloads spent ammo.
    fn update_ammo(&mut self, pidx: usize, shoot: bool) {
        // When the player presses the x button.
        if shoot { 
            // Find the first loaded ammo
            match self.player_ammo[pidx].iter_mut().find(|&&mut a| a == Ammo::Loaded) {
                Some(ammo) => {
                    // Change it to reloading
                    *ammo = Ammo::Reloading(RELOAD_TIME);
                    trace("Shot bullet");
                    tone(1000 | (10 << 16), 10, 100, TONE_NOISE);
                    let mut rng = SmallRng::seed_from_u64(self.seed);
                    self.seed = rng.gen::<u64>();
                    let attempt = self.bullets.push(
                        Bullet::new(
                            self.player_x[pidx],
                            self.player_y[pidx],
                            pidx,
                            self.player_angle[pidx] + (rng.gen::<f32>() - 0.5)/10.0,
                            true
                        )
                    );
                    match attempt {
                        Ok(()) => {},
                        Err(_) => trace("Reached max number of bullets in the air.")
                    }
                },
                None => trace("Empty")
            }
        }

        // Find the first ammo that is not loaded and incrementally reload it.
        // Spent ammo take RELOAD_TIME frames to reload and are reloaded one at a time.
        match self.player_ammo[pidx].iter_mut().find(|&&mut a| a != Ammo::Loaded) {
            Some(ammo) => match ammo {
                // Decrement time to reload until we reach 0 (means we are loaded)
                Ammo::Reloading(time_to_reload) => {
                    if *time_to_reload > 0 {
                        *ammo = Ammo::Reloading(*time_to_reload-1);
                    } else {
                        *ammo = Ammo::Loaded;
                        trace("Loaded!");
                        tone(50 | (150 << 16), 50, 100, TONE_NOISE);
                    }
                }
                _ => {}
            },
            None => {}
        }
    }

    /// Propagates bullets in flight, removing them when they hit a wall.
    fn update_bullets(&mut self) {
        // Update the position of each bullet in flight.
        self.bullets.iter_mut().for_each(|b| {
            let previous_index = get_index(b.x, b.y, WIDTH, HEIGHT);
            b.x += cosf(b.angle) * BULLET_SPEED;
            b.y += -sinf(b.angle) * BULLET_SPEED;
            let new_index = get_index(b.x, b.y, WIDTH, HEIGHT);
            if ( // If bullet leaves the maze...
                b.x <= 0.0 || b.y <= 0.0 || b.x as usize >= WIDTH || b.y as usize >= HEIGHT
            ) || ( // ...or if it would go through a wall...
                (previous_index != new_index) && 
                there_is_no_passage_here(previous_index, new_index, &self.passages)
            ) { // ... mark inflight as false.
                b.inflight = false;
            }
            
            if b.inflight {
                for pidx in 0..NUM_PLAYERS {
                    if pidx != b.owner {
                        let player_index = get_index(
                            self.player_x[pidx], 
                            self.player_y[pidx], 
                            WIDTH, 
                            HEIGHT
                        );
                        if player_index == new_index {
                            // Get the relative angle between bullet and player
                            let rise = self.player_y[pidx] - self.player_y[b.owner];
                            let run = self.player_x[pidx] - self.player_x[b.owner];
                            let angle_to_player = -1.0 * atan2f(rise, run);
                            let relative_angle = self.player_angle[pidx] - angle_to_player;

                            // Determine collision window
                            let striking_distance = PLAYER_WIDTH * fabsf(cosf(relative_angle));
                            let separation = distance(
                                b.x - self.player_x[pidx], 
                                b.y - self.player_y[pidx]
                            );

                            if separation <= striking_distance {
                                trace("Hit!");
                                self.player_life[pidx] -= 1;
                                b.inflight = false;
                            }
                        }
                    }
                }
            }
        });

        // Remove bullets that are no longer inflight.
        self.bullets = self.bullets.iter().map(|b| *b).filter(|b| b.inflight == true).collect();
    }
}