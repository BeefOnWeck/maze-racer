use heapless::Vec;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;

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

fn there_is_no_passage_here<const N: usize>(
    index: usize, 
    neighbor: usize, 
    passages: &mut Vec<(usize,usize), N>
) -> bool {
    for pass in passages {
        if (index == pass.0 && neighbor == pass.1) || (index == pass.1 && neighbor == pass.0) {
            return false;
        }
    }
    return true;
}

pub fn find_next_passage<const M: usize, const N: usize>(
    index: usize, 
    width: usize, 
    height: usize, 
    visited: &mut Vec<bool,M>, 
    passages: &mut Vec<(usize,usize),N>,
    rng: &mut SmallRng
) {

    visited[index] = true;

    let neighbors = find_neighbors(index, width, height);
    let mut potential_passages: Vec<usize,4> = neighbors.into_iter()
        .flatten() // Option implements IntoIter
        .filter(|&n| visited[n] == false)
        .filter(|&n| there_is_no_passage_here(index, n, passages))
        .collect();
    potential_passages.shuffle(rng);

    for pass in potential_passages {
        if visited[pass] == false {
            passages.push((index,pass)).unwrap();
            find_next_passage(pass, width, height, visited, passages, rng);
        }
    }
}

#[cfg(test)]

#[test]
pub fn wont_you_be_my_neighbor() {
    let width = 4;
    let height = 4;

    let index = 0;
    let neighbors = find_neighbors(index, width, height);
    assert_eq!(
        neighbors,
        [Some(4), None, None, Some(1)]
    );

    let index = 5;
    let neighbors = find_neighbors(index, width, height);
    assert_eq!(
        neighbors,
        [Some(9), Some(1), Some(4), Some(6)]
    );

    let index = 13;
    let neighbors = find_neighbors(index, width, height);
    assert_eq!(
        neighbors,
        [None, Some(9), Some(12), Some(14)]
    );
}

#[test]
pub fn simple() {
    let index = 0;
    let width = 16;
    let height = 16;
    let num_cells = width * height;
    let mut visited = vec![false; num_cells];
    let mut passages = Vec::<(usize,usize)>::new();
    let mut rng = SmallRng::seed_from_u64(11);
    find_next_passage(index, width, height, &mut visited, &mut passages, &mut rng);
}
