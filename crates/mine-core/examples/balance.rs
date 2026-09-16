use mine_core::{materials, Action, Game};
fn main() {
    let mut g = Game::default();
    let cat = materials();
    let plan = [
        "conveyor",
        "furnace",
        "steelworks",
        "shaft",
        "supports",
        "minecart",
        "power",
        "manufacturing",
        "chemical",
    ];
    let mut target = 0;
    let mut milestones = Vec::new();
    g.policy = "depth".into();
    for second in 0..14400 {
        if target < plan.len() {
            g.pinned = Some(plan[target].into());
            if g.action(Action {
                sequence: g.last_sequence + 1,
                kind: "buy".into(),
                target: plan[target].into(),
                value: 0,
            })
            .is_ok()
            {
                milestones.push(
                    serde_json::json!({"seconds":second,"upgrade":plan[target],"depth":g.depth()}),
                );
                target += 1;
            }
        }
        g.second(&cat, false);
        if g.depth() >= 300 && g.steel_made {
            milestones.push(serde_json::json!({"seconds":second,"retirement_ready":true,"credits":g.credits,"cells":g.excavated}));
            break;
        }
    }
    println!("{}", serde_json::to_string_pretty(&milestones).unwrap());
    if !(g.depth() >= 300 && g.steel_made) {
        eprintln!(
            "Baseline stalled: depth={} steel={} credits={} target={:?}",
            g.depth(),
            g.steel_made,
            g.credits,
            plan.get(target)
        );
        std::process::exit(1)
    }
}
