//! Replay an expanded diagnostic/export save through exact fixed steps.
use mine_core::{materials, Game};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let input = args.get(1).expect("replay INPUT SECONDS OUTPUT");
    let seconds: u64 = args.get(2).expect("simulation seconds").parse().unwrap();
    let output = args.get(3).expect("output save path");
    let mut game: Game = serde_json::from_str(&std::fs::read_to_string(input).unwrap()).unwrap();
    game.terrain.rebuild().unwrap();
    game.validate().unwrap();
    let initial = (game.depth(), game.excavated, game.workings.passages.len());
    let started = std::time::Instant::now();
    let catalogue = materials();
    for _ in 0..seconds {
        game.second(&catalogue, true);
    }
    game.validate().unwrap();
    std::fs::write(output, serde_json::to_vec(&game).unwrap()).unwrap();
    println!(
        "{}",
        serde_json::json!({"initial":initial,"depth":game.depth(),"excavated":game.excavated,"passages":game.workings.passages.len(),"status":game.workings.status,"seconds":seconds,"compute_ms":started.elapsed().as_millis()})
    );
}
