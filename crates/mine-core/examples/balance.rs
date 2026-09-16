use mine_core::{materials, Action, Game};
fn main() {
    let mut g = Game::default();
    let args: Vec<_> = std::env::args().collect();
    let policy = args.get(1).map(String::as_str).unwrap_or("depth");
    assert!(
        ["depth", "bulk", "vein"].contains(&policy),
        "Unknown policy"
    );
    g.profile = args
        .get(2)
        .map(|v| v.parse::<usize>().expect("Site profile number"))
        .unwrap_or(0);
    assert!(g.profile < mine_core::sites().len(), "Unknown profile");
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
    g.policy = policy.into();
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
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"policy":policy,"profile":g.profile,"events":milestones,"elapsed_seconds":g.ticks/20,"retirement_ready":g.depth() >= 300 && g.steel_made,"next_upgrade":plan.get(target),"blocker":plan.get(target).and_then(|id|g.purchase_blocker(id)),"products":g.products})
        )
        .unwrap()
    );
    if policy == "depth" && !(g.depth() >= 300 && g.steel_made) {
        eprintln!(
            "Retirement target not reached in four simulated hours: depth={} steel={} credits={} target={:?}",
            g.depth(),
            g.steel_made,
            g.credits,
            plan.get(target)
        );
        std::process::exit(1)
    }
}
