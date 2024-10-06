use libm::{atan2f, cosf, fabsf, floorf, sinf};
use core::f32::consts::{PI};

use rand::{SeedableRng, Rng};
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;

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

use crate::util::{distance, get_center_from_index, get_index};

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
    paths: Vec::<usize, MAX_PASSAGES>,
    pruned_path: Vec::<usize, MAX_PASSAGES>,
    stack: Vec::<usize, MAX_PASSAGES>,
    seed: u64
}

impl State {

    pub const fn new() -> State {
        State {
            player_x: [0.5, 12.5, 0.5, 12.5],
            player_y: [0.5, 0.5, 12.5, 12.5],
            player_angle: [0.75, 2.25, 3.75, 5.25],
            player_ammo: [[Ammo::Loaded; BULLETS_PER_PLAYER]; NUM_PLAYERS],
            player_life: [5; NUM_PLAYERS],
            player_view: [View::FirstPerson; NUM_PLAYERS],
            bullets: Vec::<Bullet,NUM_BULLETS>::new(),
            visited: Vec::<bool,NUM_CELLS>::new(),
            passages: Vec::<(usize,usize),MAX_PASSAGES>::new(),
            horizontal_walls: Vec::<u16,{HEIGHT+1}>::new(),
            vertical_walls: Vec::<u16,{WIDTH+1}>::new(),
            paths: Vec::<usize, MAX_PASSAGES>::new(),
            pruned_path: Vec::<usize, MAX_PASSAGES>::new(),
            stack: Vec::<usize, MAX_PASSAGES>::new(),
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
        p1_up: bool, p1_down: bool, p1_left: bool, p1_right: bool, p1_shoot: bool, p1_toggle_view: bool
    ) {
        // AI Update for Players 2, 3, 4
        let (p2_up, p2_down, p2_left, p2_right, p2_shoot, p2_toggle_view) = self.update_enemy(2);
        let (p3_up, p3_down, p3_left, p3_right, p3_shoot, p3_toggle_view) = self.update_enemy(3);
        let (p4_up, p4_down, p4_left, p4_right, p4_shoot, p4_toggle_view) = self.update_enemy(4);

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

        // Enemy players should move more slowly
        let step_size = if pidx > 0 {STEP_SIZE * 0.65} else {STEP_SIZE};

        // Tentative updates to player position and orientation.
        if up {
            player_x += cosf(player_angle) * step_size;
            player_y += -sinf(player_angle) * step_size;
        }
        if down {
            player_x -= cosf(player_angle) * step_size;
            player_y -= -sinf(player_angle) * step_size;
        }
        if right {
            player_angle -= step_size;
        }
        if left {
            player_angle += step_size;
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
        // Kludge to limit enemies to just one bullet
        if pidx > 0 {
            let first_bullet = self.player_ammo[pidx][0];
            self.player_ammo[pidx] = [Ammo::Reloading(RELOAD_TIME); BULLETS_PER_PLAYER];
            self.player_ammo[pidx][0] = first_bullet;
        }

        // When the player presses the x button.
        if shoot {
            // Find the first loaded ammo
            match self.player_ammo[pidx].iter_mut().find(|&&mut a| a == Ammo::Loaded) {
                Some(ammo) => {
                    // Change it to reloading
                    *ammo = Ammo::Reloading(RELOAD_TIME);
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

    fn update_enemy(&mut self, pid: usize) -> (bool,bool,bool,bool,bool,bool) {

        let idx = pid - 1;
        let mut rng = SmallRng::seed_from_u64(self.seed);
        self.seed = rng.gen::<u64>();

        let enemy_index = get_index(self.player_x[idx], self.player_y[idx], WIDTH, HEIGHT);
        let player_index = get_index(self.player_x[0], self.player_y[0], WIDTH, HEIGHT);

        self.visited.clear();
        self.visited.extend_from_slice(&[false;NUM_CELLS]).unwrap();
        self.paths.clear();
        self.pruned_path.clear();
        self.stack.clear();

        State::find_path(
            enemy_index, 
            player_index, 
            WIDTH, HEIGHT, 
            &mut self.passages, 
            &mut self.visited, 
            &mut self.stack, 
            &mut self.paths, 
            &mut self.pruned_path, 
            &mut rng
        );

        let target_index;
        if self.pruned_path.len() > 1 {
            target_index = self.pruned_path.remove(1);
        } else {
            target_index = enemy_index;
        }

        let (target_x, target_y) = get_center_from_index(target_index, WIDTH, HEIGHT);

        // Calculate the angle and unwrap
        let enemy_angle = self.player_angle[idx];
        let rise = target_y - self.player_y[idx];
        let run = target_x - self.player_x[idx];
        let target_angle = -1.0 * atan2f(rise, run);

        let num_wraps = floorf((target_angle - enemy_angle)/(2.0 * PI));
        let unwrapped = target_angle - 2.0 * PI * num_wraps;
        let extra_unwrapped = unwrapped - 2.0 * PI;

        // Sometimes unwrapping is off by one (end condition)
        let extra_is_closer = fabsf(enemy_angle - unwrapped) > fabsf(enemy_angle - extra_unwrapped);
        let unwrapped_angle = if extra_is_closer {
            extra_unwrapped
        } else {
            unwrapped
        };

        let angle_diff = unwrapped_angle - enemy_angle;

        // let mut data = String::<32>::new();
        // if pid == 2 {
        //     write!(data, "1: {angle_diff}, 2: {enemy_angle}\n").unwrap();
        //     trace(data);
        // }

        let distance_to_player = distance(
            self.player_y[0] - self.player_y[idx],
            self.player_x[0] - self.player_x[idx] 
        );
        let fire = distance_to_player <= 3.0;

        if fabsf(angle_diff) <= 0.08 {
            let target_distance = distance(rise, run);
            if target_distance < 0.05 {
                (false,false,false,false,fire,false)
            } else {
                (true,false,false,false,fire,false)
            }
        } else if angle_diff > 0.08 {
            (false,false,true,false,false,false)
        } else {
            (false,false,false,true,false,false)
        }
    }

    fn find_path<const M: usize, const N: usize>(
        start: usize,
        end: usize,
        width: usize, 
        height: usize, 
        passages: &mut Vec<(usize,usize),N>,
        visited: &mut Vec<bool,M>,
        stack: &mut Vec<usize,N>,
        paths: &mut Vec<usize,N>,
        pruned_path: &mut Vec<usize,N>,
        rng: &mut SmallRng
    ) {
    
        visited[start] = true;
        stack.push(start).unwrap();
        let mut still_looking = true;
        let mut checkpoint = start;
    
        while let Some(node) = stack.pop() {
            if still_looking {
                visited[node] = true;
                let neighbors = State::find_neighbors(node, width, height);
                let mut potential_paths: Vec<usize,4> = neighbors.into_iter()
                    .flatten() // Option implements IntoIter
                    .filter(|&n| visited[n] == false)
                    .filter(|&n| there_is_no_passage_here(node, n, passages) == false)
                    .collect();
                potential_paths.shuffle(rng);
    
                if node == end {
                    paths.push(node).unwrap();
                    still_looking = false;
                } else if potential_paths.len() > 1 {
                    checkpoint = node;
                    paths.push(node).unwrap();
                } else if potential_paths.len() == 0 {
                    while let Some(p) = paths.pop() {
                        if p == checkpoint {
                            paths.push(checkpoint).unwrap();
                            break;
                        }
                    }
                } else {
                    paths.push(node).unwrap();
                }
    
                for pass in potential_paths {
                    stack.push(pass).unwrap();
                }
            }
        }
    
        let mut last_node = paths.pop().unwrap();
        pruned_path.insert(0, last_node).unwrap();
        while let Some(node) = paths.pop() {
            if there_is_no_passage_here(last_node, node, passages) == false {
                pruned_path.insert(0, node).unwrap();
                last_node = node;
            }
        }
    }

    fn find_neighbors(index: usize, width: usize, height: usize) -> [Option<usize>;4] {
        let num_cells = width * height;
    
        let up = if index < num_cells - width {
            Some(index + width)
        } else {
            None
        };
    
        let down = if index > width - 1 {
            Some(index - width)
        } else {
            None
        };
    
        let left = if index % width != 0 {
            Some(index - 1)
        } else {
            None
        };
    
        let right = if (index + 1) % width != 0 {
            Some(index + 1)
        } else {
            None
        };
    
        return [up, down, left, right];
    }
}