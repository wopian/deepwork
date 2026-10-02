//! Public generator atlas and compatibility/performance evidence. No simulation replay.
use mine_core::{geology, materials, Game};
use std::{collections::BTreeMap, fmt::Write};
fn main() {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "test-results/natural-veins".into());
    std::fs::create_dir_all(&output).unwrap();
    let mut svg=String::from("<svg xmlns='http://www.w3.org/2000/svg' width='1080' height='1070' viewBox='0 0 1080 1070'><rect width='1080' height='1070' fill='#131722'/><style>text{font-family:monospace;fill:#ddd;font-size:15px}rect{shape-rendering:crispEdges}</style>");
    let mut yields = BTreeMap::new();
    for (index, (feed, depth)) in [
        (5, 264),
        (3, 240),
        (24, 744),
        (8, 552),
        (19, 120),
        (42, 960),
        (53, 3072),
        (2, 1056),
    ]
    .into_iter()
    .enumerate()
    {
        let left = 16 + (index % 2) * 536;
        let top = 36 + (index / 2) * 264;
        write!(
            svg,
            "<text x='{left}' y='{}'>{} · seed 42 · {depth} m</text>",
            top - 12,
            materials()[feed].name
        )
        .unwrap();
        let mut counts = BTreeMap::new();
        for row in 0..224i64 {
            let y = depth * 4 - 112 + row;
            let mut start = 0;
            let mut old = geology::sample_versioned(42, 0, 6, 0, y, materials());
            for x in 0..=512i64 {
                let id = if x == 512 {
                    usize::MAX
                } else {
                    geology::sample_versioned(42, 0, 6, x, y, materials())
                };
                if x < 512 {
                    *counts.entry(id).or_insert(0u64) += 1;
                }
                if id != old {
                    let color = if old <= 1 {
                        "#252b35"
                    } else {
                        &materials()[old].color
                    };
                    write!(
                        svg,
                        "<rect x='{}' y='{}' width='{}' height='1' fill='{color}'/>",
                        left as i64 + start,
                        top as i64 + row,
                        x - start
                    )
                    .unwrap();
                    start = x;
                    old = id;
                }
            }
        }
        yields.insert(materials()[feed].name.clone(), counts);
    }
    svg.push_str("</svg>");
    std::fs::write(format!("{output}/atlas.svg"), svg).unwrap();
    let mut fingerprints = BTreeMap::new();
    for version in [5, 6] {
        let start = std::time::Instant::now();
        let mut hash = 0xcbf29ce484222325u64;
        let mut samples = 0;
        for seed in [42, 314159] {
            for profile in 0..3 {
                for y in (0..16000).step_by(31) {
                    for x in (-1024..1024).step_by(29) {
                        let id =
                            geology::sample_versioned(seed, profile, version, x, y, materials());
                        hash = (hash ^ id as u64).wrapping_mul(0x100000001b3);
                        samples += 1;
                    }
                }
            }
        }
        fingerprints.insert(version,serde_json::json!({"fingerprint":format!("{hash:016x}"),"samples":samples,"compute_ms":start.elapsed().as_millis()}));
    }
    assert_eq!(fingerprints[&5]["fingerprint"], "35fda104e31a4a54");
    // Isolated native fixture uses real production advancement and revealed geology.
    let mut game = Game::new(42, 1);
    game.workers = 16;
    game.housing = 32;
    for (id, level) in [
        ("shaft", 3),
        ("supports", 1),
        ("drill", 3),
        ("conveyor", 3),
        ("sorter", 5),
        ("power", 5),
        ("capacity", 5),
        ("survey", 2),
    ] {
        game.levels.insert(id.into(), level);
    }
    for _ in 0..600 {
        game.second(materials(), false);
    }
    game.validate().unwrap();
    std::fs::write(
        format!("{output}/native-fixture.json"),
        serde_json::to_vec(&game).unwrap(),
    )
    .unwrap();
    let report = serde_json::json!({"generator_version":6,"legacy_preserved":true,"fingerprints":fingerprints,"atlas_counts":yields,"native_fixture":{"depth":game.depth(),"cells":game.excavated,"generator_version":game.generator_version}});
    std::fs::write(
        format!("{output}/evidence.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!(
        "{}",
        serde_json::json!({"output":output,"fingerprints":fingerprints,"native_depth":game.depth()})
    );
}
